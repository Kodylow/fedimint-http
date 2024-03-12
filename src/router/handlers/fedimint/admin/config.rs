use std::collections::HashMap;

use axum::extract::State;
use axum::Json;
use multimint::MultiMint;
use serde_json::{json, Value};
use tracing::debug;

use crate::error::AppError;
use crate::state::AppState;

async fn _config(multimint: MultiMint) -> Result<Value, AppError> {
    debug!("Generating configuration for clients");
    let mut config = HashMap::new();
    for (id, client) in multimint.clients.lock().await.iter() {
        config.insert(*id, client.get_config_json());
    }
    Ok(serde_json::to_value(config).expect("Client config is serializable"))
}

pub async fn handle_ws(state: AppState) -> Result<Value, AppError> {
    debug!("Handling WebSocket configuration request");
    let config = _config(state.multimint).await?;
    let config_json = json!(config);
    Ok(config_json)
}

#[axum_macros::debug_handler]
pub async fn handle_rest(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    debug!("Handling REST configuration request");
    let config = _config(state.multimint).await?;
    Ok(Json(config))
}
