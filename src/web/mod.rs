use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use crate::config::AppConfig;
use std::sync::Arc;

pub mod utils;

#[get("/health")]
async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("OK")
}

#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok().body("Rust SMPP Simulator is running!")
}

pub async fn start_web_server(config: Arc<AppConfig>) -> std::io::Result<()> {
    let server_config = config.server.clone();
    
    tracing::info!("Starting Web UI on {}:{}", server_config.host, server_config.port);

    HttpServer::new(|| {
        App::new()
            .service(health_check)
            .service(index)
    })
    .bind((server_config.host.as_str(), server_config.port))?
    .run()
    .await
}
