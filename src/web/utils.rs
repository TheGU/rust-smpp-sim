use actix_web::{http::header::ContentType, HttpResponse};
use askama::Template;

pub trait RenderToResponse {
    fn render_to_response(&self) -> HttpResponse;
}

impl<T: Template> RenderToResponse for T {
    fn render_to_response(&self) -> HttpResponse {
        match self.render() {
            Ok(content) => HttpResponse::Ok()
                .content_type(ContentType::html())
                .body(content),
            Err(e) => {
                tracing::error!("Template rendering error: {}", e);
                HttpResponse::InternalServerError()
                    .body("Internal Server Error")
            }
        }
    }
}
