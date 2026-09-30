use axum::Json;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;

#[derive(Deserialize)]
pub struct Input {
    pub revenue: f64,
    pub ebitda: f64,
    pub growth: f64,
    pub debt: f64,
}

#[derive(Serialize)]
pub struct Breakdown {
    pub profitability: u32,
    pub growth: u32,
    pub leverage: u32,
}

#[derive(Serialize)]
pub struct Output {
    pub ebitda_margin: f64,
    pub debt_to_ebitda: f64,
    pub score: u32,
    pub qualified: bool,
    pub breakdown: Breakdown,
}

pub async fn handler(Json(input): Json<Input>) -> Result<Json<Output>, ApiError> {
    calculate(&input).map(Json).map_err(ApiError::bad_request)
}

pub fn calculate(input: &Input) -> Result<Output, String> {
    if input.revenue <= 0.0 {
        return Err("revenue must be greater than 0".into());
    }
    if input.ebitda == 0.0 {
        return Err("ebitda must be non-zero".into());
    }

    let ebitda_margin = input.ebitda / input.revenue;
    let debt_to_ebitda = input.debt / input.ebitda;

    let breakdown = Breakdown {
        profitability: score_profitability(ebitda_margin),
        growth: score_growth(input.growth),
        leverage: score_leverage(debt_to_ebitda),
    };
    let score = breakdown.profitability + breakdown.growth + breakdown.leverage;

    Ok(Output {
        ebitda_margin,
        debt_to_ebitda,
        score,
        qualified: score >= 70 && debt_to_ebitda <= 4.0 && ebitda_margin >= 0.10,
        breakdown,
    })
}

fn score_profitability(margin: f64) -> u32 {
    match margin {
        m if m >= 0.20 => 40,
        m if m >= 0.15 => 30,
        m if m >= 0.10 => 20,
        m if m >= 0.05 => 10,
        _ => 0,
    }
}

fn score_growth(growth: f64) -> u32 {
    match growth {
        g if g >= 0.20 => 30,
        g if g >= 0.10 => 20,
        g if g >= 0.05 => 10,
        _ => 0,
    }
}

fn score_leverage(debt_to_ebitda: f64) -> u32 {
    match debt_to_ebitda {
        d if d <= 1.0 => 30,
        d if d <= 2.0 => 25,
        d if d <= 3.0 => 15,
        d if d <= 4.0 => 5,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example() -> Input {
        Input {
            revenue: 50_000_000.0,
            ebitda: 11_000_000.0,
            growth: 0.25,
            debt: 20_000_000.0,
        }
    }

    #[test]
    fn example_matches_rubric() {
        let result = calculate(&example()).unwrap();
        assert!((result.ebitda_margin - 0.22).abs() < 1e-9);
        assert!((result.debt_to_ebitda - 1.8181818).abs() < 1e-4);
        assert_eq!(result.score, 95);
        assert!(result.qualified);
    }

    #[test]
    fn weak_company_fails() {
        let weak = Input {
            revenue: 100_000_000.0,
            ebitda: 3_000_000.0,
            growth: -0.10,
            debt: 30_000_000.0,
        };
        assert!(!calculate(&weak).unwrap().qualified);
    }

    #[test]
    fn rejects_bad_input() {
        assert!(calculate(&Input { revenue: 0.0, ..example() }).is_err());
        assert!(calculate(&Input { ebitda: 0.0, ..example() }).is_err());
    }
}
