use std::time::Duration;

use async_imap::{
    error,
    types::{Fetch, Seq},
    Session,
};
use async_native_tls;
use async_native_tls::TlsStream;
use async_trait::async_trait;
use futures_util::TryStreamExt;
use lettre::{
    transport::smtp::authentication::Credentials, AsyncSmtpTransport, AsyncTransport, Message,
    Tokio1Executor,
};
use log::*;
use regex::Regex;
use tokio::net::TcpStream;
use tokio::time::sleep;

use super::backend::{Backend, BackendCommand, BackendError};
use crate::config::EmailConfig;
use crate::runner::CommandInfo;

pub struct SmtpEmailBackend {
    config: EmailConfig,
    smtp: AsyncSmtpTransport<Tokio1Executor>,
    imap: Session<TlsStream<TcpStream>>,
}

impl SmtpEmailBackend {
    pub async fn new(config: EmailConfig) -> Result<Self, BackendError> {
        // Create the smtp client
        let creds = Credentials::new(config.username.to_owned(), config.password.to_owned());

        let smtp = match AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_server) {
            Ok(smtp) => {
                debug!("Created smtp client (server {})", config.smtp_server);
                smtp
            }
            Err(err) => {
                return Err(BackendError::InitilizationError(format!(
                    "Failed to initilize smtp with:\n{}",
                    err
                )))
            }
        };
        let smtp = smtp.credentials(creds).build();

        // Create the imap client
        let tcp_stream =
            match TcpStream::connect((config.imap_server.as_str(), config.imap_port)).await {
                Ok(stream) => {
                    debug!(
                        "Set up tcp connection with {}:{}",
                        config.imap_server, config.imap_port
                    );
                    stream
                }
                Err(err) => {
                    return Err(BackendError::ServerError(format!(
                        "Failed to connect to {}:{} with:\n{}",
                        config.imap_server, config.imap_port, err
                    )))
                }
            };
        let tls = async_native_tls::TlsConnector::new();
        let tls_stream = match tls.connect(&config.imap_server, tcp_stream).await {
            Ok(stream) => {
                debug!(
                    "Set up tls connection with {}:{}",
                    config.imap_server, config.imap_port
                );
                stream
            }
            Err(err) => {
                return Err(BackendError::ServerError(format!(
                    "Failed to connect with tls to {}:{} with:\n{}",
                    config.imap_server, config.imap_port, err
                )))
            }
        };
        let client = async_imap::Client::new(tls_stream);
        let mut imap = match client.login(&config.username, &config.password).await {
            Ok(session) => {
                debug!("Created imap session");
                session
            }
            Err(_) => {
                return Err(BackendError::AuthorizationError(
                    "Failed to set up imap".to_owned(),
                ))
            }
        };

        // Remove old messages
        match imap.select("INBOX").await {
            Ok(_) => trace!("Set mailbox to INBOX"),
            Err(e) => {
                return Err(BackendError::ServerError(format!(
                    "Failed to set mailbox to INBOX with:\n{}",
                    e
                )))
            }
        };
        // Return error if fails to search for messages, as its likely it won't be able to later
        let old = match imap.search(format!("FROM {}", config.address)).await {
            Ok(old) => old,
            Err(e) => {
                return Err(BackendError::ServerError(format!(
                    "Failed to search INBOX with \"FROM {}\" with:\n{}",
                    config.address, e
                )))
            }
        };

        for msg in old {
            match delete_message(msg, &mut imap).await {
                Err(e) => error!("Failed to delete message {} with:\n{}", msg, e.to_string()),
                Ok(_) => {}
            };
        }

        Ok(SmtpEmailBackend { config, smtp, imap })
    }
}

#[async_trait]
impl Backend for SmtpEmailBackend {
    async fn recieve(&mut self) -> Result<BackendCommand, BackendError> {
        let messages = loop {
            let new = self
                .imap
                .search(format!("FROM {}", self.config.address))
                .await;

            let new = match new {
                Ok(new) => new,
                Err(_) => {
                    error!("Failed to search for message");
                    error!("Retrying in 10 seconds");
                    sleep(Duration::from_secs(10)).await;
                    continue;
                }
            };

            if !new.is_empty() {
                info!("Got message");
                break new;
            }

            debug!("No messages yet, going to sleep");
            sleep(Duration::from_secs(10)).await;
        };

        let msg_id = messages.iter().next().expect("Impossible case happened");
        let msg = &msg_id.to_string();
        let msg = match self.imap.fetch(msg, "body[]").await {
            Ok(msg) => msg,
            Err(e) => {
                return Err(BackendError::ServerError(format!(
                    "Failed to fetch email {}'s body with:\n{}",
                    msg, e
                )))
            }
        };
        let msg: Vec<Fetch> = match msg.try_collect().await {
            Ok(msg) => msg,
            Err(e) => {
                return Err(BackendError::Unknown(format!(
                    "Failed to collect messages with:\n{}",
                    e
                )))
            }
        };

        for msg in msg.iter() {
            let msg = match msg.body() {
                Some(msg) => msg,
                None => {
                    return Err(BackendError::Unknown(
                        "Failed to get email body with".to_string(),
                    ))
                }
            };
            let msg = match mail_parser::Message::parse(msg) {
                Some(msg) => msg,
                None => return Err(BackendError::Unknown("Failed to parse email".to_string())),
            };
            let body = match msg.body_text(0) {
                Some(body) => body,
                None => {
                    return Err(BackendError::Unknown(
                        "Failed to parse email body".to_string(),
                    ))
                }
            };

            trace!("Body:\n{}", body);
            let regex =
                Regex::new(r"^On.+ at .+wrote:").expect("Impossible error, failed to parse regex");
            let body: Vec<&str> = body
                .split('\n')
                .filter(|line| !line.is_empty())
                .filter(|line| !line.starts_with('>'))
                .filter(|line| !regex.is_match(line))
                .collect();
            if body.len() != 1 {
                panic!("Bad length")
            }
            let command = body[0];
            delete_message(*msg_id, &mut self.imap).await.unwrap();
            return match command.to_ascii_lowercase().as_str() {
                "rerun" => Ok(BackendCommand::Rerun),
                "done" => Ok(BackendCommand::Done),
                _ => Ok(BackendCommand::UnkownCommand(command.to_owned())),
            };
        }
        Ok(BackendCommand::UnkownCommand(
            "Could not find command".to_owned(),
        ))
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
            .body(format!(
                "STDOUT:\n{}\n\nSTDERR:\n{}",
                info.stdout, info.stderr
            ));
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

async fn delete_message(
    seq: Seq,
    session: &mut Session<TlsStream<TcpStream>>,
) -> error::Result<()> {
    let updates_stream = session
        .store(format!("{}", seq), "+FLAGS (\\Deleted)")
        .await?;
    let _updates: Vec<_> = updates_stream.try_collect().await?;
    session.expunge().await?;
    info!("Deleted message {}", seq);
    Ok(())
}
