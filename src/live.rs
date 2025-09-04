pub mod credential;

use crate::live::credential::Credential;
use client::models::BiliMessage;
use client::websocket::BiliLiveClient;
use futures_channel::mpsc;
use log::{debug, error};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use futures_util::StreamExt;
use tokio::task;
use tokio::task::JoinHandle;

pub struct LiveClient {
    rx: mpsc::Receiver<BiliMessage>,
    client: Arc<Mutex<BiliLiveClient>>,
    stop: Arc<AtomicBool>
}

impl LiveClient {
    pub fn new(room_id: &str, credential: &Credential) -> LiveClient {
        let (tx, rx) = mpsc::channel(16);

        Self {
            rx,
            client: Arc::new(Mutex::new(BiliLiveClient::new(&credential.to_string(), room_id, tx))),
            stop: Arc::new(AtomicBool::new(false))
        }
    }

    pub fn connect(&mut self) -> JoinHandle<()> {
        let mut proxy = self.client.lock().expect("failed to lock client");

        proxy.send_auth();

        let heartbeat_client = self.client.clone();
        let heartbeat_sig = self.stop.clone();

        let heartbeat_task = task::spawn_blocking(move || {
            loop {
                if heartbeat_sig.load(Ordering::SeqCst) {
                    break
                }

                match heartbeat_client.lock() {
                    Ok(mut proxy) => {
                        debug!("heartbeat");
                        proxy.send_heart_beat();
                    }
                    Err(err) => {
                        error!("error acquiring client heartbeat: {err:?}");
                        break
                    }
                }

                thread::sleep(Duration::from_secs(20));
            }
        });

        let recvmsg_client = self.client.clone();
        let recvmsg_sig = self.stop.clone();

        let recvmsg_task = task::spawn_blocking(move || {
            loop {
                if recvmsg_sig.load(Ordering::SeqCst) {
                    break;
                }

                match recvmsg_client.lock() {
                    Ok(mut proxy) => {
                        proxy.recive()
                    }
                    Err(_) => {}
                }

                thread::sleep(Duration::from_millis(50));
            }
        });

        task::spawn(async move {
            tokio::select! {
                _ = heartbeat_task => (),
                _ = recvmsg_task => (),
            }
        })
    }

    pub async fn next_message(&mut self) -> Option<BiliMessage> {
        self.rx.next().await
    }

    pub async fn close(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

