use rust_smpp_sim::config::AppConfig;
use rust_smpp_sim::smpp::session::SessionManager;
use rust_smpp_sim::smpp::queue::MessageQueue;
use rust_smpp_sim::web::{LogBuffer, LogBufferLayer};
use rust_smpp_sim::{smpp, web};
use dotenvy::dotenv;
use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
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

    // Create log buffer for web UI streaming
    let log_buffer = LogBuffer::new();

    // Initialize logging with custom layer for web UI
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::new(format!("{},{}", config.log.level, "actix_web=info")))
        .with(LogBufferLayer::new(log_buffer.clone()))
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
    let web_log_buffer = log_buffer.clone();
    std::thread::spawn(move || {
        let sys = actix_rt::System::new();
        if let Err(e) = sys.block_on(web::start_web_server(web_config, web_session_manager, web_message_queue, web_log_buffer)) {
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
