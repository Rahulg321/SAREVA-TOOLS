use axum::Json;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;

#[derive(Deserialize)]
pub struct Input {
    pub entry_ev: f64,
    pub entry_debt: f64,
    pub exit_debt: f64,
    pub hold_years: f64,
    pub target_irr: f64,
}

#[derive(Serialize)]
pub struct LadderRow {
    pub target_irr: f64,
    pub required_exit_ev: f64,
}

#[derive(Serialize)]
pub struct Output {
    pub entry_equity: f64,
    pub target_irr: f64,
    pub required_exit_equity: f64,
    pub required_exit_ev: f64,
    pub ladder: Vec<LadderRow>,
}

pub async fn handler(Json(input): Json<Input>) -> Result<Json<Output>, ApiError> {
    calculate(&input).map(Json).map_err(ApiError::bad_request)
}

pub fn calculate(input: &Input) -> Result<Output, String> {
    let entry_equity = input.entry_ev - input.entry_debt;
    if entry_equity <= 0.0 {
        return Err("entry equity must be greater than 0 (entry EV must exceed entry debt)".into());
    }
    if input.hold_years <= 0.0 {
        return Err("hold period must be greater than 0".into());
    }
    if input.target_irr <= -1.0 {
        return Err("target IRR must be greater than -100%".into());
    }

    let required = |target: f64| entry_equity * (1.0 + target).powf(input.hold_years);

    let ladder = [0.15, 0.20, 0.25, 0.30]
        .iter()
        .map(|&t| LadderRow {
            target_irr: t,
            required_exit_ev: required(t) + input.exit_debt,
        })
        .collect();

    Ok(Output {
        entry_equity,
        target_irr: input.target_irr,
        required_exit_equity: required(input.target_irr),
        required_exit_ev: required(input.target_irr) + input.exit_debt,
        ladder,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slide_example_25pct() {
        let result = calculate(&Input {
            entry_ev: 100e6,
            entry_debt: 40e6,
            exit_debt: 20e6,
            hold_years: 5.0,
            target_irr: 0.25,
        })
        .unwrap();

        assert!((result.entry_equity - 60e6).abs() < 1.0);
        assert!((result.required_exit_equity - 183.1e6).abs() < 0.2e6);
        assert!((result.required_exit_ev - 203.1e6).abs() < 0.2e6);
    }

    #[test]
    fn ladder_rises_with_target() {
        let result = calculate(&Input {
            entry_ev: 100e6,
            entry_debt: 40e6,
            exit_debt: 20e6,
            hold_years: 5.0,
            target_irr: 0.25,
        })
        .unwrap();
        let evs: Vec<f64> = result.ladder.iter().map(|r| r.required_exit_ev).collect();
        assert!(evs.windows(2).all(|w| w[1] > w[0]));
    }

    #[test]
    fn rejects_bad_input() {
        let base = Input {
            entry_ev: 100e6,
            entry_debt: 40e6,
            exit_debt: 20e6,
            hold_years: 5.0,
            target_irr: 0.25,
        };
        assert!(calculate(&Input { entry_debt: 100e6, ..base }).is_err());
        assert!(calculate(&Input { hold_years: 0.0, ..base }).is_err());
        assert!(calculate(&Input { target_irr: -1.5, ..base }).is_err());
    }
}
