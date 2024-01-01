use std::pin::Pin;

use anyhow::Context;
use axum::{extract::State, http::StatusCode, Json};
use fedimint_core::core::OperationId;
use fedimint_ln_client::{LightningClientModule, PayType};
use futures::StreamExt;
use futures_util::Stream;
use serde::Deserialize;
use serde_json::{json, Value};

use super::{
    ln_payment_updates_internal, ln_payment_updates_lightning, pay::LnPayResponse,
    wait_for_ln_payment,
};
use crate::{
    error::AppError,
    router::ws::{JsonRpcResponse, JSONRPC_VERSION},
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct AwaitLnPayRequest {
    pub operation_id: OperationId,
}

async fn _await_pay(state: AppState, req: AwaitLnPayRequest) -> Result<LnPayResponse, AppError> {
    let lightning_module = state.fm.get_first_module::<LightningClientModule>();
    let ln_pay_details = lightning_module
        .get_ln_pay_details_for(req.operation_id)
        .await?;
    let payment_type = if ln_pay_details.is_internal_payment {
        PayType::Internal(req.operation_id)
    } else {
        PayType::Lightning(req.operation_id)
    };
    wait_for_ln_payment(
        &state.fm,
        payment_type,
        ln_pay_details.contract_id.to_string(),
        false,
    )
    .await?
    .context("expected a response")
    .map_err(|e| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, e))
}

pub async fn handle_ws(
    req: Value,
    state: AppState,
) -> Result<Pin<Box<dyn Stream<Item = Result<JsonRpcResponse, AppError>> + Send + 'static>>, AppError>
{
    let req: AwaitLnPayRequest = serde_json::from_value(req).unwrap();
    let lightning_module = &state.fm.get_first_module::<LightningClientModule>();
    let ln_pay_details = lightning_module
        .get_ln_pay_details_for(req.operation_id)
        .await
        .unwrap();
    let payment_type = if ln_pay_details.is_internal_payment {
        PayType::Internal(req.operation_id)
    } else {
        PayType::Lightning(req.operation_id)
    };

    match payment_type {
        PayType::Internal(operation_id) => {
            let updates = ln_payment_updates_internal(lightning_module, operation_id).await?;
            let stream = updates.map(move |update| {
                let update_json = json!(update);
                let response = JsonRpcResponse {
                    jsonrpc: JSONRPC_VERSION.to_string(),
                    result: Some(update_json),
                    error: None,
                    id: 0,
                };
                Ok(response)
            });
            return Box::pin(stream);
        }
        PayType::Lightning(operation_id) => {
            let updates = ln_payment_updates_lightning(lightning_module, operation_id).await?;
            let stream = updates.map(move |update| {
                let update_json = json!(update);
                let response = JsonRpcResponse {
                    jsonrpc: JSONRPC_VERSION.to_string(),
                    result: Some(update_json),
                    error: None,
                    id: 0,
                };
                Ok(response)
            });
            return Box::pin(stream);
        }
    };
}

#[axum_macros::debug_handler]
pub async fn handle_rest(
    State(state): State<AppState>,
    Json(req): Json<AwaitLnPayRequest>,
) -> Result<Json<LnPayResponse>, AppError> {
    let pay = _await_pay(state, req).await?;
    Ok(Json(pay))
}
