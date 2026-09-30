use axum::Json;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;

#[derive(Deserialize)]
pub struct Input {
    pub exit_value: f64,
    pub holders: Vec<Holder>,
}

#[derive(Deserialize)]
pub struct Holder {
    pub name: String,
    pub ownership: f64,
    pub preference: f64,
}

#[derive(Serialize)]
pub struct HolderOut {
    pub name: String,
    pub ownership: f64,
    pub preference: f64,
    pub proceeds: f64,
}

#[derive(Serialize)]
pub struct Output {
    pub exit_value: f64,
    pub total_preference: f64,
    pub note: &'static str,
    pub holders: Vec<HolderOut>,
}

pub async fn handler(Json(input): Json<Input>) -> Result<Json<Output>, ApiError> {
    calculate(&input).map(Json).map_err(ApiError::bad_request)
}

pub fn calculate(input: &Input) -> Result<Output, String> {
    if input.exit_value < 0.0 {
        return Err("exit value cannot be negative".into());
    }
    if input.holders.is_empty() {
        return Err("at least one holder is required".into());
    }

    let total_ownership: f64 = input.holders.iter().map(|h| h.ownership).sum();
    if (total_ownership - 1.0).abs() > 0.01 {
        return Err("ownership percentages must sum to 100%".into());
    }
    if input.holders.iter().any(|h| h.preference < 0.0) {
        return Err("liquidation preference cannot be negative".into());
    }

    let total_preference: f64 = input.holders.iter().map(|h| h.preference).sum();

    // Participating preferred: preferences are returned first, then the
    // remainder is split pro-rata by ownership. When the exit cannot cover
    // total preference, the preferred holders split it pro-rata to preference.
    let holders = input
        .holders
        .iter()
        .map(|h| {
            let proceeds = if total_preference > 0.0 && input.exit_value <= total_preference {
                if h.preference > 0.0 {
                    h.preference / total_preference * input.exit_value
                } else {
                    0.0
                }
            } else {
                let remainder = input.exit_value - total_preference;
                h.preference + h.ownership * remainder
            };
            HolderOut {
                name: h.name.clone(),
                ownership: h.ownership,
                preference: h.preference,
                proceeds,
            }
        })
        .collect();

    Ok(Output {
        exit_value: input.exit_value,
        total_preference,
        note: "Participating preferred: preference returned first, remainder split pro-rata.",
        holders,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferences_then_prorata() {
        let result = calculate(&Input {
            exit_value: 150e6,
            holders: vec![
                Holder {
                    name: "Founder".into(),
                    ownership: 0.60,
                    preference: 0.0,
                },
                Holder {
                    name: "Investor A".into(),
                    ownership: 0.25,
                    preference: 25e6,
                },
                Holder {
                    name: "Investor B".into(),
                    ownership: 0.15,
                    preference: 15e6,
                },
            ],
        })
        .unwrap();

        let sum: f64 = result.holders.iter().map(|h| h.proceeds).sum();
        assert!((sum - 150e6).abs() < 1.0);
        // Founder: 0.60 * (150 - 40) = 66
        assert!((result.holders[0].proceeds - 66e6).abs() < 1.0);
    }

    #[test]
    fn all_proceeds_when_below_preference() {
        let result = calculate(&Input {
            exit_value: 20e6,
            holders: vec![
                Holder {
                    name: "Founder".into(),
                    ownership: 0.60,
                    preference: 0.0,
                },
                Holder {
                    name: "Investor A".into(),
                    ownership: 0.25,
                    preference: 25e6,
                },
                Holder {
                    name: "Investor B".into(),
                    ownership: 0.15,
                    preference: 15e6,
                },
            ],
        })
        .unwrap();
        // Preferred split 20 pro-rata to preference: A 12.5, B 7.5, founder 0
        assert!((result.holders[0].proceeds - 0.0).abs() < 1.0);
        assert!((result.holders[1].proceeds - 12.5e6).abs() < 1.0);
        let sum: f64 = result.holders.iter().map(|h| h.proceeds).sum();
        assert!((sum - 20e6).abs() < 1.0);
    }

    #[test]
    fn rejects_bad_ownership() {
        assert!(calculate(&Input {
            exit_value: 100e6,
            holders: vec![Holder {
                name: "X".into(),
                ownership: 0.30,
                preference: 0.0
            }]
        })
        .is_err());
    }
}
