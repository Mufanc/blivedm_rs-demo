mod live;
mod data;

use crate::data::event_logger;
use crate::live::credential::Credential;
use crate::live::LiveClient;
use anyhow::Result;
use std::env;
use tokio::runtime::Runtime;

fn main() -> Result<()> {
    event_logger::init();

    let room_id = &env::var("ROOM_ID").expect("ROOM_ID must be set");
    let sessdata = &env::var("SESSDATA").expect("SESSDATA must be set");

    let rt = Runtime::new().expect("failed to initialize tokio runtime");
    let mut client = LiveClient::new(room_id, &Credential::from_sessdata(sessdata));

    rt.block_on(async {
        let handle = client.connect();

        while let Some(message) = client.next_message().await {
            tracing::info!(target: "raw", data = %message);
        }

        client.close().await;

        let _ = handle.await;
    });

    Ok(())
}
