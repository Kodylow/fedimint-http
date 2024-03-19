use anyhow::anyhow;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use fedimint_client::ClientArc;
use fedimint_core::Amount;
use fedimint_mint_client::{MintClientModule, OOBNotes};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use utoipa::ToSchema;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ValidateRequest {
    #[schema(value_type = Object)]
    pub notes: OOBNotes,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ValidateResponse {
    #[schema(value_type = u64)]
    pub amount_msat: Amount,
}

async fn _validate(client: ClientArc, req: ValidateRequest) -> Result<ValidateResponse, AppError> {
    let amount_msat = client
        .get_first_module::<MintClientModule>()
        .validate_notes(req.notes)
        .await?;

    Ok(ValidateResponse { amount_msat })
}

pub async fn handle_ws(state: AppState, v: Value) -> Result<Value, AppError> {
    let v = serde_json::from_value::<ValidateRequest>(v)
        .map_err(|e| AppError::new(StatusCode::BAD_REQUEST, anyhow!("Invalid request: {}", e)))?;
    let client = state
        .get_client_by_prefix(&v.notes.federation_id_prefix())
        .await?;
    let validate = _validate(client, v).await?;
    let validate_json = json!(validate);
    Ok(validate_json)
}

#[utoipa::path(
post,
tag="Validate",
path="/fedimint/v2/mint/validate",
request_body(content = ValidateRequest, description = "Validate request", content_type = "application/json"),
responses(
(status = 200, description = "Verifies the signatures of e-cash notes, but *not* if they have been spent already.", body = ValidateResponse),
(status = 500, description = "Internal Server Error", body = AppError),
(status = 422, description = "Unprocessable Entity", body = AppError)
)
)]
#[axum_macros::debug_handler]
pub async fn handle_rest(
    State(state): State<AppState>,
    Json(req): Json<ValidateRequest>,
) -> Result<Json<ValidateResponse>, AppError> {
    let client = state
        .get_client_by_prefix(&req.notes.federation_id_prefix())
        .await?;
    let validate = _validate(client, req).await?;
    Ok(Json(validate))
}
