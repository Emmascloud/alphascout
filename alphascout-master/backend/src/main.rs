use axum::{
    routing::{get, post},
    Json,
    Router,
};

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    service: String,
}

#[derive(Deserialize)]
struct ChatRequest {
    message: String,
}

#[derive(Serialize)]
struct ChatResponse {
    reply: String,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        service: "AlphaScout".to_string(),
    })
}

async fn chat(
    Json(payload): Json<ChatRequest>,
) -> Json<ChatResponse> {

    let reply =
        alphascout_agent::engine::process_message(payload.message)
            .await;

    Json(ChatResponse { reply })
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let app = Router::new()
        .route("/health", get(health))
        .route("/chat", post(chat));

    let addr = SocketAddr::from(([127,0,0,1],8000));

    println!("🚀 AlphaScout API");
    println!("🌐 http://127.0.0.1:8000");

    let listener =
        tokio::net::TcpListener::bind(addr)
            .await
            .unwrap();

    axum::serve(listener, app)
        .await
        .unwrap();
}
