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
    pub fn new(config: EmailConfig) -> Self {
        let creds = Credentials::new(config.username.to_owned(), config.password.to_owned());

        trace!("Creating smtp client (server {})", config.smtp_server);
        let smtp = AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_server)
            .unwrap()
            .credentials(creds)
            .build();

        trace!(
            "Creating pop3 client (server: {}, port: {})",
            config.pop_server,
            config.pop_port
        );
        let mut pop = Pop3Connection::new(&config.pop_server, config.pop_port).unwrap();
        pop.login(&config.username, &config.password).unwrap();

        let infos = pop.list().unwrap();
        for info in infos {
            info!("Cleaning message {}", info.message_id);
            pop.delete(info.message_id).unwrap();
        }

        SmtpEmailBackend { config, smtp, pop }
    }
}

#[async_trait]
impl Backend for SmtpEmailBackend {
    async fn recieve(&mut self) -> Result<BackendCommand, BackendError> {
        let infos = loop {
            let infos = match self.pop.list() {
                Ok(i) => i,
                Err(_) => return Err(BackendError::RecieveError),
            };
            if infos.len() > 0 {
                debug!("{} messages found, proceeding to download", infos.len());
                break infos;
            }
            trace!("No messages yet, going to sleep");
            sleep(Duration::from_secs(5)).await;
        };

        for info in infos {
            info!("Retrieving email with id {}", info.message_id);
            let mut buf = Vec::new();
            self.pop.retrieve(info.message_id, &mut buf).unwrap();
            println!("{:?}", buf);
        }

        Ok(BackendCommand::Done)
    }

    async fn send_text(&mut self, info: &CommandInfo) -> Result<(), BackendError> {
        let email = Message::builder()
            .from(self.config.username.parse().unwrap())
            .to(self.config.address.parse().unwrap())
            .subject(format!(
                "Command \"{}\" finished in {}",
                info.command,
                info.time.as_secs_f64()
            ))
            .body(format!("STDOUT:\n{}\n\nSTDERR:\n{}", info.stdout, info.stderr).to_owned())
            .unwrap();

        match self.smtp.send(email).await {
            Ok(_) => Ok(()),
            Err(_) => Err(BackendError::SendError),
        }
    }
}
