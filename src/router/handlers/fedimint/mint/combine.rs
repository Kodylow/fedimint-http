use anyhow::anyhow;
use axum::http::StatusCode;
use axum::Json;
use fedimint_mint_client::OOBNotes;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use utoipa::ToSchema;

use crate::error::AppError;

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CombineRequest {
    #[schema(value_type = Object)]
    pub notes: Vec<OOBNotes>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CombineResponse {
    #[schema(value_type = Object)]
    pub notes: OOBNotes,
}

async fn _combine(req: CombineRequest) -> Result<CombineResponse, AppError> {
    let federation_id_prefix = match req
        .notes
        .iter()
        .map(|notes| notes.federation_id_prefix())
        .all_equal_value()
    {
        Ok(id) => id,
        Err(None) => Err(AppError::new(
            StatusCode::BAD_REQUEST,
            anyhow!("E-cash notes strings from different federations"),
        ))?,
        Err(Some((a, b))) => Err(AppError::new(
            StatusCode::BAD_REQUEST,
            anyhow!(
                "E-cash notes strings from different federations: {:?} and {:?}",
                a,
                b
            ),
        ))?,
    };

    let combined_notes = req
        .notes
        .iter()
        .flat_map(|notes| notes.notes().iter_items().map(|(amt, note)| (amt, *note)))
        .collect();

    let combined_oob_notes = OOBNotes::new(federation_id_prefix, combined_notes);

    Ok(CombineResponse {
        notes: combined_oob_notes,
    })
}

pub async fn handle_ws(v: Value) -> Result<Value, AppError> {
    let v = serde_json::from_value(v).unwrap();
    let combine = _combine(v).await?;
    let combine_json = json!(combine);
    Ok(combine_json)
}

#[utoipa::path(
post,
tag="Combine",
path="/fedimint/v2/mint/combine",
request_body(content = CombineRequest, description = "Combine request", content_type = "application/json"),
responses(
(status = 200, description = "Combines two or more serialized e-cash notes strings.", body = CombineResponse),
(status = 500, description = "Internal Server Error", body = AppError),
(status = 422, description = "Unprocessable Entity", body = AppError)
)
)]
#[axum_macros::debug_handler]
pub async fn handle_rest(
    Json(req): Json<CombineRequest>,
) -> Result<Json<CombineResponse>, AppError> {
    let combine = _combine(req).await?;
    Ok(Json(combine))
}
