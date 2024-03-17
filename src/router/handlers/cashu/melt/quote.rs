use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use multimint::client;
use serde::Deserialize;
use serde::Serialize;
use anyhow::anyhow;

use crate::error::AppError;
use crate::state::AppState;

enum MeltQuoteUnit {
    Sat,
}
#[derive(Debug, Deserialize)]
struct PostMeltQuoteRequest {
    request: String,
    unit: MeltQuoteUnit,
}
#[derive(Debug, Serialize)]
struct PostMeltQuoteResponse {
    quote: String,
    amount: u64,
    fee_reverse: u64,
    paid: bool,
    expiry: u64,
}

#[axum_macros::debug_handler]
pub async fn handle_method(State(_state): State<AppState>) -> Result<(), AppError> {
    // TODO: Implement this function

    Ok(())
}

#[axum_macros::debug_handler]
pub async fn handle_rest(
    State(state): State<AppState>,
    Json(req): Json<PostMeltQuoteRequest>,
) -> Result<Json<PostMeltQuoteResponse>, AppError> {
    // TODO: Implement this function
    let client = match state.multimint.get_default().await {
        Some(client) => Ok(client),
            None => Err(AppError::new(
                StatusCode::BAD_REQUEST,
                anyhow!("No default client found "),)),
    };

    let response= PostMeltQuoteResponse{}

    Ok(())
}
