use anyhow::anyhow;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use fedimint_client::ClientArc;
use fedimint_core::config::FederationId;
use fedimint_core::core::OperationId;
use fedimint_wallet_client::{DepositState, WalletClientModule};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use utoipa::ToSchema;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AwaitDepositRequest {
    #[schema(value_type = String)]
    pub operation_id: OperationId,
    #[schema(value_type = String)]
    pub federation_id: Option<FederationId>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AwaitDepositResponse {
    #[schema(value_type = String)]
    pub status: DepositState,
}

async fn _await_deposit(
    client: ClientArc,
    req: AwaitDepositRequest,
) -> Result<AwaitDepositResponse, AppError> {
    let mut updates = client
        .get_first_module::<WalletClientModule>()
        .subscribe_deposit_updates(req.operation_id)
        .await?
        .into_stream();

    while let Some(update) = updates.next().await {
        match update {
            DepositState::Confirmed(tx) => {
                return Ok(AwaitDepositResponse {
                    status: DepositState::Confirmed(tx),
                })
            }
            DepositState::Claimed(tx) => {
                return Ok(AwaitDepositResponse {
                    status: DepositState::Claimed(tx),
                })
            }
            DepositState::Failed(reason) => {
                return Err(AppError::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    anyhow!(reason),
                ))
            }
            _ => {}
        }
    }

    Err(AppError::new(
        StatusCode::INTERNAL_SERVER_ERROR,
        anyhow!("Unexpected end of stream"),
    ))
}

pub async fn handle_ws(state: AppState, v: Value) -> Result<Value, AppError> {
    let v = serde_json::from_value::<AwaitDepositRequest>(v)
        .map_err(|e| AppError::new(StatusCode::BAD_REQUEST, anyhow!("Invalid request: {}", e)))?;
    let client = state.get_client(v.federation_id).await?;
    let await_deposit = _await_deposit(client, v).await?;
    let await_deposit_json = json!(await_deposit);
    Ok(await_deposit_json)
}

#[utoipa::path(
post,
tag="Await Deposit",
path="/fedimint/v2/onchain/await-deposit",
request_body(content = AwaitDepositRequest, description = "Wait deposit request", content_type = "application/json"),
responses(
(status = 200, description = "Wait for deposit on previously generated address.", body = AwaitDepositResponse),
(status = 500, description = "Internal Server Error", body = AppError),
(status = 422, description = "Unprocessable Entity", body = AppError)
)
)]
#[axum_macros::debug_handler]
pub async fn handle_rest(
    State(state): State<AppState>,
    Json(req): Json<AwaitDepositRequest>,
) -> Result<Json<AwaitDepositResponse>, AppError> {
    let client = state.get_client(req.federation_id).await?;
    let await_deposit = _await_deposit(client, req).await?;
    Ok(Json(await_deposit))
}
