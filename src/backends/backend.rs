use async_trait::async_trait;

use thiserror::Error;

use crate::runner::CommandInfo;

#[derive(Error, Debug)]
pub enum BackendError {
    #[error("Initilization error: {0}")]
    InitilizationError(String),
    #[error("Authorization error: {0}")]
    AuthorizationError(String),
    #[error("Server error: {0}")]
    ServerError(String),
    #[error("Error {0}")]
    Unknown(String),
}

pub enum BackendCommand {
    Rerun,
    Done,
    UnkownCommand(String),
}

#[async_trait]
pub trait Backend {
    async fn send_text(&mut self, info: &CommandInfo) -> Result<(), BackendError>;
    async fn recieve(&mut self) -> Result<BackendCommand, BackendError>;
}
