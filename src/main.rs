use std::str::FromStr;

use axum::{
    extract::FromRef,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};

use serde::{Deserialize, Serialize};
use solana_sdk::{
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
};
use spl_token::{id as token_program_id, instruction::initialize_mint, state::Mint};

#[tokio::main]
async fn main() {
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());

    let app = Router::new()
        .route("/", get(root))
        .route("/keypair", post(create_keypair))
        .route("/token/create", post(create_token))
        .route("/message/sign", post(sign_message))
        .route("/message/verify", post(verify_message));

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
    let mint_pubkey = Pubkey::from_str(&payload.mint)
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(Response {
                    status: false,
                    data: serde_json::json!({
                        "error": "Invalid mint address"
                    }),
                }),
            )
        })
        .unwrap();
    let mint_authority = Pubkey::from_str(&payload.mintAuthority)
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(Response {
                    status: false,
                    data: serde_json::json!({
                        "error": "Invalid mint authority address"
                    }),
                }),
            )
        })
        .unwrap();

    let ix = initialize_mint(
        &token_program_id(),
        &mint_pubkey,
        &mint_authority,
        None,
        payload.decimals,
    )
    .expect("should build InitializeMint");

    let accounts: Vec<AccountMetaInfo> = ix
        .accounts
        .iter()
        .map(|meta| AccountMetaInfo {
            pubkey: bs58::encode(meta.pubkey.to_bytes()).into_string(),
            is_signer: meta.is_signer,
            is_writable: meta.is_writable,
            owner: bs58::encode(token_program_id().to_bytes()).into_string(),
        })
        .collect::<Vec<_>>();

    let instruction_data = base64::encode(ix.data);

    let response = Response {
        status: true,
        data: serde_json::json!({
            "program_id": ix.program_id.to_string(),
            "accounts": accounts,
            "instruction_data": instruction_data,
        }),
    };

    (StatusCode::OK, Json(response))
}

async fn sign_message(Json(payload): Json<SignData>) -> (StatusCode, Json<Response>) {
    let keypair = Keypair::from_bytes(&bs58::decode(payload.secret).into_vec().unwrap()).unwrap();

    let message = payload.message;
    let signature = keypair.sign_message(message.as_bytes());

    let response = Response {
        status: true,
        data: serde_json::json!({
            "signature": base64::encode(signature),
            "public_key": bs58::encode(keypair.pubkey().to_bytes()).into_string(),
            "message": message,
        }),
    };
    (StatusCode::OK, Json(response))
}

async fn verify_message(Json(payload): Json<VerifyData>) -> (StatusCode, Json<Response>) {
    let signature_bytes = base64::decode(&payload.signature).unwrap();
    let signature = Signature::try_from(signature_bytes.as_slice()).unwrap();
    let message = payload.message;
    let public_key = Pubkey::from_str(&payload.pubkey).unwrap();

    let is_valid_signature = signature.verify(&public_key.to_bytes(), message.as_bytes());

    let response = Response {
        status: true,
        data: serde_json::json!({
            "valid": is_valid_signature,
            "signature": base64::encode(signature),
            "pubkey": bs58::encode(public_key.to_bytes()).into_string(),
            "message": message,
        }),
    };

    (StatusCode::OK, Json(response))
}

#[derive(Serialize, Debug)]
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

#[derive(Deserialize)]
struct SignData {
    message: String,
    secret: String,
}

#[derive(Deserialize)]
struct VerifyData {
    signature: String,
    message: String,
    pubkey: String,
}

#[derive(Serialize)]
struct AccountMetaInfo {
    pubkey: String,
    is_signer: bool,
    is_writable: bool,
    owner: String,
}
