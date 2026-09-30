use axum::{extract::State, http::HeaderMap, Json};
use serde::{Deserialize, Deserializer, Serialize};

use crate::error::ApiError;
use crate::{guard, llm, AppState};

/// Accepts a number or a numeric string; null / unparseable → None.
fn de_opt_f64<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(match value {
        Some(serde_json::Value::Number(n)) => n.as_f64(),
        Some(serde_json::Value::String(s)) => {
            s.replace([',', '$', ' '], "").trim().parse::<f64>().ok()
        }
        _ => None,
    })
}

#[derive(Deserialize)]
pub struct Input {
    pub text: String,
    #[serde(default)]
    pub turnstile_token: String,
}

/// Raw fields the model pulls out of the CIM. Ratios are computed in Rust.
#[derive(Deserialize, Default)]
pub struct Extracted {
    #[serde(default)]
    pub company_name: Option<String>,
    #[serde(default)]
    pub industry: Option<String>,
    #[serde(default)]
    pub geography: Option<String>,
    #[serde(default)]
    pub business_model: Option<String>,
    #[serde(default, deserialize_with = "de_opt_f64")]
    pub employees: Option<f64>,
    #[serde(default, deserialize_with = "de_opt_f64")]
    pub revenue: Option<f64>,
    #[serde(default, deserialize_with = "de_opt_f64")]
    pub revenue_growth: Option<f64>,
    #[serde(default, deserialize_with = "de_opt_f64")]
    pub ebitda: Option<f64>,
    #[serde(default, deserialize_with = "de_opt_f64")]
    pub net_debt: Option<f64>,
    #[serde(default, deserialize_with = "de_opt_f64")]
    pub customer_concentration: Option<f64>,
    #[serde(default)]
    pub risks: Vec<String>,
    #[serde(default)]
    pub management_questions: Vec<String>,
}

#[derive(Serialize)]
pub struct Flag {
    pub label: String,
    pub level: &'static str,
    pub detail: String,
}

#[derive(Serialize)]
pub struct Snapshot {
    pub company_name: Option<String>,
    pub industry: Option<String>,
    pub geography: Option<String>,
    pub business_model: Option<String>,
    pub employees: Option<f64>,
    pub revenue: Option<f64>,
    pub revenue_growth: Option<f64>,
    pub ebitda: Option<f64>,
    pub ebitda_margin: Option<f64>,
    pub net_debt: Option<f64>,
    pub net_debt_to_ebitda: Option<f64>,
    pub flags: Vec<Flag>,
    pub risks: Vec<String>,
    pub management_questions: Vec<String>,
}

#[worker::send]
pub async fn handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<Input>,
) -> Result<Json<Snapshot>, ApiError> {
    guard::check(&state.env, &headers, "cim-snapshot", &input.turnstile_token).await?;

    if input.text.trim().is_empty() {
        return Err(ApiError::bad_request("CIM text is required"));
    }

    let system = "You extract structured data from a CIM (confidential information memorandum). \
Respond with raw JSON only, no markdown, matching exactly: \
{\"company_name\": string|null, \"industry\": string|null, \"geography\": string|null, \
\"business_model\": string|null, \"employees\": number|null, \
\"revenue\": number|null (ABSOLUTE currency amount, e.g. 84200000 — never millions), \
\"revenue_growth\": number|null (decimal, e.g. 0.21), \
\"ebitda\": number|null (ABSOLUTE currency amount), \
\"net_debt\": number|null (ABSOLUTE currency amount), \
\"customer_concentration\": number|null (top-10 customers as a decimal), \
\"risks\": string[], \"management_questions\": string[]}. \
Use null when the document does not state a value. Convert any millions/billions to absolute units. \
Do not compute any ratios yourself.";
    let user = format!("CIM text:\n{}", input.text.trim());

    let extracted: Extracted = llm::complete_json(&state.env, system, &user, 1400)
        .await
        .map_err(ApiError::bad_gateway)?;
    Ok(Json(compute(&extracted)))
}

pub fn compute(e: &Extracted) -> Snapshot {
    let ebitda_margin = divide(e.ebitda, e.revenue);
    let net_debt_to_ebitda = divide(e.net_debt, e.ebitda);

    Snapshot {
        company_name: e.company_name.clone(),
        industry: e.industry.clone(),
        geography: e.geography.clone(),
        business_model: e.business_model.clone(),
        employees: e.employees,
        revenue: e.revenue,
        revenue_growth: e.revenue_growth,
        ebitda: e.ebitda,
        ebitda_margin,
        net_debt: e.net_debt,
        net_debt_to_ebitda,
        flags: build_flags(
            e.revenue_growth,
            ebitda_margin,
            net_debt_to_ebitda,
            e.customer_concentration,
        ),
        risks: e.risks.clone(),
        management_questions: e.management_questions.clone(),
    }
}

fn divide(numerator: Option<f64>, denominator: Option<f64>) -> Option<f64> {
    match (numerator, denominator) {
        (Some(n), Some(d)) if d != 0.0 => Some(n / d),
        _ => None,
    }
}

fn build_flags(
    growth: Option<f64>,
    margin: Option<f64>,
    leverage: Option<f64>,
    concentration: Option<f64>,
) -> Vec<Flag> {
    let mut flags = Vec::new();

    if let Some(g) = growth {
        if g >= 0.10 {
            flags.push(good("Revenue growth", format!("{:.1}%", g * 100.0)));
        } else {
            flags.push(warn("Revenue growth", format!("{:.1}% — below 10%", g * 100.0)));
        }
    }
    if let Some(m) = margin {
        if m >= 0.15 {
            flags.push(good("EBITDA margin", format!("{:.1}%", m * 100.0)));
        } else {
            flags.push(warn("EBITDA margin", format!("{:.1}% — below 15%", m * 100.0)));
        }
    }
    if let Some(l) = leverage {
        if l <= 3.0 {
            flags.push(good("Leverage", format!("{l:.2}x")));
        } else {
            flags.push(warn("Leverage", format!("{l:.2}x — above 3.0x")));
        }
    }
    if let Some(c) = concentration {
        if c <= 0.30 {
            flags.push(good("Customer concentration", format!("{:.0}%", c * 100.0)));
        } else {
            flags.push(warn(
                "Customer concentration",
                format!("{:.0}% — above 30%", c * 100.0),
            ));
        }
    }

    flags
}

fn good(label: &str, detail: String) -> Flag {
    Flag {
        label: label.into(),
        level: "good",
        detail,
    }
}

fn warn(label: &str, detail: String) -> Flag {
    Flag {
        label: label.into(),
        level: "warn",
        detail,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Extracted {
        Extracted {
            company_name: Some("Acme SaaS".into()),
            revenue: Some(84_200_000.0),
            revenue_growth: Some(0.21),
            ebitda: Some(16_840_000.0),
            net_debt: Some(31_000_000.0),
            customer_concentration: Some(0.42),
            risks: vec!["Customer concentration".into()],
            management_questions: vec!["Why did DSO increase?".into()],
            ..Default::default()
        }
    }

    #[test]
    fn computes_ratios_in_rust() {
        let snapshot = compute(&sample());
        assert!((snapshot.ebitda_margin.unwrap() - 0.20).abs() < 1e-9);
        assert!((snapshot.net_debt_to_ebitda.unwrap() - 1.8409).abs() < 1e-3);
    }

    #[test]
    fn flags_concentration_warns_others_pass() {
        let snapshot = compute(&sample());
        let level = |label: &str| {
            snapshot
                .flags
                .iter()
                .find(|f| f.label == label)
                .map(|f| f.level)
                .unwrap()
        };
        assert_eq!(level("Revenue growth"), "good");
        assert_eq!(level("EBITDA margin"), "good");
        assert_eq!(level("Leverage"), "good");
        assert_eq!(level("Customer concentration"), "warn");
    }

    #[test]
    fn missing_fields_yield_no_ratio_and_no_flag() {
        let snapshot = compute(&Extracted::default());
        assert!(snapshot.ebitda_margin.is_none());
        assert!(snapshot.net_debt_to_ebitda.is_none());
        assert!(snapshot.flags.is_empty());
    }
}
