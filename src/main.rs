use clap::Parser;
use std::fs;

use backends::backend::{Backend, BackendCommand, BackendList};
use backends::smtp_email_backend::SmtpEmailBackend;

use crate::config::Config;
use crate::runner::run;

mod backends;
mod config;
mod runner;

#[derive(Parser, Debug)]
#[command(author = "Luca Manolache", version = "0.1.0", about = "Run command controllable by email/text", long_about = None)]
struct Args {
    /// Location of the config file (default ./config.toml)
    #[arg(short, long, default_value = "./config.toml")]
    config: String,

    /// Backend to use (requires relevent section of config to be set)
    #[arg(short = 'b', long = "backend")]
    backend: BackendList,

    #[arg()]
    command: String,
}

async fn get_backend(backend: &BackendList, config: Config) -> Box<dyn Backend> {
    match backend {
        BackendList::Email => Box::new(
            SmtpEmailBackend::new(config.email.expect("Missing email section in config!"))
                .await
                .expect("Failed to create email backend!"),
        ),
        _ => panic!("Unknown backend!"),
    }
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let args = Args::parse();

    let config = fs::read_to_string("./config.toml").expect("Failed to open config");
    let config: Config = toml::from_str(&config).expect("Failed to parse config");

    let mut backend = get_backend(&args.backend, config).await;

    loop {
        let info = run(&args.command).unwrap();

        match backend.send_text(&info).await {
            Ok(_) => println!("email sent"),
            Err(err) => println!("failed to send email alert: {}", err),
        }

        let command = backend.recieve().await.unwrap();
        match command {
            BackendCommand::Rerun => continue,
            BackendCommand::Done => return,
            BackendCommand::UnkownCommand(_) => return,
        }
    }
}
