    use anyhow::anyhow;
    use axum::extract::State;
    use axum::http::StatusCode;
    use axum::Json;
    use axum::extract::ws::{Message, Sender};
    use fedimint_client::ClientArc;
    use fedimint_core::config::FederationId;
    use fedimint_core::core::OperationId;
    use fedimint_wallet_client::{DepositState, WalletClientModule};
    use futures_util::{SinkExt, StreamExt};
    use serde::{Deserialize, Serialize};
    use serde_json::{json, Value};

    use crate::error::AppError;
    use crate::state::AppState;

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct AwaitDepositRequest {
        pub operation_id: OperationId,
        pub federation_id: Option<FederationId>,
    }

    #[derive(Debug, Serialize)] 
    #[serde(rename_all = "camelCase")]
    pub struct AwaitDepositResponse {
        pub status: DepositState,
    }

    pub async fn handle_ws(
        state: AppState,
        params: Value,
        ws_sender: Sender,
    ) -> Result<(), AppError> {
        // Deserialize requested parameters
        let req: AwaitDepositRequest = serde_json::from_value(params)
            .map_err(|e| AppError::new(StatusCode::BAD_REQUEST, anyhow!("Invalid request: {}", e)))?;
    
        let client = state.get_client(req.federation_id).await.map_err(|_| AppError::from(StatusCode::INTERNAL_SERVER_ERROR))?;
    
        let mut updates = client
            .get_first_module::<WalletClientModule>()
            .subscribe_deposit_updates(req.operation_id)
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
    //     Json(req): Json<AwaitDepositRequest>,
    // ) -> Result<Json<AwaitDepositResponse>, AppError> {
    //     let client = state.get_client(req.federation_id).await?;
    //     let await_deposit = _await_deposit(client, req).await?;
    //     Ok(Json(await_deposit))
    // }