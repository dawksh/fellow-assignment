use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use solana_sdk::{signature::Keypair, signer::Signer};

#[tokio::main]
async fn main() {
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());

    let app = Router::new()
        .route("/", get(root))
        .route("/keypair", post(create_keypair))
        .route("/token/create", post(create_token));
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn root() -> &'static str {
    "Hello, World!"
}

async fn create_keypair() -> (StatusCode, Json<Response>) {
    let keypair = Keypair::new();
    let response = Response {
        status: true,
        data: serde_json::json!({
            "public_key": bs58::encode(keypair.pubkey().to_bytes()).into_string(),
            "private_key": bs58::encode(keypair.to_bytes()).into_string(),
        }),
    };
    (StatusCode::OK, Json(response))
}

async fn create_token(Json(payload): Json<MintToken>) -> (StatusCode, Json<Response>) {
    let response = Response {
        status: true,
        data: serde_json::json!({
            "mintAuthority": payload.mintAuthority,
            "mint": payload.mint,
            "decimals": payload.decimals,
        }),
    };
    (StatusCode::OK, Json(response))
}

#[derive(Serialize)]
struct Response {
    status: bool,
    data: serde_json::Value,
}

#[derive(Deserialize)]
struct MintToken {
    mintAuthority: String,
    mint: String,
    decimals: u8,
}
