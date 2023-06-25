use backends::matrix_backend::MatrixBackend;
use clap::Parser;
use std::fs;
use std::time::Duration;
use tokio::time::sleep;

use backends::backend::{Backend, BackendCommand, BackendList, Sendable};
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

    #[arg(short = 'f', long = "file")]
    files: Option<Vec<String>>,

    #[arg()]
    command: String,
}

async fn get_backend(backend: &BackendList, config: Config) -> Box<dyn Backend> {
    match backend {
        BackendList::Matrix => Box::new(
            MatrixBackend::new(config.matrix.expect("Missing matrix section in config!"))
                .await
                .expect("Failed to create matrix backend!"),
        ),
        BackendList::Email => Box::new(
            SmtpEmailBackend::new(config.email.expect("Missing email section in config!"))
                .await
                .expect("Failed to create email backend!"),
        ),
        _ => unimplemented!(),
    }
}

async fn send(backend: &mut dyn Backend, msg: &Sendable) {
    match backend.send_text(msg).await {
        Ok(_) => println!("Sent"),
        Err(err) => {
            eprintln!("Failed to send email with:\n{}", err);
            println!("Trying again in 30s");
            sleep(Duration::from_secs(30)).await;
            match backend.send_text(msg).await {
                Ok(_) => println!("Sent"),
                Err(err) => panic!("Can't send results with:\n{}", err),
            }
        }
    }
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let args = Args::parse();

    let config = fs::read_to_string("./config.toml").expect("Failed to open config");
    let config: Config = toml::from_str(&config).expect("Failed to parse config");

    let mut backend = get_backend(&args.backend, config).await;

    let mut command = BackendCommand::Rerun;

    let cat = fs::read("./cat.jpeg").expect("Can't open image file.");

    loop {
        if command == BackendCommand::Rerun {
            let info = run(&args.command).unwrap();
            send(&mut *backend, &Sendable::CommandInfo(info)).await;
            if args.files.is_some() {
                for name in args.files.as_ref().expect("Something impossible happened") {
                    let file = fs::read(name).expect(&format!("File {name} doesn't exist"));

                    let parts: Vec<&str> = name.split('.').collect();
                    let res = match parts.last() {
                        Some(v) => match *v {
                            "png" => mime::IMAGE_PNG,
                            "jpg" => mime::IMAGE_JPEG,
                            "jpeg" => mime::IMAGE_JPEG,
                            "json" => mime::APPLICATION_JSON,
                            "csv" => mime::TEXT_CSV,
                            &_ => mime::TEXT_PLAIN,
                        },
                        None => mime::TEXT_PLAIN,
                    };

                    send(
                        &mut *backend,
                        &Sendable::Image((res, name.to_string(), file)),
                    )
                    .await;
                }
            }
        }

        command = backend.recieve().await.unwrap();
        match &command {
            BackendCommand::Rerun => continue,
            BackendCommand::Done => {
                send(&mut *backend, &Sendable::Raw("Done!".to_string())).await;
                return;
            }
            BackendCommand::UnkownCommand(s) => {
                send(
                    &mut *backend,
                    &Sendable::Raw(format!("Unkown command: {}", s)),
                )
                .await
            }
            BackendCommand::Cat => {
                send(
                    &mut *backend,
                    &Sendable::Image((mime::IMAGE_JPEG, "cat".to_string(), cat.clone())),
                )
                .await
            }
        }
    }
}
