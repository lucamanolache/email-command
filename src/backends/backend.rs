use std::{fmt::Display, str::FromStr};

use async_trait::async_trait;
use mime::Mime;
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

#[derive(Error, Debug)]
pub struct ParseError;

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("backend name doesn't exist")
    }
}

pub enum Sendable {
    Raw(String),
    CommandInfo(CommandInfo),
    Image((Mime, String, Vec<u8>)),
    File((Mime, String, Vec<u8>)),
}

#[derive(PartialEq)]
pub enum BackendCommand {
    Rerun,
    Done,
    UnkownCommand(String),
    Cat,
}

#[async_trait]
pub trait Backend {
    async fn send_text(&mut self, info: &Sendable) -> Result<(), BackendError>;
    async fn recieve(&mut self) -> Result<BackendCommand, BackendError>;
}

#[derive(Clone, Debug)]
pub enum BackendList {
    Email,
    Matrix,
}

impl FromStr for BackendList {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "email" => Ok(Self::Email),
            "matrix" => Ok(Self::Matrix),
            _ => Err(ParseError),
        }
    }
}
