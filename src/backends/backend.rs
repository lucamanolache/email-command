use async_trait::async_trait;

use thiserror::Error;

use crate::CommandInfo;

#[derive(Error, Debug)]
pub enum BackendError {
    #[error("Failed to send notification")]
    SendError,
    #[error("Failed to recieve reply")]
    RecieveError,
    #[error("Unknown error")]
    Unknown,
}

pub enum BackendCommand {
    Done,
}

#[async_trait]
pub trait Backend {
    async fn send_text(&mut self, info: &CommandInfo) -> Result<(), BackendError>;
    async fn recieve(&mut self) -> Result<BackendCommand, BackendError>;
}
