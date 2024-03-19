use std::collections::BTreeMap;

use anyhow::anyhow;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use fedimint_client::backup::Metadata;
use fedimint_client::ClientArc;
use fedimint_core::config::FederationId;
use serde::Deserialize;
use serde_json::{json, Value};
use utoipa::ToSchema;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BackupRequest {
    pub metadata: BTreeMap<String, String>,
    #[schema(value_type = String)]
    pub federation_id: Option<FederationId>,
}


async fn _backup(client: ClientArc, req: BackupRequest) -> Result<(), AppError> {
    client
        .backup_to_federation(Metadata::from_json_serialized(req.metadata))
        .await
        .map_err(|e| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, e))
}

pub async fn handle_ws(state: AppState, v: Value) -> Result<Value, AppError> {
    let v = serde_json::from_value::<BackupRequest>(v)
        .map_err(|e| AppError::new(StatusCode::BAD_REQUEST, anyhow!("Invalid request: {}", e)))?;
    let client = state.get_client(v.federation_id).await?;
    _backup(client, v).await?;
    Ok(json!(()))
}


#[utoipa::path(
post,
tag="Backup",
path="/fedimint/v2/admin/backup",
request_body(content = BackupRequest, description = "Backup request", content_type = "application/json"),
responses(
(status = 200, description = "Upload the (encrypted) snapshot of mint notes to federation.", body = ()),
(status = 500, description = "Internal Server Error", body = AppError),
(status = 422, description = "Unprocessable Entity", body = AppError)
)
)]
#[axum_macros::debug_handler]
pub async fn handle_rest(
    State(state): State<AppState>,
    Json(req): Json<BackupRequest>,
) -> Result<Json<()>, AppError> {
    let client = state.get_client(req.federation_id).await?;
    _backup(client, req).await?;
    Ok(Json(()))
}
