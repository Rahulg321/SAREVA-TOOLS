use axum::{extract::State, http::HeaderMap, Json};
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::{guard, llm, AppState};

#[derive(Deserialize)]
pub struct Input {
    pub formula: String,
    #[serde(default)]
    pub turnstile_token: String,
}

#[derive(Serialize, Deserialize)]
pub struct Output {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub explanation: String,
    #[serde(default)]
    pub finance_interpretation: String,
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
        "formula-explain",
        &input.turnstile_token,
    )
    .await?;

    if input.formula.trim().is_empty() {
        return Err(ApiError::bad_request("formula is required"));
    }

    let system = "You are a financial analyst. Explain spreadsheet formulas concisely for a finance professional. \
Respond with raw JSON only, no markdown, matching exactly: \
{\"name\": string, \"explanation\": string, \"finance_interpretation\": string}.";
    let user = format!("Formula:\n{}", input.formula.trim());

    let output: Output = llm::complete_json(&state.env, system, &user, 700)
        .await
        .map_err(ApiError::bad_gateway)?;
    Ok(Json(output))
}
