use std::time::Duration;

use anyhow::anyhow;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use bitcoin::Address;
use fedimint_client::ClientArc;
use fedimint_core::config::FederationId;
use fedimint_core::core::OperationId;
use fedimint_core::time::now;
use fedimint_wallet_client::WalletClientModule;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use utoipa::ToSchema;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DepositAddressRequest {
    pub timeout: u64,
    #[schema(value_type = String)]
    pub federation_id: Option<FederationId>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DepositAddressResponse {
    #[schema(value_type = Object, example = json!({"payload": "P2PKH address", "network": "bitcoin"}))]
    pub address: Address,
    #[schema(value_type = String)]
    pub operation_id: OperationId,
}

async fn _deposit_address(
    client: ClientArc,
    req: DepositAddressRequest,
) -> Result<DepositAddressResponse, AppError> {
    let wallet_module = client.get_first_module::<WalletClientModule>();
    let (operation_id, address) = wallet_module
        .get_deposit_address(now() + Duration::from_secs(req.timeout), ())
        .await?;

    Ok(DepositAddressResponse {
        address,
        operation_id,
    })
}

pub async fn handle_ws(state: AppState, v: Value) -> Result<Value, AppError> {
    let v: DepositAddressRequest = serde_json::from_value::<DepositAddressRequest>(v)
        .map_err(|e| AppError::new(StatusCode::BAD_REQUEST, anyhow!("Invalid request: {}", e)))?;
    let client = state.get_client(v.federation_id).await?;
    let withdraw = _deposit_address(client, v).await?;
    let withdraw_json = json!(withdraw);
    Ok(withdraw_json)
}

#[utoipa::path(
post,
tag="Deposit Address",
path="/fedimint/v2/onchain/deposit-address",
request_body(content = DepositAddressRequest, description = "Deposit Address request", content_type = "application/json"),
responses(
(status = 200, description = "Generate a new deposit address, funds sent to it can later be claimed.", body = DepositAddressResponse),
(status = 500, description = "Internal Server Error", body = AppError),
(status = 422, description = "Unprocessable Entity", body = AppError)

)
)]
#[axum_macros::debug_handler]
pub async fn handle_rest(
    State(state): State<AppState>,
    Json(req): Json<DepositAddressRequest>,
) -> Result<Json<DepositAddressResponse>, AppError> {
    let client = state.get_client(req.federation_id).await?;
    let withdraw = _deposit_address(client, req).await?;
    Ok(Json(withdraw))
}
