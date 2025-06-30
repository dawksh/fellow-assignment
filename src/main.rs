use std::str::FromStr;

use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};

use serde::{Deserialize, Serialize};
use solana_program::example_mocks::solana_sdk::system_instruction;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::{id as token_program_id, instruction::initialize_mint};

#[tokio::main]
async fn main() {
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());

    let app = Router::new()
        .route("/", get(root))
        .route("/keypair", post(create_keypair))
        .route("/keypair", get(incorrect_method))
        .route("/token/create", post(create_token))
        .route("/token/create", get(incorrect_method))
        .route("/token/mint", post(mint_token))
        .route("/token/mint", get(incorrect_method))
        .route("/message/sign", post(sign_message))
        .route("/message/sign", get(incorrect_method))
        .route("/message/verify", post(verify_message))
        .route("/message/verify", get(incorrect_method))
        .route("/send/sol", post(send_sol))
        .route("/send/sol", get(incorrect_method))
        .route("/send/token", post(send_token))
        .route("/send/token", get(incorrect_method));

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn root() -> &'static str {
    "Hello, World!"
}

async fn incorrect_method() -> (StatusCode, Json<Response>) {
    (
        StatusCode::BAD_REQUEST,
        Json(Response::Error {
            success: false,
            error: "Method not allowed".to_string(),
        }),
    )
}

async fn create_keypair() -> (StatusCode, Json<Response>) {
    let keypair = Keypair::new();
    let response = Response::Success {
        success: true,
        data: serde_json::json!({
            "pubkey": bs58::encode(keypair.pubkey().to_bytes()).into_string(),
            "secret": bs58::encode(keypair.to_bytes()).into_string(),
        }),
    };
    (StatusCode::OK, Json(response))
}

async fn create_token(Json(payload): Json<MintToken>) -> (StatusCode, Json<Response>) {
    let mint_pubkey = match Pubkey::from_str(&payload.mint) {
        Ok(pk) => pk,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(Response::Error {
                    success: false,
                    error: "Invalid mint address".to_string(),
                }),
            )
        }
    };
    let mint_authority = match Pubkey::from_str(&payload.mintAuthority) {
        Ok(pk) => pk,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(Response::Error {
                    success: false,
                    error: "Invalid mint authority address".to_string(),
                }),
            )
        }
    };
    let ix = match initialize_mint(
        &token_program_id(),
        &mint_pubkey,
        &mint_authority,
        None,
        payload.decimals,
    ) {
        Ok(ix) => ix,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(Response::Error {
                    success: false,
                    error: "Failed to build InitializeMint instruction".to_string(),
                }),
            )
        }
    };
    let accounts: Vec<AccountMetaInfo> = ix
        .accounts
        .iter()
        .map(|meta| AccountMetaInfo {
            pubkey: bs58::encode(meta.pubkey.to_bytes()).into_string(),
            is_signer: meta.is_signer,
            is_writable: meta.is_writable,
        })
        .collect();
    let instruction_data = base64::encode(ix.data);
    let response = Response::Success {
        success: true,
        data: serde_json::json!({
            "program_id": bs58::encode(ix.program_id.to_bytes()).into_string(),
            "accounts": accounts,
            "instruction_data": instruction_data,
        }),
    };
    (StatusCode::OK, Json(response))
}

async fn sign_message(Json(payload): Json<SignData>) -> (StatusCode, Json<Response>) {
    let keypair = match bs58::decode(payload.secret)
        .into_vec()
        .ok()
        .and_then(|v| Keypair::from_bytes(&v).ok())
    {
        Some(kp) => kp,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(Response::Error {
                    success: false,
                    error: "Invalid secret key".to_string(),
                }),
            )
        }
    };
    let message = payload.message;
    let signature = keypair.sign_message(message.as_bytes());
    let response = Response::Success {
        success: true,
        data: serde_json::json!({
            "signature": base64::encode(signature),
            "public_key": bs58::encode(keypair.pubkey().to_bytes()).into_string(),
            "message": message,
        }),
    };
    (StatusCode::OK, Json(response))
}

async fn verify_message(Json(payload): Json<VerifyData>) -> (StatusCode, Json<Response>) {
    let signature_bytes = match base64::decode(&payload.signature) {
        Ok(b) => b,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(Response::Error {
                    success: false,
                    error: "Invalid signature encoding".to_string(),
                }),
            )
        }
    };
    let signature = match Signature::try_from(signature_bytes.as_slice()) {
        Ok(s) => s,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(Response::Error {
                    success: false,
                    error: "Invalid signature format".to_string(),
                }),
            )
        }
    };
    let public_key = match Pubkey::from_str(&payload.pubkey) {
        Ok(pk) => pk,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(Response::Error {
                    success: false,
                    error: "Invalid public key".to_string(),
                }),
            )
        }
    };
    let is_valid_signature = signature.verify(&public_key.to_bytes(), payload.message.as_bytes());
    let response = Response::Success {
        success: true,
        data: serde_json::json!({
            "valid": is_valid_signature,
            "message": payload.message,
            "pubkey": bs58::encode(public_key.to_bytes()).into_string(),
        }),
    };
    (StatusCode::OK, Json(response))
}

async fn send_sol(Json(payload): Json<SendSol>) -> (StatusCode, Json<Response>) {
    let from = match Pubkey::from_str(&payload.from) {
        Ok(pk) => pk,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(Response::Error {
                    success: false,
                    error: "Invalid from address".to_string(),
                }),
            )
        }
    };
    let to = match Pubkey::from_str(&payload.to) {
        Ok(pk) => pk,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(Response::Error {
                    success: false,
                    error: "Invalid to address".to_string(),
                }),
            )
        }
    };
    let ix = system_instruction::transfer(&from, &to, payload.lamports);
    let response = Response::Success {
        success: true,
        data: serde_json::json!({
            "program_id": bs58::encode(ix.program_id.to_bytes()).into_string(),
            "accounts": ix.accounts.iter().map(|meta| bs58::encode(meta.pubkey.to_bytes()).into_string()).collect::<Vec<_>>(),
            "instruction_data": base64::encode(ix.data),
        }),
    };
    (StatusCode::OK, Json(response))
}

async fn send_token(Json(payload): Json<SendToken>) -> (StatusCode, Json<Response>) {
    let destination = match Pubkey::from_str(&payload.destination) {
        Ok(pk) => pk,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(Response::Error {
                    success: false,
                    error: "Invalid destination address".to_string(),
                }),
            )
        }
    };
    let mint = match Pubkey::from_str(&payload.mint) {
        Ok(pk) => pk,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(Response::Error {
                    success: false,
                    error: "Invalid mint address".to_string(),
                }),
            )
        }
    };
    let owner = match Pubkey::from_str(&payload.owner) {
        Ok(pk) => pk,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(Response::Error {
                    success: false,
                    error: "Invalid owner address".to_string(),
                }),
            )
        }
    };
    let source_ata = get_associated_token_address(&owner, &mint);
    let dest_ata = get_associated_token_address(&destination, &mint);
    let ix = match spl_token::instruction::transfer(
        &spl_token::id(),
        &source_ata,
        &dest_ata,
        &owner,
        &[],
        payload.amount,
    ) {
        Ok(ix) => ix,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(Response::Error {
                    success: false,
                    error: "Failed to build transfer instruction".to_string(),
                }),
            )
        }
    };
    let response = Response::Success {
        success: true,
        data: serde_json::json!({
            "program_id": bs58::encode(ix.program_id.to_bytes()).into_string(),
            "accounts": ix.accounts.iter().map(|meta| SendTokenResponse {
                pubkey: bs58::encode(meta.pubkey.to_bytes()).into_string(),
                isSigner: meta.is_signer,
            }).collect::<Vec<_>>(),
            "instruction_data": base64::encode(ix.data),
        }),
    };
    (StatusCode::OK, Json(response))
}

async fn mint_token(Json(payload): Json<MintTokenRequest>) -> (StatusCode, Json<Response>) {
    let mint = match Pubkey::from_str(&payload.mint) {
        Ok(pk) => pk,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(Response::Error {
                    success: false,
                    error: "Invalid mint address".to_string(),
                }),
            )
        }
    };
    let mint_authority = match Pubkey::from_str(&payload.authority) {
        Ok(pk) => pk,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(Response::Error {
                    success: false,
                    error: "Invalid authority address".to_string(),
                }),
            )
        }
    };
    let destination = match Pubkey::from_str(&payload.destination) {
        Ok(pk) => pk,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(Response::Error {
                    success: false,
                    error: "Invalid destination address".to_string(),
                }),
            )
        }
    };
    let ix = match spl_token::instruction::mint_to(
        &spl_token::id(),
        &mint,
        &mint_authority,
        &destination,
        &[],
        payload.amount,
    ) {
        Ok(ix) => ix,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(Response::Error {
                    success: false,
                    error: "Failed to build mint_to instruction".to_string(),
                }),
            )
        }
    };
    let response = Response::Success {
        success: true,
        data: serde_json::json!({
            "program_id": bs58::encode(ix.program_id.to_bytes()).into_string(),
            "accounts": ix.accounts.iter().map(|meta| AccountMetaInfo {
                pubkey: bs58::encode(meta.pubkey.to_bytes()).into_string(),
                is_signer: false,
                is_writable: true,
            }).collect::<Vec<_>>(),
            "instruction_data": base64::encode(ix.data),
        }),
    };
    (StatusCode::OK, Json(response))
}

#[derive(Serialize, Debug)]
#[serde(untagged)]
enum Response {
    Success {
        success: bool,
        data: serde_json::Value,
    },
    Error {
        success: bool,
        error: String,
    },
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

#[derive(Deserialize)]
struct SendSol {
    from: String,
    to: String,
    lamports: u64,
}

#[derive(Deserialize)]
struct SendToken {
    destination: String,
    mint: String,
    owner: String,
    amount: u64,
}

#[derive(Deserialize)]
struct MintTokenRequest {
    mint: String,
    destination: String,
    authority: String,
    amount: u64,
}

#[derive(Serialize)]
struct AccountMetaInfo {
    pubkey: String,
    is_signer: bool,
    is_writable: bool,
}
#[derive(Serialize)]
struct SendTokenResponse {
    pubkey: String,
    isSigner: bool,
}
