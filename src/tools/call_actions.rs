use axum::{extract::State, http::HeaderMap, Json};
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::{guard, llm, AppState};

#[derive(Deserialize)]
pub struct Input {
    pub transcript: String,
    #[serde(default)]
    pub turnstile_token: String,
}

#[derive(Serialize, Deserialize)]
pub struct Output {
    pub key_takeaways: Vec<String>,
    pub follow_ups: Vec<String>,
    pub red_flags: Vec<String>,
}

#[worker::send]
pub async fn handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<Input>,
) -> Result<Json<Output>, ApiError> {
    guard::check(&state.env, &headers, "call-actions", &input.turnstile_token).await?;

    if input.transcript.trim().is_empty() {
        return Err(ApiError::bad_request("transcript is required"));
    }

    let system = "You are a private equity analyst summarizing a management call. Be specific and concise. \
Respond with raw JSON only, no markdown, matching exactly: \
{\"key_takeaways\": string[], \"follow_ups\": string[], \"red_flags\": string[]}.";
    let user = format!("Transcript:\n{}", input.transcript.trim());

    let output: Output = llm::complete_json(&state.env, system, &user, 900)
        .await
        .map_err(ApiError::bad_gateway)?;
    Ok(Json(output))
}
