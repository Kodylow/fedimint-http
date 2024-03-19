use std::collections::BTreeMap;

use axum::Json;
use fedimint_core::{Amount, TieredMulti};
use fedimint_mint_client::OOBNotes;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use utoipa::ToSchema;

use crate::error::AppError;

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SplitRequest {
    #[schema(value_type = Object)]
    pub notes: OOBNotes,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SplitResponse {
    #[schema(value_type = Object)]
    pub notes: BTreeMap<Amount, OOBNotes>,
}

async fn _split(req: SplitRequest) -> Result<SplitResponse, AppError> {
    let federation = req.notes.federation_id_prefix();
    let notes = req
        .notes
        .notes()
        .iter()
        .map(|(amount, notes)| {
            let notes = notes
                .iter()
                .map(|note| {
                    OOBNotes::new(
                        federation,
                        TieredMulti::new(vec![(*amount, vec![*note])].into_iter().collect()),
                    )
                })
                .collect::<Vec<_>>();
            (*amount, notes[0].clone()) // clone the amount and return a single
                                        // OOBNotes
        })
        .collect::<BTreeMap<_, _>>();

    Ok(SplitResponse { notes })
}

pub async fn handle_ws(v: Value) -> Result<Value, AppError> {
    let v = serde_json::from_value(v).unwrap();
    let split = _split(v).await?;
    let split_json = json!(split);
    Ok(split_json)
}

#[utoipa::path(
post,
tag="Split",
path="/fedimint/v2/mint/split",
request_body(content = SplitRequest, description = "Split request", content_type = "application/json"),
responses(
(status = 200, description = "Splits a string containing multiple e-cash notes (e.g. from the `spend` command) into ones that contain exactly one.", body = SplitResponse),
(status = 500, description = "Internal Server Error", body = AppError),
(status = 422, description = "Unprocessable Entity", body = AppError)
)
)]
#[axum_macros::debug_handler]
pub async fn handle_rest(Json(req): Json<SplitRequest>) -> Result<Json<SplitResponse>, AppError> {
    let split = _split(req).await?;
    Ok(Json(split))
}
