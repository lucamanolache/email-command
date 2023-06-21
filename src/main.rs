use std::fs;
use std::process::Command;
use std::time::{Duration, SystemTime};

use backends::backend::Backend;
use backends::smtp_email_backend::SmtpEmailBackend;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, Message};
use lettre::{AsyncTransport, Tokio1Executor};

use tokio;

use log::trace;

use crate::config::Config;

mod backends;
mod config;

pub struct CommandInfo {
    pub time: Duration,
    pub command: String,
    pub stdout: String,
    pub stderr: String,
}

impl CommandInfo {
    fn new(command: String, time: Duration, stdout: String, stderr: String) -> Self {
        Self {
            time,
            command,
            stdout,
            stderr,
        }
    }
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let config: Config = toml::from_str(&fs::read_to_string("./config.toml").unwrap()).unwrap();

    let start = SystemTime::now();
    let command = "ll";
    let output = Command::new(command)
        .arg("-c")
        .arg(command)
        .output()
        .expect("failed to execute process");

    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();

    let backend: Box<dyn Backend> = Box::new(SmtpEmailBackend::new(config.email));

    let info = CommandInfo::new(command.to_owned(), start.elapsed().unwrap(), stdout, stderr);

    match backend.send_text(&info).await {
        Ok(_) => println!("email sent"),
        Err(err) => println!("failed to send email alert: {}", err),
    }
}
