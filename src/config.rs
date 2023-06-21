use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub email: EmailConfig,
}

#[derive(Deserialize)]
pub struct EmailConfig {
    pub to_address: String,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_server: String,
}
