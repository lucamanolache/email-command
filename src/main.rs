use std::fs;
use std::process::Command;
use std::time::SystemTime;

use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, Message};
use lettre::{AsyncTransport, Tokio1Executor};
use tokio;

use log::trace;

use crate::config::Config;

mod backends;
mod config;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let config: Config = toml::from_str(&fs::read_to_string("./config.toml").unwrap()).unwrap();

    let start = SystemTime::now();
    let command = "ls";
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .expect("failed to execute process");

    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();

    let email = Message::builder()
        .from(config.email.smtp_username.parse().unwrap())
        .to(config.email.to_address.parse().unwrap())
        .subject(format!(
            "Command {} finished in {}",
            command,
            start.elapsed().unwrap().as_secs_f64()
        ))
        .body(format!("STDOUT:\n{}\n\nSTDERR:\n{}", stdout, stderr).to_owned())
        .unwrap();

    let creds = Credentials::new(
        config.email.smtp_username.to_owned(),
        config.email.smtp_password.to_owned(),
    );

    let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&config.email.smtp_server)
        .unwrap()
        .credentials(creds)
        .build();

    trace!("a trace example");
    let result = mailer.send(email).await;
    match result {
        Ok(_) => println!("email sent"),
        Err(err) => println!("failed to send email alert: {}", err),
    }
}
