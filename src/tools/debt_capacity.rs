use axum::Json;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;

#[derive(Deserialize)]
pub struct Input {
    pub ebitda: f64,
    pub senior_debt: f64,
    pub mezzanine: f64,
    pub interest_rate: f64,
}

#[derive(Serialize)]
pub struct StressRow {
    pub ebitda: f64,
    pub debt_to_ebitda: f64,
    pub interest_coverage: f64,
}

#[derive(Serialize)]
pub struct Output {
    pub total_debt: f64,
    pub debt_to_ebitda: f64,
    pub annual_interest: f64,
    pub interest_coverage: f64,
    pub stress: Vec<StressRow>,
}

pub async fn handler(Json(input): Json<Input>) -> Result<Json<Output>, ApiError> {
    calculate(&input).map(Json).map_err(ApiError::bad_request)
}

pub fn calculate(input: &Input) -> Result<Output, String> {
    if input.ebitda <= 0.0 {
        return Err("EBITDA must be greater than 0".into());
    }
    if input.senior_debt < 0.0 || input.mezzanine < 0.0 {
        return Err("debt amounts cannot be negative".into());
    }
    let total_debt = input.senior_debt + input.mezzanine;
    if total_debt <= 0.0 {
        return Err("total debt must be greater than 0".into());
    }
    if input.interest_rate < 0.0 {
        return Err("interest rate cannot be negative".into());
    }

    let annual_interest = total_debt * input.interest_rate;
    let coverage = |ebitda: f64| {
        if annual_interest <= 0.0 {
            0.0
        } else {
            ebitda / annual_interest
        }
    };

    let stress = [1.0, 0.9, 0.8, 0.7]
        .iter()
        .map(|&s| {
            let ebitda = input.ebitda * s;
            StressRow {
                ebitda,
                debt_to_ebitda: total_debt / ebitda,
                interest_coverage: coverage(ebitda),
            }
        })
        .collect();

    Ok(Output {
        total_debt,
        debt_to_ebitda: total_debt / input.ebitda,
        annual_interest,
        interest_coverage: coverage(input.ebitda),
        stress,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slide_example_and_coverage_correction() {
        let result = calculate(&Input {
            ebitda: 25e6,
            senior_debt: 50e6,
            mezzanine: 15e6,
            interest_rate: 0.08,
        })
        .unwrap();

        assert!((result.total_debt - 65e6).abs() < 1.0);
        assert!((result.debt_to_ebitda - 2.6).abs() < 1e-9);
        assert!((result.annual_interest - 5.2e6).abs() < 1.0);
        // 25 / 5.2 = 4.8077, not the 3.8x written in the slide
        assert!((result.interest_coverage - 4.8077).abs() < 1e-3);
    }

    #[test]
    fn stress_grid_worsens() {
        let result = calculate(&Input {
            ebitda: 25e6,
            senior_debt: 50e6,
            mezzanine: 15e6,
            interest_rate: 0.08,
        })
        .unwrap();
        assert_eq!(result.stress[0].ebitda, 25e6);
        let ratios: Vec<f64> = result.stress.iter().map(|r| r.debt_to_ebitda).collect();
        assert!(ratios.windows(2).all(|w| w[1] > w[0]));
    }

    #[test]
    fn rejects_bad_input() {
        let base = Input {
            ebitda: 25e6,
            senior_debt: 50e6,
            mezzanine: 15e6,
            interest_rate: 0.08,
        };
        assert!(calculate(&Input { ebitda: 0.0, ..base }).is_err());
        assert!(calculate(&Input { senior_debt: 0.0, mezzanine: 0.0, ..base }).is_err());
    }
}
