use anyhow::{anyhow, Context};
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use fedimint_client::ClientArc;
use fedimint_core::config::FederationId;
use fedimint_core::core::OperationId;
use fedimint_ln_client::{LightningClientModule, PayType};
use serde::Deserialize;
use serde_json::{json, Value};
use utoipa::ToSchema;

use super::pay::LnPayResponse;
use super::wait_for_ln_payment;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AwaitLnPayRequest {
    #[schema(value_type = String)]
    pub operation_id: OperationId,
    #[schema(value_type = String)]
    pub federation_id: Option<FederationId>,
}

async fn _await_pay(client: ClientArc, req: AwaitLnPayRequest) -> Result<LnPayResponse, AppError> {
    let lightning_module = client.get_first_module::<LightningClientModule>();
    let ln_pay_details = lightning_module
        .get_ln_pay_details_for(req.operation_id)
        .await?;
    let payment_type = if ln_pay_details.is_internal_payment {
        PayType::Internal(req.operation_id)
    } else {
        PayType::Lightning(req.operation_id)
    };
    wait_for_ln_payment(
        &client,
        payment_type,
        ln_pay_details.contract_id.to_string(),
        false,
    )
    .await?
    .context("expected a response")
    .map_err(|e| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, e))
}

pub async fn handle_ws(state: AppState, v: Value) -> Result<Value, AppError> {
    let v = serde_json::from_value::<AwaitLnPayRequest>(v)
        .map_err(|e| AppError::new(StatusCode::BAD_REQUEST, anyhow!("Invalid request: {}", e)))?;
    let client = state.get_client(v.federation_id).await?;
    let pay = _await_pay(client, v).await?;
    let pay_json = json!(pay);
    Ok(pay_json)
}

#[utoipa::path(
post,
tag="Await pay",
path="/fedimint/v2/ln/await-pay",
request_body(content = AwaitLnPayRequest, description = "Await pay request", content_type = "application/json"),
responses(
(status = 200, description = "Wait for a lightning payment to complete.", body = LnPayResponse),
(status = 500, description = "Internal Server Error", body = AppError),
(status = 422, description = "Unprocessable Entity", body = AppError)
)
)]
#[axum_macros::debug_handler]
pub async fn handle_rest(
    State(state): State<AppState>,
    Json(req): Json<AwaitLnPayRequest>,
) -> Result<Json<LnPayResponse>, AppError> {
    let client = state.get_client(req.federation_id).await?;
    let pay = _await_pay(client, req).await?;
    Ok(Json(pay))
}
