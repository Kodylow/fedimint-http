use anyhow::anyhow;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use fedimint_client::ClientArc;
use serde_json::{json, Value};

use crate::error::AppError;
use crate::state::AppState;

async fn _restore(_client: ClientArc, _v: Value) -> Result<(), AppError> {
    // TODO: unimplemented in cli
    Err(AppError::new(
        StatusCode::INTERNAL_SERVER_ERROR,
        anyhow!("Not implemented"),
    ))
}

pub async fn handle_ws(state: AppState, v: Value) -> Result<Value, AppError> {
    let client = state.get_client(None).await?;
    _restore(client, v).await?;
    Ok(json!(()))
}

#[utoipa::path(
post,
tag="Restore",
path="/fedimint/v2/admin/restore",
request_body(content = Object, description = "Restore request", content_type = "application/json"),
responses(
(status = 200, description = "Restore the previously created backup of mint notes (with backup command).", body = Object),
(status = 500, description = "Internal Server Error", body = AppError),
(status = 422, description = "Unprocessable Entity", body = AppError)
)
)]
#[axum_macros::debug_handler]
pub async fn handle_rest(
    State(state): State<AppState>,
    Json(req): Json<Value>,
) -> Result<Json<()>, AppError> {
    let client = state.get_client(None).await?;
    _restore(client, req).await?;
    Ok(Json(()))
}
