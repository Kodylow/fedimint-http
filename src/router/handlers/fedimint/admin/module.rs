use anyhow::anyhow;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use fedimint_client::ClientArc;
use fedimint_core::config::FederationId;
use fedimint_core::core::{ModuleInstanceId, ModuleKind};
use serde::Deserialize;
use serde_json::{json, Value};
use utoipa::ToSchema;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub enum ModuleSelector {
    Id(ModuleInstanceId),
    #[schema(value_type = String)]
    Kind(ModuleKind),
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ModuleRequest {
    pub module: ModuleSelector,
    pub args: Vec<String>,
    #[schema(value_type = String)]
    pub federation_id: Option<FederationId>,
}

async fn _module(_client: ClientArc, _req: ModuleRequest) -> Result<(), AppError> {
    // TODO: Figure out how to impl this
    Err(AppError::new(
        StatusCode::INTERNAL_SERVER_ERROR,
        anyhow!("Not implemented"),
    ))
}

pub async fn handle_ws(state: AppState, v: Value) -> Result<Value, AppError> {
    let v = serde_json::from_value::<ModuleRequest>(v).unwrap();
    let client = state.get_client(v.federation_id).await?;
    _module(client, v).await?;
    Ok(json!(()))
}

#[utoipa::path(
post,
tag="Module",
path="/fedimint/v2/admin/module",
request_body(content = ModuleRequest, description = "Module request", content_type = "application/json"),
responses(
(status = 200, description = "Call a module subcommand.", body = Object),
(status = 500, description = "Internal Server Error", body = AppError),
(status = 422, description = "Unprocessable Entity", body = AppError)
)
)]
#[axum_macros::debug_handler]
pub async fn handle_rest(
    State(state): State<AppState>,
    Json(req): Json<ModuleRequest>,
) -> Result<Json<()>, AppError> {
    let client = state.get_client(req.federation_id).await?;
    _module(client, req).await?;
    Ok(Json(()))
}
