mod config;
mod smpp;
mod web;
mod model;

use crate::config::AppConfig;
use dotenvy::dotenv;
use tracing::{info, warn};

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

    // Start Web Server
    let web_config = std::sync::Arc::new(config.clone());
    std::thread::spawn(move || {
        let sys = actix_rt::System::new();
        if let Err(e) = sys.block_on(web::start_web_server(web_config)) {
             tracing::error!("Web server error: {}", e);
        }
    });

    // Placeholder for SMPP Server
    info!("SMPP Server listening on 0.0.0.0:{}", config.smpp.port);

    // Keep the main thread alive (simulate long running process)
    // In the future, the SMPP server loop will be awaited here
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    }
    
    Ok(())
}
