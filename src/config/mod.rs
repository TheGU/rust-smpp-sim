use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub smpp: SmppConfig,
    pub log: LogConfig,
    #[serde(default)]
    pub lifecycle: LifecycleConfig,
    #[serde(default)]
    pub mo_service: MoServiceConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SmppConfig {
    pub system_id: String, // Fallback/Default system_id
    pub password: String,  // Fallback/Default password
    pub port: u16,
    pub max_sessions: usize,
    #[serde(default)]
    pub accounts: Vec<SmppAccount>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SmppAccount {
    pub system_id: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LifecycleConfig {
    pub message_state_check_frequency_ms: u64,
    pub max_time_enroute_ms: u64,
    pub discard_from_queue_after_ms: u64,
    pub percent_delivered: u8,
    pub percent_undeliverable: u8,
    pub percent_accepted: u8,
    pub percent_rejected: u8,
    pub delivery_receipt_tlv: Option<String>,
}

impl Default for LifecycleConfig {
    fn default() -> Self {
        Self {
            message_state_check_frequency_ms: 5000,
            max_time_enroute_ms: 10000,
            discard_from_queue_after_ms: 60000,
            percent_delivered: 90,
            percent_undeliverable: 6,
            percent_accepted: 2,
            percent_rejected: 2,
            delivery_receipt_tlv: None,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct MoServiceConfig {
    pub enabled: bool,
    pub delivery_messages_per_minute: u32,
    pub file_path: String,
}

impl Default for MoServiceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            delivery_messages_per_minute: 0,
            file_path: "deliver_messages.csv".to_string(),
        }
    }
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
            
            // Lifecycle defaults
            .set_default("lifecycle.message_state_check_frequency_ms", 5000)?
            .set_default("lifecycle.max_time_enroute_ms", 10000)?
            .set_default("lifecycle.discard_from_queue_after_ms", 60000)?
            .set_default("lifecycle.percent_delivered", 90)?
            .set_default("lifecycle.percent_undeliverable", 6)?
            .set_default("lifecycle.percent_accepted", 2)?
            .set_default("lifecycle.percent_rejected", 2)?

             // MO Service defaults
            .set_default("mo_service.enabled", false)?
            .set_default("mo_service.delivery_messages_per_minute", 0)?
            .set_default("mo_service.file_path", "deliver_messages.csv")?

            // Add configuration file
            .add_source(File::with_name("config").required(false))
            .add_source(File::with_name(&format!("config.{}", run_mode)).required(false))
            
            // Add environment variables (e.g., SMPP_SERVER_PORT=8080)
            // Add environment variables (prefix with SMPP__)
            .add_source(Environment::with_prefix("SMPP").separator("__"))
            
            // Allow explicit overrides for documented env vars
            .set_override_option("server.host", env::var("SERVER_HOST").ok())?
            .set_override_option("server.port", env::var("SERVER_PORT").ok().map(|v| v.parse::<u16>().unwrap_or(8080)))?
            .set_override_option("smpp.port", env::var("SMPP_PORT").ok().map(|v| v.parse::<u16>().unwrap_or(2775)))?
            .set_override_option("smpp.system_id", env::var("SMPP_SYSTEM_ID").ok())?
            .set_override_option("smpp.password", env::var("SMPP_PASSWORD").ok())?
            .set_override_option("log.level", env::var("LOG_LEVEL").ok())?
            .set_override_option("lifecycle.max_time_enroute_ms", env::var("LIFECYCLE_MAX_TIME_ENROUTE_MS").ok().map(|v| v.parse::<u64>().unwrap_or(10000)))?
            .set_override_option("lifecycle.percent_delivered", env::var("LIFECYCLE_PERCENT_DELIVERED").ok().map(|v| v.parse::<u8>().unwrap_or(90)))?
            
            .build()?;

        s.try_deserialize()
    }
}
