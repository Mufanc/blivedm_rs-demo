mod data;
mod live;

use crate::data::logger;
use crate::data::logger::MessageLogger;
use crate::live::LiveClient;
use crate::live::credential::Credential;
use crate::live::message::LiveMessage;
use anyhow::Result;
use log::{debug, error};
use std::env;
use tokio::runtime::Runtime;

fn main() -> Result<()> {
    logger::init();

    let room_id = &env::var("ROOM_ID").expect("ROOM_ID must be set");
    let sessdata = &env::var("SESSDATA").expect("SESSDATA must be set");

    let rt = Runtime::new().expect("failed to initialize tokio runtime");
    let mut client = LiveClient::new(room_id, &Credential::from_sessdata(sessdata));

    rt.block_on(async {
        let handle = client.connect();
        let mut message_logger = MessageLogger::new(room_id);

        while let Some(message) = client.next_message().await {
            let data = message.data();

            if let Err(err) = message_logger.write(&data) {
                error!("failed to write message: {}", err);
            }

            let message = LiveMessage::try_from(message);

            match message {
                Ok(LiveMessage::Unsupported(msg_type)) => {
                    debug!("unsupported message type: {}", msg_type);
                }
                Ok(message) => {
                    println!("{message:?}");
                }
                Err(msg) => {
                    error!("failed to parse message: {:?}", msg);
                }
            }
        }

        client.close().await;

        let _ = handle.await;
    });

    Ok(())
}
