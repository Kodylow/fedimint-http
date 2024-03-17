use axum::extract::{Json, Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::state::AppState;
use crate::router::handlers::cashu::{Method, Unit, PostMintQuoteMethodRequest};
use crate::services::quote::QuoteService; // Assuming QuoteService is used for handling quotes.

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostMintQuoteMethodRequest {
    pub amount: Amount,
    pub unit: Unit,
    pub federation_id: Option<FederationId>,
}

#[derive(Debug, Serialize)]
pub struct PostMintQuoteMethodResponse {
    pub quote_id: String,
    pub paid: bool,
    pub expiry: u64,
}

/// Handler for minting a quote based on the provided method.
#[axum_macros::debug_handler]
pub async fn handle_method(
    Path(method): Path<Method>,
    State(state): State<AppState>,
    Json(req): Json<PostMintQuoteMethodRequest>,
) -> Result<Json<PostMintQuoteMethodResponse>, AppError> {
    let client = state.get_client(req.federation_id).await?;
    let quote_service = QuoteService::new(client.clone(), state.quote_store.clone());

    let quote = match method {
        Method::Bolt11 => quote_service.generate_quote(req.amount, req.unit, req.federation_id).await?,
        _ => return Err(AppError::new(
            StatusCode::BAD_REQUEST,
            anyhow!("Unsupported mint method: {:?}", method),
        )),
    };

    Ok(Json(PostMintQuoteMethodResponse {
        quote_id: quote.id,
        paid: quote.paid,
        expiry: quote.expiry,
    }))
}

/// Handler for retrieving a quote by its ID.
#[axum_macros::debug_handler]
pub async fn handle_method_quote_id(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Quote>, AppError> {
    let quote_service = QuoteService::new(state.get_client(None).await?, state.quote_store.clone());
    let quote = quote_service.retrieve_quote(&id).await?;

    Ok(Json(quote))
}
