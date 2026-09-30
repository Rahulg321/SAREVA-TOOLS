use axum::{extract::State, http::HeaderMap, Json};
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::{guard, llm, AppState};

#[derive(Deserialize)]
pub struct Input {
    pub text: String,
    #[serde(default)]
    pub turnstile_token: String,
}

#[derive(Serialize, Deserialize)]
pub struct Question {
    pub category: String,
    pub question: String,
    pub why: String,
    pub evidence: String,
    pub priority: String,
}

#[derive(Serialize, Deserialize)]
pub struct Output {
    pub questions: Vec<Question>,
}

#[worker::send]
pub async fn handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<Input>,
) -> Result<Json<Output>, ApiError> {
    guard::check(&state.env, &headers, "cim-questions", &input.turnstile_token).await?;

    if input.text.trim().is_empty() {
        return Err(ApiError::bad_request("CIM text is required"));
    }

    let system = "You are a private equity associate reviewing a CIM. Produce 20-25 sharp due-diligence questions \
for management, grouped by category (Revenue, Customers, Competition, Operations, Financials, Working capital, \
Technology, Legal, Management, Deal structure). Set priority to High, Medium or Low. \
Respond with raw JSON only, no markdown, matching exactly: \
{\"questions\": [{\"category\": string, \"question\": string, \"why\": string, \"evidence\": string, \"priority\": string}]}.";
    let user = format!("CIM / company notes:\n{}", input.text.trim());

    let output: Output = llm::complete_json(&state.env, system, &user, 1600)
        .await
        .map_err(ApiError::bad_gateway)?;
    Ok(Json(output))
}
