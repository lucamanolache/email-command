use execute::shell;
use std::fs;

use std::time::{Duration, SystemTime};

use backends::backend::Backend;
use backends::smtp_email_backend::SmtpEmailBackend;

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

    let config = fs::read_to_string("./config.toml").expect("Failed to open config");
    let config: Config = toml::from_str(&config).expect("Failed to parse config");

    let start = SystemTime::now();
    let command = "ll";

    let output = shell("ls").output().expect("failed to execute process");

    let stdout = String::from_utf8(output.stdout).expect("Failed to parse stdout");
    let stderr = String::from_utf8(output.stderr).expect("Failed to parse stderr");

    println!("{}\n{}", stdout, stderr);

    let mut backend: Box<dyn Backend> =
        Box::new(SmtpEmailBackend::new(config.email).await.unwrap());

    // let info = CommandInfo::new(command.to_owned(), start.elapsed().unwrap(), stdout, stderr);

    // match backend.send_text(&info).await {
    //     Ok(_) => println!("email sent"),
    //     Err(err) => println!("failed to send email alert: {}", err),
    // }

    backend.recieve().await.unwrap();
}
