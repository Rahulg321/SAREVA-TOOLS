use axum::Json;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;

#[derive(Deserialize)]
pub struct Input {
    pub entry_ev: f64,
    pub entry_debt: f64,
    pub exit_ev: f64,
    pub exit_debt: f64,
    pub hold_years: f64,
}

#[derive(Serialize)]
pub struct Sensitivity {
    pub col_label: &'static str,
    pub row_label: &'static str,
    pub value_format: &'static str,
    pub cols: Vec<f64>,
    pub rows: Vec<f64>,
    pub values: Vec<Vec<f64>>,
}

#[derive(Serialize)]
pub struct Output {
    pub entry_equity: f64,
    pub exit_equity: f64,
    pub moic: f64,
    pub irr: f64,
    pub sensitivity: Sensitivity,
}

pub async fn handler(Json(input): Json<Input>) -> Result<Json<Output>, ApiError> {
    calculate(&input).map(Json).map_err(ApiError::bad_request)
}

pub fn calculate(input: &Input) -> Result<Output, String> {
    let entry_equity = input.entry_ev - input.entry_debt;
    let exit_equity = input.exit_ev - input.exit_debt;

    if entry_equity <= 0.0 {
        return Err("entry equity must be greater than 0 (entry EV must exceed entry debt)".into());
    }
    if input.hold_years <= 0.0 {
        return Err("hold period must be greater than 0".into());
    }

    let moic = exit_equity / entry_equity;
    let irr = annualized(moic, input.hold_years);

    let cols: Vec<f64> = [0.6, 0.8, 1.0, 1.2, 1.4]
        .iter()
        .map(|s| input.exit_ev * s)
        .collect();

    let base = input.hold_years.round().max(1.0);
    let rows: Vec<f64> = (-2..=2)
        .map(|d| base + d as f64)
        .filter(|y| *y >= 1.0)
        .collect();

    let values = rows
        .iter()
        .map(|&y| {
            cols.iter()
                .map(|&ev| annualized((ev - input.exit_debt) / entry_equity, y))
                .collect()
        })
        .collect();

    Ok(Output {
        entry_equity,
        exit_equity,
        moic,
        irr,
        sensitivity: Sensitivity {
            col_label: "Exit EV",
            row_label: "Hold (yrs)",
            value_format: "percent",
            cols,
            rows,
            values,
        },
    })
}

fn annualized(moic: f64, years: f64) -> f64 {
    if moic <= 0.0 {
        -1.0
    } else {
        moic.powf(1.0 / years) - 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corrected_slide_example() {
        let result = calculate(&Input {
            entry_ev: 100e6,
            entry_debt: 30e6,
            exit_ev: 250e6,
            exit_debt: 20e6,
            hold_years: 5.0,
        })
        .unwrap();

        assert!((result.entry_equity - 70e6).abs() < 1.0);
        assert!((result.exit_equity - 230e6).abs() < 1.0);
        assert!((result.moic - 3.2857).abs() < 1e-3);
        assert!((result.irr - 0.2686).abs() < 1e-3);
    }

    #[test]
    fn slide_figures_came_from_debt_of_40() {
        let result = calculate(&Input {
            entry_ev: 100e6,
            entry_debt: 40e6,
            exit_ev: 250e6,
            exit_debt: 20e6,
            hold_years: 5.0,
        })
        .unwrap();

        assert!((result.moic - 3.8333).abs() < 1e-3);
        assert!((result.irr - 0.3081).abs() < 1e-3);
    }

    #[test]
    fn one_year_irr_equals_moic_minus_one() {
        let result = calculate(&Input {
            entry_ev: 100e6,
            entry_debt: 40e6,
            exit_ev: 180e6,
            exit_debt: 20e6,
            hold_years: 1.0,
        })
        .unwrap();
        assert!((result.irr - (result.moic - 1.0)).abs() < 1e-9);
    }

    #[test]
    fn center_sensitivity_cell_equals_base_irr() {
        let result = calculate(&Input {
            entry_ev: 100e6,
            entry_debt: 30e6,
            exit_ev: 250e6,
            exit_debt: 20e6,
            hold_years: 5.0,
        })
        .unwrap();
        let s = &result.sensitivity;
        let row = s.rows.iter().position(|&y| y == 5.0).unwrap();
        let col = s.cols.iter().position(|&c| c == 250e6).unwrap();
        assert!((s.values[row][col] - result.irr).abs() < 1e-9);
    }

    #[test]
    fn rejects_non_positive_equity_and_hold() {
        let mut input = Input {
            entry_ev: 100e6,
            entry_debt: 30e6,
            exit_ev: 250e6,
            exit_debt: 20e6,
            hold_years: 5.0,
        };
        input.entry_debt = 100e6;
        assert!(calculate(&input).is_err());
        input.entry_debt = 30e6;
        input.hold_years = 0.0;
        assert!(calculate(&input).is_err());
    }
}
