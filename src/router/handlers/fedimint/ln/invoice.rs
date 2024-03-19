use anyhow::anyhow;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use fedimint_client::ClientArc;
use fedimint_core::config::FederationId;
use fedimint_core::core::OperationId;
use fedimint_core::Amount;
use fedimint_ln_client::LightningClientModule;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use utoipa::ToSchema;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LnInvoiceRequest {
    #[schema(value_type = u64)]
    pub amount_msat: Amount,
    pub description: String,
    pub expiry_time: Option<u64>,
    #[schema(value_type = String)]
    pub federation_id: Option<FederationId>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LnInvoiceResponse {
    #[schema(value_type = String)]
    pub operation_id: OperationId,
    pub invoice: String,
}

async fn _invoice(client: ClientArc, req: LnInvoiceRequest) -> Result<LnInvoiceResponse, AppError> {
    let lightning_module = client.get_first_module::<LightningClientModule>();
    lightning_module.select_active_gateway().await?;

    let (operation_id, invoice) = lightning_module
        .create_bolt11_invoice(req.amount_msat, req.description, req.expiry_time, ())
        .await?;
    Ok(LnInvoiceResponse {
        operation_id,
        invoice: invoice.to_string(),
    })
}

pub async fn handle_ws(state: AppState, v: Value) -> Result<Value, AppError> {
    let v = serde_json::from_value::<LnInvoiceRequest>(v)
        .map_err(|e| AppError::new(StatusCode::BAD_REQUEST, anyhow!("Invalid request: {}", e)))?;
    let client = state.get_client(v.federation_id).await?;
    let invoice = _invoice(client, v).await?;
    let invoice_json = json!(invoice);
    Ok(invoice_json)
}

#[utoipa::path(
post,
tag="Invoice",
path="/fedimint/v2/ln/invoice",
request_body(content = LnInvoiceRequest, description = "Invoice request", content_type = "application/json"),
responses(
(status = 200, description = "Create a lightning invoice to receive payment via gateway.", body = LnInvoiceResponse),
(status = 500, description = "Internal Server Error", body = AppError),
(status = 422, description = "Unprocessable Entity", body = AppError)
)
)]
#[axum_macros::debug_handler]
pub async fn handle_rest(
    State(state): State<AppState>,
    Json(req): Json<LnInvoiceRequest>,
) -> Result<Json<LnInvoiceResponse>, AppError> {
    let client = state.get_client(req.federation_id).await?;
    let invoice = _invoice(client, req).await?;
    Ok(Json(invoice))
}
