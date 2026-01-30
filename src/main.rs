mod config;
mod smpp;
mod web;
mod model;

use crate::config::AppConfig;
use crate::smpp::session::SessionManager;
use crate::smpp::queue::MessageQueue;
use dotenvy::dotenv;
use tracing::info;
use std::sync::Arc;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    // Initialize configuration
    let config = match AppConfig::new() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            std::process::exit(1);
        }
    };

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(format!("{},{}", config.log.level, "actix_web=info"))
        .init();

    info!("Starting Rust SMPP Simulator...");
    info!("Configuration loaded: {:?}", config);

    // Create shared state
    let config = Arc::new(config);
    let session_manager = Arc::new(SessionManager::new());
    let message_queue = Arc::new(MessageQueue::new());

    // Start Web Server
    let web_config = config.clone();
    let web_session_manager = session_manager.clone();
    let web_message_queue = message_queue.clone();
    std::thread::spawn(move || {
        let sys = actix_rt::System::new();
        if let Err(e) = sys.block_on(web::start_web_server(web_config, web_session_manager, web_message_queue)) {
             tracing::error!("Web server error: {}", e);
        }
    });

    // Start SMPP Server
    let smpp_config = config.clone();
    let smpp_session_manager = session_manager.clone();
    let smpp_message_queue = message_queue.clone();
    let smpp_server = tokio::spawn(async move {
        if let Err(e) = smpp::server::start_smpp_server(smpp_config, smpp_session_manager, smpp_message_queue).await {
            tracing::error!("SMPP server error: {}", e);
        }
    });

    // Wait for the servers
    let _ = tokio::join!(smpp_server);
    
    Ok(())
}

