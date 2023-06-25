use async_trait::async_trait;
use lazy_static::lazy_static;
use log::*;
use matrix_sdk::{
    config::SyncSettings,
    ruma::{
        events::room::message::{RoomMessageEventContent, SyncRoomMessageEvent},
        events::MessageLikeEventType,
        OwnedUserId, UserId,
    },
    Client, Error,
};
use tokio::{
    sync::{Mutex, Notify},
    task::JoinHandle,
};

use super::backend::{Backend, BackendCommand, BackendError};
use crate::config::MatrixConfig;
use crate::runner::CommandInfo;

lazy_static! {
    static ref MESSAGES: Mutex<Vec<String>> = Mutex::new(Vec::new());
    static ref NOTIFY: Notify = Notify::new();
}

static mut ADDRESS: Option<OwnedUserId> = None;

pub struct MatrixBackend {
    config: MatrixConfig,
    handle: JoinHandle<Result<(), Error>>,
}

impl MatrixBackend {
    pub async fn new(config: MatrixConfig) -> Result<Self, BackendError> {
        let user = UserId::parse(&config.username).unwrap();
        let address = UserId::parse(&config.address).unwrap();
        println!("{}", user.server_name());
        let client = Client::builder()
            .server_name(user.server_name())
            .build()
            .await
            .unwrap();

        // First we need to log in.
        client
            .login_username(&user, &config.password)
            .send()
            .await
            .unwrap();

        unsafe {
            ADDRESS = Some(address);
        }

        let handle = tokio::spawn(async move {
            client.add_event_handler(|ev: SyncRoomMessageEvent| async move {
                info!("Got message");
                let mut msgs = MESSAGES.lock().await;
                trace!("Obtained MESSAGES lock");
                unsafe {
                    if ev.sender() != ADDRESS.clone().unwrap() {
                        warn!("Got message from wrong sender: {}", ev.sender());
                        return;
                    }
                }
                match ev.event_type() {
                    MessageLikeEventType::RoomMessage => {
                        let ev: RoomMessageEventContent =
                            ev.as_original().unwrap().content.clone().into();
                        msgs.push(ev.body().to_owned());
                        NOTIFY.notify_one();
                        trace!("Sent notification");
                        return;
                    }
                    _ => {
                        debug!("Message of wrong type: {}", ev.event_type())
                    }
                }
            });
            client.sync(SyncSettings::default()).await
        });

        Ok(MatrixBackend { config, handle })
    }
}

#[async_trait]
impl Backend for MatrixBackend {
    async fn recieve(&mut self) -> Result<BackendCommand, BackendError> {
        trace!("Preparing to recieve");
        NOTIFY.notified().await;
        trace!("Notification recieved");
        let mut msgs = MESSAGES.lock().await;
        trace!("Lock recieved");
        let message = msgs.pop().unwrap();

        match message.as_str() {
            "rerun" => Ok(BackendCommand::Rerun),
            "done" => Ok(BackendCommand::Done),
            _ => Ok(BackendCommand::UnkownCommand(message)),
        }
    }

    async fn send_text(&mut self, info: &CommandInfo) -> Result<(), BackendError> {
        println!("Congrats!");
        Ok(())
    }
}
