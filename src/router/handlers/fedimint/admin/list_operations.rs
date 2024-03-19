use std::time::UNIX_EPOCH;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use fedimint_client::ClientArc;
use fedimint_core::config::FederationId;
use fedimint_core::core::OperationId;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use time::format_description::well_known::iso8601;
use time::OffsetDateTime;
use utoipa::ToSchema;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListOperationsRequest {
    pub limit: usize,
    #[schema(value_type = String)]
    pub federation_id: Option<FederationId>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationOutput {
    pub id: OperationId,
    pub creation_time: String,
    pub operation_kind: String,
    pub operation_meta: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<serde_json::Value>,
}

async fn _list_operations(
    client: ClientArc,
    req: ListOperationsRequest,
) -> Result<Value, AppError> {
    const ISO8601_CONFIG: iso8601::EncodedConfig = iso8601::Config::DEFAULT
        .set_formatted_components(iso8601::FormattedComponents::DateTime)
        .encode();
    let operations = client
        .operation_log()
        .list_operations(req.limit, None)
        .await
        .into_iter()
        .map(|(k, v)| {
            let creation_time = OffsetDateTime::from_unix_timestamp(
                k.creation_time
                    .duration_since(UNIX_EPOCH)
                    .expect("Couldn't convert time from SystemTime to timestamp")
                    .as_secs() as i64,
            )
            .expect("Couldn't convert time from SystemTime to OffsetDateTime")
            .format(&iso8601::Iso8601::<ISO8601_CONFIG>)
            .expect("Couldn't format OffsetDateTime as ISO8601");

            OperationOutput {
                id: k.operation_id,
                creation_time,
                operation_kind: v.operation_module_kind().to_owned(),
                operation_meta: v.meta(),
                outcome: v.outcome(),
            }
        })
        .collect::<Vec<_>>();

    Ok(json!({
        "operations": operations,
    }))
}

pub async fn handle_ws(state: AppState, v: Value) -> Result<Value, AppError> {
    let v = serde_json::from_value::<ListOperationsRequest>(v).map_err(|e| {
        AppError::new(
            StatusCode::BAD_REQUEST,
            anyhow::anyhow!("Invalid request: {}", e),
        )
    })?;
    let client = state.get_client(v.federation_id).await?;
    let operations = _list_operations(client, v).await?;
    let operations_json = json!(operations);
    Ok(operations_json)
}

#[utoipa::path(
post,
tag="List operations",
path="/fedimint/v2/admin/list-operations",
request_body(content = ListOperationsRequest, description = "List operations request", content_type = "application/json"),
responses(
(status = 200, description = "List operations.", body = Object),
(status = 500, description = "Internal Server Error", body = AppError),
(status = 422, description = "Unprocessable Entity", body = AppError)
)
)]
#[axum_macros::debug_handler]
pub async fn handle_rest(
    State(state): State<AppState>,
    Json(req): Json<ListOperationsRequest>,
) -> Result<Json<Value>, AppError> {
    let client = state.get_client(None).await?;
    let operations = _list_operations(client, req).await?;
    Ok(Json(operations))
}
