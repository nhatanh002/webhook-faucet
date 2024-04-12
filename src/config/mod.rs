use anyhow::{anyhow, Context, Result};
use once_cell::sync::Lazy;
use serde::Deserialize;
use serde_inline_default::serde_inline_default;

#[serde_inline_default]
#[derive(Deserialize, Debug)]
pub struct Config {
    #[serde_inline_default("127.0.0.1".to_string())]
    pub app_host: String,
    #[serde_inline_default(8080)]
    pub app_port: u32,
    #[serde_inline_default("error".to_string())]
    pub rust_log: String,
    pub redis_url: String,
    pub shopify_client_secret: String,
    #[serde_inline_default("http://localhost:8888".to_string())]
    pub downstream_app_url: String,
    // #[serde_inline_default("localhost".to_string())]
    // pub db_host: String,
    // #[serde_inline_default(3306)]
    // pub db_port: u32,
    // #[serde_inline_default("default".to_string())]
    // pub db_database: String,
    // pub db_username: String,
    // pub db_password: String,
}
// static mut CONFIG: Option<Config> = None;
static CONFIG: Lazy<Config> = Lazy::new(|| load_config().unwrap());
// pub fn load_config() -> std::result::Result<Config, Box<dyn std::error::Error>> {
fn load_config() -> Result<Config> {
    dotenvy::dotenv()?;
    envy::from_env::<Config>()
        .map_err(|e| anyhow!(e))
        .context(format!(
            "at {} line {} column {}",
            file!(),
            line!(),
            column!(),
        ))
    // envy::from_env::<Config>().map_err(Into::into)
    // envy::from_env::<Config>().map_err(|e| Box::new(e) as Box<dyn Error>)
    // envy::from_env::<Config>().map_err(|e| Box::<dyn std::error::Error>::from(e))
}

pub fn get() -> &'static Lazy<Config> {
    // let cnf = unsafe { CONFIG.get_or_insert(load_config().unwrap()) };
    // cnf
    &CONFIG
}
