use async_trait::async_trait;
use lazy_static::lazy_static;
use log::*;
use matrix_sdk::{
    attachment::AttachmentConfig,
    config::SyncSettings,
    room::{Joined, Room},
    ruma::{
        events::{
            room::message::{
                ImageMessageEventContent, MessageType, OriginalSyncRoomMessageEvent,
                RoomMessageEventContent,
            },
            MessageLikeEvent, MessageLikeEventContent,
        },
        OwnedUserId, RoomId, UserId,
    },
    Client, Error,
};
use tokio::{
    sync::{Mutex, Notify},
    task::JoinHandle,
};

use super::backend::{Backend, BackendCommand, BackendError, Sendable};
use crate::config::MatrixConfig;

lazy_static! {
    static ref MESSAGES: Mutex<Vec<String>> = Mutex::new(Vec::new());
    static ref NOTIFY: Notify = Notify::new();
}

static mut ADDRESS: Option<OwnedUserId> = None;

pub struct MatrixBackend {
    config: MatrixConfig,
    handle: JoinHandle<Result<(), Error>>,
    room: Joined,
}

impl MatrixBackend {
    pub async fn new(config: MatrixConfig) -> Result<Self, BackendError> {
        let user = UserId::parse(&config.username).unwrap();
        let address = UserId::parse(&config.address).unwrap();
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

        client.sync_once(SyncSettings::default()).await.unwrap();

        unsafe {
            ADDRESS = Some(address);
        }

        let room = client
            .get_joined_room(<&RoomId>::try_from(config.room.as_str()).unwrap())
            .unwrap();

        let handle = tokio::spawn(async move {
            client.add_event_handler(|ev: OriginalSyncRoomMessageEvent, room: Room| async move {
                let Room::Joined(room) = room else { panic!("Didn't join room") };
                let MessageType::Text(text_content) = ev.content.msgtype else { return };

                info!("Got message");
                let mut msgs = MESSAGES.lock().await;
                trace!("Obtained MESSAGES lock");
                unsafe {
                    if ev.sender != ADDRESS.clone().unwrap() {
                        warn!("Got message from wrong sender: {}", ev.sender);
                        return;
                    }
                }
                msgs.push(text_content.body);
                NOTIFY.notify_one();
                trace!("Sent notification");
                return;
            });
            client.sync(SyncSettings::default()).await
        });

        Ok(MatrixBackend {
            config,
            handle,
            room,
        })
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
            "cat" => Ok(BackendCommand::Cat),
            _ => Ok(BackendCommand::UnkownCommand(message)),
        }
    }

    async fn send_text(&mut self, msg: &Sendable) -> Result<(), BackendError> {
        match msg {
            Sendable::CommandInfo(info) => {
                let content =
                    RoomMessageEventContent::text_markdown(
                        format!(
                            "Ran command *{}* in *{}*s \n\n **STANDARD OUT:**\n\n{}\n\n**STANDARD ERROR:**\n\n{}",
                            info.command,
                            info.time.as_secs(),
                            info.stdout,
                            info.stderr));
                self.room.send(content, None).await
            }
            Sendable::Raw(s) => {
                let content = RoomMessageEventContent::text_markdown(s.to_string());
                self.room.send(content, None).await
            }
            Sendable::Image((mime, data)) => {
                self.room.send_attachment("cat", mime, data, AttachmentConfig::new()).await
            }
            _ => {
                unimplemented!()
            }
        }.unwrap();
        Ok(())
    }
}
