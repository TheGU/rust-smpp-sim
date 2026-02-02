use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use askama::Template;
use crate::config::AppConfig;
use crate::smpp::session::SessionManager;
use crate::smpp::queue::{MessageQueue, MoMessageQueue, MoMessage};
use std::sync::Arc;
use serde::{Deserialize, Serialize};

pub mod utils;
pub mod logs;

pub use logs::{LogBuffer, LogBufferLayer};

/// Shared application state
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub session_manager: Arc<SessionManager>,
    pub message_queue: Arc<MessageQueue>,
    pub mo_queue: Arc<MoMessageQueue>,
    pub log_buffer: Arc<LogBuffer>,
}

#[derive(Serialize)]
struct MessageDisplay {
    message_id: String,
    source_addr: String,
    dest_addr: String,
    content: String,
}

#[derive(Serialize)]
struct SessionDisplay {
    id: String,
    system_id: String,
    bind_type: String,
    addr: String,
}

#[derive(Template)]
#[template(path = "dashboard.html")]
struct DashboardTemplate {
    smpp_port: u16,
    // web_port: u16,
    system_id: String,
    session_count: usize,
    message_count: usize,
    pending_dr_count: usize,
    sessions: Vec<SessionDisplay>,
    messages: Vec<MessageDisplay>,
}

#[derive(Deserialize)]
struct InjectMoRequest {
    source: String,
    dest: String,
    message: String,
}

#[get("/health")]
async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("OK")
}

fn get_display_data(data: &web::Data<AppState>) -> (Vec<SessionDisplay>, Vec<MessageDisplay>) {
    let sessions: Vec<SessionDisplay> = data.session_manager.get_all_sessions()
        .into_iter()
        .map(|s| SessionDisplay {
            id: s.id,
            system_id: s.system_id,
            bind_type: format!("{:?}", s.bind_type),
            addr: s.addr.to_string(),
        })
        .collect();
    let messages: Vec<MessageDisplay> = data.message_queue.get_recent_messages()
        .into_iter()
        .map(|m| MessageDisplay {
            message_id: m.message_id,
            source_addr: m.source_addr,
            dest_addr: m.dest_addr,
            content: String::from_utf8_lossy(&m.short_message).to_string(),
        })
        .collect();
    (sessions, messages)
}

#[get("/")]
async fn dashboard(data: web::Data<AppState>) -> impl Responder {
    let (sessions, messages) = get_display_data(&data);

    let template = DashboardTemplate {
        smpp_port: data.config.smpp.port,
        // web_port: data.config.server.port,
        system_id: data.config.smpp.system_id.clone(),
        session_count: sessions.len(),
        message_count: messages.len(),
        pending_dr_count: data.message_queue.pending_dr_count(),
        sessions,
        messages,
    };
    
    match template.render() {
        Ok(html) => HttpResponse::Ok().content_type("text/html").body(html),
        Err(e) => {
            tracing::error!("Template error: {}", e);
            HttpResponse::InternalServerError().body("Template error")
        }
    }
}

#[get("/partials/stats")]
async fn partials_stats(data: web::Data<AppState>) -> impl Responder {
    let (sessions, messages) = get_display_data(&data);

    #[derive(Template)]
    #[template(path = "partials/stats.html")]
    struct T { system_id: String, session_count: usize, message_count: usize, pending_dr_count: usize }

    let template = T {
        system_id: data.config.smpp.system_id.clone(),
        session_count: sessions.len(),
        message_count: messages.len(),
        pending_dr_count: data.message_queue.pending_dr_count(),
    };
    
    match template.render() {
        Ok(html) => HttpResponse::Ok().content_type("text/html").body(html),
        Err(e) => {
            tracing::error!("Template error: {}", e);
            HttpResponse::InternalServerError().body("Template error")
        }
    }
}

#[get("/partials/sessions")]
async fn partials_sessions(data: web::Data<AppState>) -> impl Responder {
    let (sessions, _) = get_display_data(&data);

    #[derive(Template)]
    #[template(path = "partials/sessions.html")]
    struct T { sessions: Vec<SessionDisplay> }

    match (T { sessions }).render() {
        Ok(html) => HttpResponse::Ok().content_type("text/html").body(html),
        Err(e) => {
            tracing::error!("Template error: {}", e);
            HttpResponse::InternalServerError().body("Template error")
        }
    }
}

#[get("/partials/messages")]
async fn partials_messages(data: web::Data<AppState>) -> impl Responder {
    let (_, messages) = get_display_data(&data);

    #[derive(Template)]
    #[template(path = "partials/messages.html")]
    struct T { messages: Vec<MessageDisplay> }

    match (T { messages }).render() {
        Ok(html) => HttpResponse::Ok().content_type("text/html").body(html),
        Err(e) => {
            tracing::error!("Template error: {}", e);
            HttpResponse::InternalServerError().body("Template error")
        }
    }
}

#[get("/api/stats")]
async fn get_stats(data: web::Data<AppState>) -> impl Responder {
    let sessions = data.session_manager.get_all_sessions();
    let messages: Vec<MessageDisplay> = data.message_queue.get_recent_messages()
        .into_iter()
        .map(|m| MessageDisplay {
            message_id: m.message_id,
            source_addr: m.source_addr,
            dest_addr: m.dest_addr,
            content: String::from_utf8_lossy(&m.short_message).to_string(),
        })
        .collect();

    let stats = serde_json::json!({
        "session_count": sessions.len(),
        "message_count": messages.len(),
        "pending_dr_count": data.message_queue.pending_dr_count(),
        "sessions": sessions,
        "messages": messages,
    });
    HttpResponse::Ok().json(stats)
}

#[post("/api/inject-mo")]
async fn inject_mo(data: web::Data<AppState>, body: web::Form<InjectMoRequest>) -> impl Responder {
    tracing::info!("MO Injection: {} -> {}: {}", body.source, body.dest, body.message);
    
    let mo_msg = MoMessage {
         source_addr: body.source.clone(),
         dest_addr: body.dest.clone(),
         short_message: body.message.clone(),
    };
    
    if let Err(e) = data.mo_queue.inject(mo_msg).await {
         tracing::error!("Failed to inject MO message: {}", e);
         return HttpResponse::InternalServerError().body("Failed to inject");
    }
    
    HttpResponse::Ok()
        .content_type("text/html")
        .body("<div class=\"success\">✓ Message queued for delivery</div>")
}

/// Get recent logs as HTML partial (for initial load)
#[get("/partials/logs")]
async fn partials_logs(data: web::Data<AppState>) -> impl Responder {
    let logs = data.log_buffer.get_all();
    let html: String = logs.iter()
        .rev()  // newest first
        .take(50)
        .map(|l| {
            let escaped = l.replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;");
            format!("<div class=\"log-line\">{}</div>", escaped)
        })
        .collect();
    HttpResponse::Ok().content_type("text/html").body(html)
}

/// SSE endpoint for real-time log streaming
#[get("/api/logs/stream")]
async fn logs_stream(data: web::Data<AppState>) -> impl Responder {
    use actix_web::http::header;
    use futures::StreamExt;

    let mut rx = data.log_buffer.subscribe();
    
    let stream = async_stream::stream! {
        while let Ok(line) = rx.recv().await {
            let escaped = line.replace('\n', " ").replace('\r', "");
            yield Ok::<_, std::io::Error>(
                actix_web::web::Bytes::from(format!("data: {}\n\n", escaped))
            );
        }
    };

    HttpResponse::Ok()
        .insert_header((header::CONTENT_TYPE, "text/event-stream"))
        .insert_header((header::CACHE_CONTROL, "no-cache"))
        .streaming(stream.boxed())
}

pub async fn start_web_server(
    config: Arc<AppConfig>,
    session_manager: Arc<SessionManager>,
    message_queue: Arc<MessageQueue>,
    mo_queue: Arc<MoMessageQueue>,
    log_buffer: Arc<LogBuffer>,
) -> std::io::Result<()> {
    let server_config = config.server.clone();
    
    let app_state = web::Data::new(AppState {
        config: config.clone(),
        session_manager,
        message_queue,
        mo_queue,
        log_buffer,
    });
    
    tracing::info!("Starting Web UI on {}:{}", server_config.host, server_config.port);

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(health_check)
            .service(dashboard)
            .service(partials_stats)
            .service(partials_sessions)
            .service(partials_messages)
            .service(partials_logs)
            .service(logs_stream)
            .service(get_stats)
            .service(inject_mo)
            .service(actix_files::Files::new("/static", "static").show_files_listing())
    })
    .bind((server_config.host.as_str(), server_config.port))?
    .run()
    .await
}
