use axum::Json;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;

#[derive(Deserialize)]
pub struct Input {
    pub reported_ebitda: f64,
    pub adjustments: Vec<Adjustment>,
}

#[derive(Deserialize)]
pub struct Adjustment {
    pub label: String,
    pub amount: f64,
}

#[derive(Serialize)]
pub struct AdjustmentOut {
    pub label: String,
    pub amount: f64,
}

#[derive(Serialize)]
pub struct Output {
    pub reported_ebitda: f64,
    pub total_adjustments: f64,
    pub normalized_ebitda: f64,
    pub adjustments: Vec<AdjustmentOut>,
}

pub async fn handler(Json(input): Json<Input>) -> Result<Json<Output>, ApiError> {
    calculate(&input).map(Json).map_err(ApiError::bad_request)
}

pub fn calculate(input: &Input) -> Result<Output, String> {
    if input.adjustments.is_empty() {
        return Err("at least one adjustment is required".into());
    }

    let total_adjustments: f64 = input.adjustments.iter().map(|a| a.amount).sum();
    let normalized_ebitda = input.reported_ebitda + total_adjustments;

    Ok(Output {
        reported_ebitda: input.reported_ebitda,
        total_adjustments,
        normalized_ebitda,
        adjustments: input
            .adjustments
            .iter()
            .map(|a| AdjustmentOut {
                label: a.label.clone(),
                amount: a.amount,
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slide_example() {
        let result = calculate(&Input {
            reported_ebitda: 12_400_000.0,
            adjustments: vec![
                Adjustment { label: "Founder compensation".into(), amount: 800_000.0 },
                Adjustment { label: "One-time legal".into(), amount: 300_000.0 },
                Adjustment { label: "Office relocation".into(), amount: 150_000.0 },
                Adjustment { label: "Consulting".into(), amount: 400_000.0 },
                Adjustment { label: "Stock compensation".into(), amount: 700_000.0 },
            ],
        })
        .unwrap();

        assert!((result.total_adjustments - 2_350_000.0).abs() < 1.0);
        assert!((result.normalized_ebitda - 14_750_000.0).abs() < 1.0);
    }

    #[test]
    fn negative_adjustment_reduces_ebitda() {
        let result = calculate(&Input {
            reported_ebitda: 10_000_000.0,
            adjustments: vec![Adjustment { label: "Non-recurring gain".into(), amount: -500_000.0 }],
        })
        .unwrap();
        assert!((result.normalized_ebitda - 9_500_000.0).abs() < 1.0);
    }

    #[test]
    fn rejects_empty() {
        assert!(calculate(&Input {
            reported_ebitda: 1.0,
            adjustments: vec![]
        })
        .is_err());
    }
}
