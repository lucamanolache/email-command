use async_trait::async_trait;

use thiserror::Error;

use crate::CommandInfo;

#[derive(Error, Debug)]
pub enum BackendError {
    #[error("Initilization error: {0}")]
    InitilizationError(String),
    #[error("Authorization error")]
    AuthorizationError,
    #[error("Server error: {0}")]
    ServerError(String),
    #[error("Error {0}")]
    Unknown(String),
}

pub enum BackendCommand {
    Done,
}

#[async_trait]
pub trait Backend {
    async fn send_text(&mut self, info: &CommandInfo) -> Result<(), BackendError>;
    async fn recieve(&mut self) -> Result<BackendCommand, BackendError>;
}
