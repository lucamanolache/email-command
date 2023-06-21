use async_trait::async_trait;

use thiserror::Error;

use crate::CommandInfo;

#[derive(Error, Debug)]
pub enum BackendError {
    #[error("Failed to send notification")]
    SendError,
    #[error("Command read error")]
    Redaction(String),
    #[error("unknown data store error")]
    Unknown,
}

pub enum BackendCommand {
    Done,
}

#[async_trait]
pub trait Backend {
    async fn send_text(&self, info: &CommandInfo) -> Result<(), BackendError>;
    async fn recieve(&self) -> Result<BackendCommand, BackendError>;
}
