use axum::{Json, response::IntoResponse};

pub mod dto;
pub mod state;
pub mod ws;

pub async fn health_checker_handler() -> impl IntoResponse {
    const MESSAGE: &str = "Working fine, thanks!";

    let json_response = serde_json::json!({
        "status": "success",
        "message": MESSAGE
    });

    Json(json_response)
}
