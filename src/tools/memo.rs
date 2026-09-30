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
pub struct Memo {
    pub company_overview: String,
    pub revenue_ebitda_trends: String,
    pub growth_rates: String,
    pub key_risks: Vec<String>,
    pub investment_highlights: Vec<String>,
    pub management_questions: Vec<String>,
    pub investment_thesis: String,
    pub ic_summary: String,
}

#[worker::send]
pub async fn handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<Input>,
) -> Result<Json<Memo>, ApiError> {
    guard::check(&state.env, &headers, "memo", &input.turnstile_token).await?;

    if input.text.trim().is_empty() {
        return Err(ApiError::bad_request("company description or financials are required"));
    }

    let system = "You are a private equity analyst writing a concise investment memo. \
Be specific and reference the given facts. Respond with raw JSON only, no markdown, matching exactly: \
{\"company_overview\": string, \"revenue_ebitda_trends\": string, \"growth_rates\": string, \
\"key_risks\": string[], \"investment_highlights\": string[], \"management_questions\": string[], \
\"investment_thesis\": string, \"ic_summary\": string}. \
The ic_summary must be a tight one-paragraph investment-committee summary.";
    let user = format!("Company description, financials and notes:\n{}", input.text.trim());

    let memo: Memo = llm::complete_json(&state.env, system, &user, 1800)
        .await
        .map_err(ApiError::bad_gateway)?;
    Ok(Json(memo))
}
