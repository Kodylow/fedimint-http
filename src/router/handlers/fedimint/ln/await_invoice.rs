use anyhow::anyhow;
use axum::extract::ws::Message;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use fedimint_client::ClientArc;
use fedimint_core::config::FederationId;
use fedimint_core::core::OperationId;
use fedimint_ln_client::{LightningClientModule, LnReceiveState};
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::info;

use crate::error::AppError;
use crate::router::handlers::fedimint::admin::get_note_summary;
use crate::router::handlers::fedimint::admin::info::InfoResponse;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AwaitInvoiceRequest {
    pub operation_id: OperationId,
    pub federation_id: Option<FederationId>,
}

pub async fn handle_ws(
    state: AppState,
    params: Value,
    ws_sender: Sender,
) -> Result<(), AppError> {
    // Deserialize requested parameters
    let req: AwaitInvoiceRequest = serde_json::from_value(params)
        .map_err(|e| AppError::new(StatusCode::BAD_REQUEST, anyhow!("Invalid request: {}", e)))?;

    let client = state.get_client(req.federation_id).await.map_err(|_| AppError::from(StatusCode::INTERNAL_SERVER_ERROR))?;

    let mut updates = client
        .get_first_module::<LightningClientModule>()
        .subscribe_ln_receive(req.operation_id)
        .await.map_err(|_| AppError::from(StatusCode::INTERNAL_SERVER_ERROR))?
        .into_stream();

    while let Some(update) = updates.next().await {
        let message = serde_json::to_string(&update).map_err(|_| AppError::from(StatusCode::INTERNAL_SERVER_ERROR))?;
        ws_sender.send(Message::Text(message)).await.map_err(|_| AppError::from(StatusCode::INTERNAL_SERVER_ERROR))?;
    }

    Ok(())
}

// #[axum_macros::debug_handler]
// pub async fn handle_rest(
//     State(state): State<AppState>,
//     Json(req): Json<AwaitInvoiceRequest>,
// ) -> Result<Json<InfoResponse>, AppError> {
//     let client = state.get_client(req.federation_id).await?;
//     let invoice = _await_invoice(client, req).await?;
//     Ok(Json(invoice))
// }
