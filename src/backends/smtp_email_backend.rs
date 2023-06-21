use async_trait::async_trait;
use lettre::{
    transport::smtp::authentication::Credentials, AsyncSmtpTransport, AsyncTransport, Message,
    Tokio1Executor,
};

use crate::{config::EmailConfig, CommandInfo};

use super::backend::{Backend, BackendCommand, BackendError};

pub struct SmtpEmailBackend {
    config: EmailConfig,
    mailer: AsyncSmtpTransport<Tokio1Executor>,
}

impl SmtpEmailBackend {
    pub fn new(config: EmailConfig) -> Self {
        let creds = Credentials::new(
            config.smtp_username.to_owned(),
            config.smtp_password.to_owned(),
        );

        let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_server)
            .unwrap()
            .credentials(creds)
            .build();

        SmtpEmailBackend { config, mailer }
    }
}

#[async_trait]
impl Backend for SmtpEmailBackend {
    async fn recieve(&self) -> Result<BackendCommand, BackendError> {
        Ok(BackendCommand::Done)
    }

    async fn send_text(&self, info: &CommandInfo) -> Result<(), BackendError> {
        let email = Message::builder()
            .from(self.config.smtp_username.parse().unwrap())
            .to(self.config.to_address.parse().unwrap())
            .subject(format!(
                "Command \"{}\" finished in {}",
                info.command,
                info.time.as_secs_f64()
            ))
            .body(format!("STDOUT:\n{}\n\nSTDERR:\n{}", info.stdout, info.stderr).to_owned())
            .unwrap();

        match self.mailer.send(email).await {
            Ok(_) => Ok(()),
            Err(_) => Err(BackendError::SendError),
        }
    }
}
