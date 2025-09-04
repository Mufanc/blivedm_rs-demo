mod live;

use std::env;
use crate::live::credential::Credential;
use crate::live::LiveClient;
use anyhow::Result;
use log::info;
use tokio::runtime::Runtime;

fn main() -> Result<()> {
    env_logger::init();

    let room_id = &env::var("ROOM_ID").expect("ROOM_ID must be set");
    let sessdata = &env::var("SESSDATA").expect("SESSDATA must be set");

    let rt = Runtime::new().expect("failed to initialize tokio runtime");
    let mut client = LiveClient::new(room_id, &Credential::from_sessdata(sessdata));

    rt.block_on(async {
        let handle = client.connect();

        while let Some(message) = client.next_message().await {
            info!("{:?}", message);
        }

        client.close().await;

        let _ = handle.await;
    });

    Ok(())
}
