use anyhow::Error;
use axum::extract::State;
use axum::Json;
use fedimint_core::config::FederationId;
use multimint::MultiMint;
use serde::Serialize;
use serde_json::{json, Value};
use tracing::debug;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FederationIdsResponse {
    pub federation_ids: Vec<FederationId>,
}

async fn _federation_ids(multimint: MultiMint) -> Result<FederationIdsResponse, Error> {
    debug!("Fetching federation IDs");
    let federation_ids = multimint.ids().await.into_iter().collect::<Vec<_>>();
    Ok(FederationIdsResponse { federation_ids })
}

pub async fn handle_ws(state: AppState, _v: Value) -> Result<Value, AppError> {
    debug!("Handling WebSocket request for federation IDs");
    let federation_ids = _federation_ids(state.multimint).await?;
    let federation_ids_json = json!(federation_ids);
    Ok(federation_ids_json)
}

#[axum_macros::debug_handler]
pub async fn handle_rest(
    State(state): State<AppState>,
) -> Result<Json<FederationIdsResponse>, AppError> {
    debug!("Handling REST request for federation IDs");
    let federation_ids = _federation_ids(state.multimint).await?;
    Ok(Json(federation_ids))
}
