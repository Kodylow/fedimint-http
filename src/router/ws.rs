use std::pin::Pin;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures_util::{stream::StreamExt, Stream};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::info;

use crate::{error::AppError, state::AppState};

use super::handlers;

pub const JSONRPC_VERSION: &str = "2.0";
pub const JSONRPC_ERROR_INVALID_REQUEST: i16 = -32600;

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: JsonRpcMethod,
    pub params: Value,
    pub id: u64,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub result: Option<Value>,
    pub error: Option<JsonRpcError>,
    pub id: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JsonRpcError {
    pub code: i16,
    pub message: String,
}

#[derive(Debug, Deserialize, Serialize, Copy, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum JsonRpcSingleMethod {
    AdminInfo,
    AdminBackup,
    AdminConfig,
    AdminDiscoverVersion,
    AdminModule,
    AdminRestore,
    AdminListOperations,
    MintReissue,
    MintSpend,
    MintValidate,
    MintSplit,
    MintCombine,
    LnInvoice,
    LnPay,
    LnListGateways,
    LnSwitchGateway,
    WalletDepositAddress,
}

#[derive(Debug, Deserialize, Serialize, Copy, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum JsonSubscriptionMethod {
    LnAwaitInvoice,
    LnAwaitPay,
    WalletAwaitDeposit,
    WalletWithdraw,
}

#[derive(Debug, Deserialize, Serialize, Copy, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum JsonRpcMethod {
    Single(JsonRpcSingleMethod),
    Subscription(JsonSubscriptionMethod),
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    while let Some(Ok(msg)) = socket.next().await {
        if let Message::Text(text) = msg {
            info!("Received: {}", text);
            let req = match serde_json::from_str::<JsonRpcRequest>(&text) {
                Ok(request) => request,
                Err(err) => {
                    send_err_invalid_req(&mut socket, err, &text).await;
                    continue;
                }
            };
            let state_clone = state.clone();
            match req.method {
                JsonRpcMethod::Single(method) => {
                    let res = match_single_method(req.clone(), method, state_clone).await;
                    let res_msg = create_json_rpc_response(res, req.id);
                    socket.send(res_msg).await.unwrap();
                }
                JsonRpcMethod::Subscription(method) => {
                    let res =
                        match_subscription_method(req.clone(), method, socket, state_clone).await;
                    let res_msg = create_json_rpc_response(res, req.id);
                    socket.send(res_msg).await.unwrap();
                }
            };
        }
    }
}

fn create_json_rpc_response(res: Result<Value, AppError>, req_id: u64) -> Message {
    let json_rpc_msg = match res {
        Ok(res) => JsonRpcResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: Some(res),
            error: None,
            id: req_id,
        },
        Err(e) => JsonRpcResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: e.status.as_u16() as i16,
                message: e.error.to_string(),
            }),
            id: req_id,
        },
    };

    // TODO: Proper error handling for serialization, but this should never fail
    let msg_text = serde_json::to_string(&json_rpc_msg).map_err(|err| {
        "Internal Error - Failed to serialize JSON-RPC response: ".to_string() + &err.to_string()
    });

    Message::Text(msg_text.unwrap())
}

async fn send_err_invalid_req(socket: &mut WebSocket, err: serde_json::Error, text: &str) {
    // Try to extract the id from the request
    let id = serde_json::from_str::<Value>(text)
        .ok()
        .and_then(|v| v.get("id").cloned())
        .and_then(|v| v.as_u64());

    let err_msg = JsonRpcResponse {
        jsonrpc: JSONRPC_VERSION.to_string(),
        result: None,
        error: Some(JsonRpcError {
            code: JSONRPC_ERROR_INVALID_REQUEST,
            message: err.to_string(),
        }),
        id: id.unwrap_or(0),
    };
    socket
        .send(Message::Text(serde_json::to_string(&err_msg).unwrap()))
        .await
        .unwrap();
}

async fn match_single_method(
    req: JsonRpcRequest,
    method: JsonRpcSingleMethod,
    state: AppState,
) -> Result<Value, AppError> {
    match method {
        JsonRpcSingleMethod::AdminInfo => {
            handlers::fedimint::admin::info::handle_ws(req.params, state).await
        }
        JsonRpcSingleMethod::AdminBackup => {
            handlers::fedimint::admin::backup::handle_ws(req.params, state).await
        }
        JsonRpcSingleMethod::AdminConfig => {
            handlers::fedimint::admin::config::handle_ws(state).await
        }
        JsonRpcSingleMethod::AdminDiscoverVersion => {
            handlers::fedimint::admin::discover_version::handle_ws(state).await
        }
        JsonRpcSingleMethod::AdminModule => {
            handlers::fedimint::admin::module::handle_ws(req.params, state).await
        }
        JsonRpcSingleMethod::AdminRestore => {
            handlers::fedimint::admin::restore::handle_ws(req.params, state).await
        }
        JsonRpcSingleMethod::AdminListOperations => {
            handlers::fedimint::admin::list_operations::handle_ws(req.params, state).await
        }
        JsonRpcSingleMethod::MintReissue => {
            handlers::fedimint::mint::reissue::handle_ws(req.params, state).await
        }
        JsonRpcSingleMethod::MintSpend => {
            handlers::fedimint::mint::spend::handle_ws(req.params, state).await
        }
        JsonRpcSingleMethod::MintValidate => {
            handlers::fedimint::mint::validate::handle_ws(req.params, state).await
        }
        JsonRpcSingleMethod::MintSplit => {
            handlers::fedimint::mint::split::handle_ws(req.params).await
        }
        JsonRpcSingleMethod::MintCombine => {
            handlers::fedimint::mint::combine::handle_ws(req.params).await
        }
        JsonRpcSingleMethod::LnInvoice => {
            handlers::fedimint::ln::invoice::handle_ws(req.params, state).await
        }
        JsonRpcSingleMethod::LnPay => {
            handlers::fedimint::ln::pay::handle_ws(req.params, state).await
        }
        JsonRpcSingleMethod::LnListGateways => {
            handlers::fedimint::ln::list_gateways::handle_ws(state).await
        }
        JsonRpcSingleMethod::LnSwitchGateway => {
            handlers::fedimint::ln::switch_gateway::handle_ws(req.params, state).await
        }
        JsonRpcSingleMethod::WalletDepositAddress => {
            handlers::fedimint::wallet::deposit_address::handle_ws(req.params, state).await
        }
    }
}

async fn match_subscription_method(
    req: JsonRpcRequest,
    method: JsonSubscriptionMethod,
    mut socket: WebSocket,
    state: AppState,
) {
    let stream: Pin<Box<dyn Stream<Item = Result<JsonRpcResponse, AppError>> + Send + 'static>> =
        match method {
            JsonSubscriptionMethod::LnAwaitInvoice => {
                handlers::fedimint::ln::await_invoice::handle_ws(req.params, state).await
            }
            JsonSubscriptionMethod::LnAwaitPay => {
                handlers::fedimint::ln::await_pay::handle_ws(req.params, state).await
            }
            JsonSubscriptionMethod::WalletAwaitDeposit => {
                handlers::fedimint::wallet::await_deposit::handle_ws(req.params, state).await
            }
            JsonSubscriptionMethod::WalletWithdraw => {
                handlers::fedimint::wallet::withdraw::handle_ws(req.params, state).await
            }
        };

    // Forward all messages from the stream to the WebSocket
    while let Some(result) = stream.next().await {
        let message = serde_json::to_string(&result).map_err(|err| {
            "Internal Error - Failed to serialize JSON-RPC response: ".to_string()
                + &err.to_string()
        })?;
        match result {
            Ok(message) => {
                socket.send(message).await.unwrap();
            }
            Err(e) => {
                // Handle error, e.g. by logging it and/or closing the socket
            }
        }
    }
}
