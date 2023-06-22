use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub email: EmailConfig,
}

#[derive(Deserialize)]
pub struct EmailConfig {
    pub address: String,
    pub username: String,
    pub password: String,
    pub smtp_server: String,
    pub pop_server: String,
    pub pop_port: u16,
}
