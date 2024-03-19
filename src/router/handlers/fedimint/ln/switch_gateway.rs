use std::str::FromStr;

use anyhow::anyhow;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use bitcoin::secp256k1::PublicKey;
use fedimint_client::ClientArc;
use fedimint_core::config::FederationId;
use fedimint_ln_client::LightningClientModule;
use serde::Deserialize;
use serde_json::{json, Value};
use utoipa::ToSchema;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwitchGatewayRequest {
    pub gateway_id: String,
    #[schema(value_type = String)]
    pub federation_id: Option<FederationId>,
}

async fn _switch_gateway(client: ClientArc, req: SwitchGatewayRequest) -> Result<Value, AppError> {
    let public_key = PublicKey::from_str(&req.gateway_id)?;
    let lightning_module = client.get_first_module::<LightningClientModule>();
    lightning_module.set_active_gateway(&public_key).await?;
    let gateway = lightning_module.select_active_gateway().await?;
    let mut gateway_json = json!(&gateway);
    gateway_json["active"] = json!(true);
    Ok(serde_json::to_value(gateway_json).unwrap())
}

pub async fn handle_ws(state: AppState, v: Value) -> Result<Value, AppError> {
    let v = serde_json::from_value::<SwitchGatewayRequest>(v)
        .map_err(|e| AppError::new(StatusCode::BAD_REQUEST, anyhow!("Invalid request: {}", e)))?;
    let client = state.get_client(None).await?;
    let gateway = _switch_gateway(client, v).await?;
    let gateway_json = json!(gateway);
    Ok(gateway_json)
}

#[utoipa::path(
post,
tag="Switch gateway",
path="/fedimint/v2/ln/switch-gateway",
request_body(content = SwitchGatewayRequest, description = "Switch gateway request", content_type = "application/json"),
responses(
(status = 200, description = "Switch active gateway.", body = Object),
(status = 500, description = "Internal Server Error", body = AppError),
(status = 422, description = "Unprocessable Entity", body = AppError)
)
)]
#[axum_macros::debug_handler]
pub async fn handle_rest(
    State(state): State<AppState>,
    Json(req): Json<SwitchGatewayRequest>,
) -> Result<Json<Value>, AppError> {
    let client = state.get_client(req.federation_id).await?;
    let gateway = _switch_gateway(client, req).await?;
    Ok(Json(gateway))
}
