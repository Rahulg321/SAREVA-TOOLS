use axum::{extract::State, http::HeaderMap, Json};
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::{guard, llm, AppState};

#[derive(Deserialize)]
pub struct Input {
    pub description: String,
    #[serde(default)]
    pub turnstile_token: String,
}

#[derive(Serialize, Deserialize)]
pub struct Output {
    pub formula: String,
    pub explanation: String,
}

#[worker::send]
pub async fn handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<Input>,
) -> Result<Json<Output>, ApiError> {
    guard::check(
        &state.env,
        &headers,
        "formula-generate",
        &input.turnstile_token,
    )
    .await?;

    if input.description.trim().is_empty() {
        return Err(ApiError::bad_request("description is required"));
    }

    let system = "You write spreadsheet formulas for a financial model. Prefer Excel-compatible syntax. \
Respond with raw JSON only, no markdown, matching exactly: {\"formula\": string, \"explanation\": string}.";
    let user = format!("Create a formula for: {}", input.description.trim());

    let output: Output = llm::complete_json(&state.env, system, &user, 500)
        .await
        .map_err(ApiError::bad_gateway)?;
    Ok(Json(output))
}
