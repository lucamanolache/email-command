use std::time::Duration;

use async_trait::async_trait;
use lettre::{
    transport::smtp::authentication::Credentials, AsyncSmtpTransport, AsyncTransport, Message,
    Tokio1Executor,
};
use log::*;
use rust_pop3_client::Pop3Connection;
use tokio::time::sleep;

use crate::{config::EmailConfig, CommandInfo};

use super::backend::{Backend, BackendCommand, BackendError};

pub struct SmtpEmailBackend {
    config: EmailConfig,
    smtp: AsyncSmtpTransport<Tokio1Executor>,
    pop: Pop3Connection,
}

impl SmtpEmailBackend {
    pub fn new(config: EmailConfig) -> Result<Self, BackendError> {
        let creds = Credentials::new(config.username.to_owned(), config.password.to_owned());

        trace!("Creating smtp client (server {})", config.smtp_server);
        let mut smtp = match AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_server) {
            Ok(smtp) => smtp,
            Err(err) => {
                return Err(BackendError::InitilizationError(format!(
                    "Failed to initilize smtp with:\n{}",
                    err.to_string()
                )))
            }
        };
        let smtp = smtp.credentials(creds).build();

        trace!(
            "Creating pop3 client (server: {}, port: {})",
            config.pop_server,
            config.pop_port
        );
        let mut pop = match Pop3Connection::new(&config.pop_server, config.pop_port) {
            Ok(pop) => pop,
            Err(err) => return Err(BackendError::InitilizationError(err.to_string())),
        };
        match pop.login(&config.username, &config.password) {
            Ok(_) => trace!("pop3 login sucessful"),
            Err(_) => return Err(BackendError::AuthorizationError),
        };

        let infos = match pop.list() {
            Ok(infos) => infos,
            Err(err) => {
                return Err(BackendError::ServerError(format!(
                    "Failed getting pop3 list of emails with:\n{}",
                    err.to_string()
                )))
            }
        };
        for info in infos {
            info!("Cleaning message {}", info.message_id);
            match pop.delete(info.message_id) {
                Ok(_) => info!("Cleaned message {}", info.message_id),
                Err(err) => {
                    return Err(BackendError::ServerError(format!(
                        "Failed to delete message {} with:\n{}",
                        info.message_id,
                        err.to_string()
                    )))
                }
            };
        }

        Ok(SmtpEmailBackend { config, smtp, pop })
    }
}

#[async_trait]
impl Backend for SmtpEmailBackend {
    async fn recieve(&mut self) -> Result<BackendCommand, BackendError> {
        let infos = loop {
            let infos = match self.pop.list() {
                Ok(infos) => infos,
                Err(err) => {
                    return Err(BackendError::ServerError(format!(
                        "Failed getting pop3 list of emails with:\n{}",
                        err.to_string()
                    )))
                }
            };
            if infos.len() > 0 {
                debug!("{} messages found, proceeding to download", infos.len());
                break infos;
            }
            trace!("No messages yet, going to sleep");
            sleep(Duration::from_secs(10)).await;
        };

        for info in infos {
            let mut buf = Vec::new();
            match self.pop.retrieve(info.message_id, &mut buf) {
                Ok(_) => info!("Retrieved email {}", info.message_id),
                Err(err) => {
                    return Err(BackendError::ServerError(format!(
                        "Failed to retrieve email {} with:\n{}",
                        info.message_id,
                        err.to_string()
                    )))
                }
            };
            println!("{:?}", buf);
        }

        Ok(BackendCommand::Done)
    }

    async fn send_text(&mut self, info: &CommandInfo) -> Result<(), BackendError> {
        let email = Message::builder()
            .from(match self.config.username.parse() {
                Ok(user) => user,
                Err(_) => {
                    return Err(BackendError::Unknown(format!(
                        "Failed to parse username {}",
                        self.config.username,
                    )))
                }
            })
            .to(match self.config.address.parse() {
                Ok(address) => address,
                Err(_) => {
                    return Err(BackendError::Unknown(format!(
                        "Failed to parse address {}",
                        self.config.address
                    )))
                }
            })
            .subject(format!(
                "Command \"{}\" finished in {}",
                info.command,
                info.time.as_secs_f64()
            ))
            .body(format!("STDOUT:\n{}\n\nSTDERR:\n{}", info.stdout, info.stderr).to_owned());
        let email = match email {
            Ok(email) => email,
            Err(_) => return Err(BackendError::Unknown("Failed to generate email".into())),
        };

        match self.smtp.send(email).await {
            Ok(_) => Ok(()),
            Err(_) => Err(BackendError::ServerError("Failed to send email".into())),
        }
    }
}
