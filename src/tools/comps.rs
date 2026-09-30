use axum::Json;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;

#[derive(Deserialize)]
pub struct Input {
    pub company: Company,
    pub comps: Vec<Comp>,
}

#[derive(Deserialize)]
pub struct Company {
    pub revenue: f64,
    pub ebitda: f64,
    pub growth: f64,
}

#[derive(Deserialize)]
pub struct Comp {
    pub name: String,
    pub ev: f64,
    pub revenue: f64,
    pub ebitda: f64,
    pub growth: f64,
}

#[derive(Serialize)]
pub struct CompOut {
    pub name: String,
    pub ev_revenue: f64,
    pub ev_ebitda: f64,
    pub growth: f64,
}

#[derive(Serialize)]
pub struct Multiples {
    pub ev_revenue: f64,
    pub ev_ebitda: f64,
    pub growth: f64,
}

#[derive(Serialize)]
pub struct Implied {
    pub ev_low: f64,
    pub ev_high: f64,
}

#[derive(Serialize)]
pub struct Output {
    pub company_growth: f64,
    pub comps: Vec<CompOut>,
    pub median: Multiples,
    pub mean: Multiples,
    pub implied: Implied,
}

pub async fn handler(Json(input): Json<Input>) -> Result<Json<Output>, ApiError> {
    calculate(&input).map(Json).map_err(ApiError::bad_request)
}

pub fn calculate(input: &Input) -> Result<Output, String> {
    if input.comps.is_empty() {
        return Err("at least one comparable is required".into());
    }
    if input.company.revenue <= 0.0 || input.company.ebitda <= 0.0 {
        return Err("company revenue and EBITDA must be greater than 0".into());
    }

    let mut rows = Vec::with_capacity(input.comps.len());
    for comp in &input.comps {
        if comp.revenue <= 0.0 || comp.ebitda <= 0.0 {
            return Err(format!(
                "comparable '{}' must have positive revenue and EBITDA",
                comp.name
            ));
        }
        rows.push(CompOut {
            name: comp.name.clone(),
            ev_revenue: comp.ev / comp.revenue,
            ev_ebitda: comp.ev / comp.ebitda,
            growth: comp.growth,
        });
    }

    let ev_rev: Vec<f64> = rows.iter().map(|r| r.ev_revenue).collect();
    let ev_ebitda: Vec<f64> = rows.iter().map(|r| r.ev_ebitda).collect();
    let growth: Vec<f64> = rows.iter().map(|r| r.growth).collect();

    let median = Multiples {
        ev_revenue: median(&ev_rev),
        ev_ebitda: median(&ev_ebitda),
        growth: median(&growth),
    };
    let mean = Multiples {
        ev_revenue: mean(&ev_rev),
        ev_ebitda: mean(&ev_ebitda),
        growth: mean(&growth),
    };

    let from_revenue = input.company.revenue * median.ev_revenue;
    let from_ebitda = input.company.ebitda * median.ev_ebitda;
    let implied = Implied {
        ev_low: from_revenue.min(from_ebitda),
        ev_high: from_revenue.max(from_ebitda),
    };

    Ok(Output {
        company_growth: input.company.growth,
        comps: rows,
        median,
        mean,
        implied,
    })
}

fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        sorted[mid]
    }
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Input {
        Input {
            company: Company {
                revenue: 100e6,
                ebitda: 20e6,
                growth: 0.25,
            },
            comps: vec![
                Comp {
                    name: "Comp A".into(),
                    ev: 420e6,
                    revenue: 100e6,
                    ebitda: 23e6,
                    growth: 0.21,
                },
                Comp {
                    name: "Comp B".into(),
                    ev: 510e6,
                    revenue: 100e6,
                    ebitda: 22.8e6,
                    growth: 0.29,
                },
                Comp {
                    name: "Comp C".into(),
                    ev: 380e6,
                    revenue: 100e6,
                    ebitda: 22.8e6,
                    growth: 0.18,
                },
            ],
        }
    }

    #[test]
    fn median_and_implied() {
        let result = calculate(&sample()).unwrap();
        assert!((result.median.ev_revenue - 4.2).abs() < 1e-9);
        assert!((result.median.growth - 0.21).abs() < 1e-9);
        // implied EV from revenue = 100 * 4.2 = 420
        assert!((result.implied.ev_low - 420e6).abs() < 1e6 || (result.implied.ev_high - 420e6).abs() < 1e6);
    }

    #[test]
    fn rejects_empty_and_bad_comps() {
        assert!(calculate(&Input {
            company: Company {
                revenue: 100e6,
                ebitda: 20e6,
                growth: 0.2
            },
            comps: vec![]
        })
        .is_err());

        let mut bad = sample();
        bad.comps[0].revenue = 0.0;
        assert!(calculate(&bad).is_err());
    }
}
