use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub smpp: SmppConfig,
    pub log: LogConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SmppConfig {
    pub system_id: String,
    pub password: String,
    pub port: u16,
    pub max_sessions: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LogConfig {
    pub level: String,
}

impl AppConfig {
    pub fn new() -> Result<Self, ConfigError> {
        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());

        let s = Config::builder()
            // Start with default values
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 8080)?
            .set_default("smpp.port", 2775)?
            .set_default("smpp.system_id", "smppclient1")?
            .set_default("smpp.password", "password")?
            .set_default("smpp.max_sessions", 50)?
            .set_default("log.level", "info")?
            
            // Add configuration file
            .add_source(File::with_name("config").required(false))
            .add_source(File::with_name(&format!("config.{}", run_mode)).required(false))
            
            // Add environment variables (e.g., SMPP_SERVER_PORT=8080)
            .add_source(Environment::with_prefix("SMPP").separator("__"))
            .build()?;

        s.try_deserialize()
    }
}
