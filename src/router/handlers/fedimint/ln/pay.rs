use anyhow::{anyhow, Context};
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use fedimint_client::ClientArc;
use fedimint_core::config::FederationId;
use fedimint_core::core::OperationId;
use fedimint_core::Amount;
use fedimint_ln_client::{LightningClientModule, OutgoingLightningPayment, PayType};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::info;
use utoipa::ToSchema;

use crate::error::AppError;
use crate::router::handlers::fedimint::ln::{get_invoice, wait_for_ln_payment};
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LnPayRequest {
    pub payment_info: String,
    #[schema(value_type = u64)]
    pub amount_msat: Option<Amount>,
    pub finish_in_background: bool,
    pub lnurl_comment: Option<String>,
    #[schema(value_type = String)]
    pub federeation_id: Option<FederationId>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LnPayResponse {
    #[schema(value_type = String)]
    pub operation_id: OperationId,
    #[schema(value_type = String)]
    pub payment_type: PayType,
    pub contract_id: String,
    #[schema(value_type = u64)]
    pub fee: Amount,
}

async fn _pay(client: ClientArc, req: LnPayRequest) -> Result<LnPayResponse, AppError> {
    let bolt11 = get_invoice(&req).await?;
    info!("Paying invoice: {bolt11}");
    let lightning_module = client.get_first_module::<LightningClientModule>();
    lightning_module.select_active_gateway().await?;

    let OutgoingLightningPayment {
        payment_type,
        contract_id,
        fee,
    } = lightning_module.pay_bolt11_invoice(bolt11, ()).await?;
    let operation_id = payment_type.operation_id();
    info!("Gateway fee: {fee}, payment operation id: {operation_id}");
    if req.finish_in_background {
        wait_for_ln_payment(&client, payment_type, contract_id.to_string(), true).await?;
        info!("Payment will finish in background, use await-ln-pay to get the result");
        Ok(LnPayResponse {
            operation_id,
            payment_type,
            contract_id: contract_id.to_string(),
            fee,
        })
    } else {
        Ok(
            wait_for_ln_payment(&client, payment_type, contract_id.to_string(), false)
                .await?
                .context("expected a response")?,
        )
    }
}

pub async fn handle_ws(state: AppState, v: Value) -> Result<Value, AppError> {
    let v = serde_json::from_value::<LnPayRequest>(v)
        .map_err(|e| AppError::new(StatusCode::BAD_REQUEST, anyhow!("Invalid request: {}", e)))?;
    let client = state.get_client(v.federeation_id).await?;
    let pay = _pay(client, v).await?;
    let pay_json = json!(pay);
    Ok(pay_json)
}

#[utoipa::path(
post,
tag="Pay",
path="/fedimint/v2/ln/pay",
request_body(content = LnPayRequest, description = "Pay request", content_type = "application/json"),
responses(
(status = 200, description = "Pay a lightning invoice or lnurl via a gateway.", body = LnPayResponse),
(status = 500, description = "Internal Server Error", body = AppError),
(status = 422, description = "Unprocessable Entity", body = AppError)
)
)]
#[axum_macros::debug_handler]
pub async fn handle_rest(
    State(state): State<AppState>,
    Json(req): Json<LnPayRequest>,
) -> Result<Json<LnPayResponse>, AppError> {
    let client = state.get_client(req.federeation_id).await?;
    let pay = _pay(client, req).await?;
    Ok(Json(pay))
}
