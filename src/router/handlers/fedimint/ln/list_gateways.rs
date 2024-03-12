use anyhow::anyhow;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use fedimint_client::ClientArc;
use fedimint_core::config::FederationId;
use fedimint_ln_client::LightningClientModule;
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::debug;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListGatewaysRequest {
    pub federation_id: Option<FederationId>,
}

async fn _list_gateways(client: ClientArc) -> Result<Value, AppError> {
    debug!("Listing gateways");
    let lightning_module = client.get_first_module::<LightningClientModule>();
    let gateways = lightning_module.fetch_registered_gateways().await?;
    if gateways.is_empty() {
        return Ok(serde_json::to_value(Vec::<String>::new()).unwrap());
    }
    debug!("Fetched gateways: {:?}", gateways);

    let mut gateways_json = json!(&gateways);
    let active_gateway = lightning_module.select_active_gateway().await?;
    debug!("Active gateway: {:?}", active_gateway);

    gateways_json
        .as_array_mut()
        .expect("gateways_json is not an array")
        .iter_mut()
        .for_each(|gateway| {
            if gateway["node_pub_key"] == json!(active_gateway.node_pub_key) {
                gateway["active"] = json!(true);
            } else {
                gateway["active"] = json!(false);
            }
        });
    Ok(serde_json::to_value(gateways_json).unwrap())
}

pub async fn handle_ws(state: AppState, v: Value) -> Result<Value, AppError> {
    debug!("Handling WebSocket list gateways request");
    let v = serde_json::from_value::<ListGatewaysRequest>(v)
        .map_err(|e| AppError::new(StatusCode::BAD_REQUEST, anyhow!("Invalid request: {}", e)))?;
    let client = state.get_client(v.federation_id).await?;
    let gateways = _list_gateways(client).await?;
    let gateways_json = json!(gateways);
    Ok(gateways_json)
}

#[axum_macros::debug_handler]
pub async fn handle_rest(
    State(state): State<AppState>,
    Json(req): Json<ListGatewaysRequest>,
) -> Result<Json<Value>, AppError> {
    debug!("Handling REST list gateways request");
    let client = state.get_client(req.federation_id).await?;
    let gateways = _list_gateways(client).await?;
    Ok(Json(gateways))
}
