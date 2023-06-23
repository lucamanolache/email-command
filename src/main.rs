use std::fs;

use backends::backend::Backend;
use backends::smtp_email_backend::SmtpEmailBackend;

use crate::config::Config;

mod backends;
mod config;
mod runner;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let config = fs::read_to_string("./config.toml").expect("Failed to open config");
    let config: Config = toml::from_str(&config).expect("Failed to parse config");

    let mut backend: Box<dyn Backend> =
        Box::new(SmtpEmailBackend::new(config.email).await.unwrap());

    // match backend.send_text(&info).await {
    //     Ok(_) => println!("email sent"),
    //     Err(err) => println!("failed to send email alert: {}", err),
    // }

    backend.recieve().await.unwrap();
}
