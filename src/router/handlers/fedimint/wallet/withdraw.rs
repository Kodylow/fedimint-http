use anyhow::anyhow;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use bitcoin::Address;
use bitcoin_hashes::hex::ToHex;
use fedimint_client::ClientArc;
use fedimint_core::config::FederationId;
use fedimint_core::BitcoinAmountOrAll;
use fedimint_wallet_client::{WalletClientModule, WithdrawState};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::info;
use utoipa::ToSchema;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawRequest {
    #[schema(value_type = Object, example = json!({"payload": "P2PKH address", "network": "bitcoin"}))]
    pub address: Address,
    #[schema(value_type = String)]
    pub amount_msat: BitcoinAmountOrAll,
    #[schema(value_type = String)]
    pub federation_id: Option<FederationId>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawResponse {
    pub txid: String,
    pub fees_sat: u64,
}

async fn _withdraw(client: ClientArc, req: WithdrawRequest) -> Result<WithdrawResponse, AppError> {
    let wallet_module = client.get_first_module::<WalletClientModule>();
    let (amount, fees) = match req.amount_msat {
        // If the amount is "all", then we need to subtract the fees from
        // the amount we are withdrawing
        BitcoinAmountOrAll::All => {
            let balance = bitcoin::Amount::from_sat(client.get_balance().await.msats / 1000);
            let fees = wallet_module
                .get_withdraw_fees(req.address.clone(), balance)
                .await?;
            let amount = balance.checked_sub(fees.amount());
            if amount.is_none() {
                Err(AppError::new(
                    StatusCode::BAD_REQUEST,
                    anyhow!("Insufficient balance to pay fees"),
                ))?;
            }
            (amount.unwrap(), fees)
        }
        BitcoinAmountOrAll::Amount(amount) => (
            amount,
            wallet_module
                .get_withdraw_fees(req.address.clone(), amount)
                .await?,
        ),
    };
    let absolute_fees = fees.amount();

    info!("Attempting withdraw with fees: {fees:?}");

    let operation_id = wallet_module
        .withdraw(req.address, amount, fees, ())
        .await?;

    let mut updates = wallet_module
        .subscribe_withdraw_updates(operation_id)
        .await?
        .into_stream();

    while let Some(update) = updates.next().await {
        info!("Update: {update:?}");

        match update {
            WithdrawState::Succeeded(txid) => {
                return Ok(WithdrawResponse {
                    txid: txid.to_hex(),
                    fees_sat: absolute_fees.to_sat(),
                });
            }
            WithdrawState::Failed(e) => {
                return Err(AppError::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    anyhow!("Withdraw failed: {:?}", e),
                ));
            }
            _ => continue,
        };
    }

    Err(AppError::new(
        StatusCode::INTERNAL_SERVER_ERROR,
        anyhow!("Update stream ended without outcome"),
    ))
}

pub async fn handle_ws(state: AppState, v: Value) -> Result<Value, AppError> {
    let v = serde_json::from_value::<WithdrawRequest>(v)
        .map_err(|e| AppError::new(StatusCode::BAD_REQUEST, anyhow!("Invalid request: {}", e)))?;
    let client = state.get_client(v.federation_id).await?;
    let withdraw = _withdraw(client, v).await?;
    let withdraw_json = json!(withdraw);
    Ok(withdraw_json)
}

#[utoipa::path(
post,
tag="Withdraw",
path="/fedimint/v2/onchain/withdraw",
request_body(content = WithdrawRequest, description = "Withdraw request", content_type = "application/json"),
responses(
(status = 200, description = "Withdraw funds from the federation.", body = WithdrawResponse),
(status = 500, description = "Internal Server Error", body = AppError),
(status = 422, description = "Unprocessable Entity", body = AppError)
)
)]
#[axum_macros::debug_handler]
pub async fn handle_rest(
    State(state): State<AppState>,
    Json(req): Json<WithdrawRequest>,
) -> Result<Json<WithdrawResponse>, AppError> {
    let client = state.get_client(req.federation_id).await?;
    let withdraw = _withdraw(client, req).await?;
    Ok(Json(withdraw))
}
