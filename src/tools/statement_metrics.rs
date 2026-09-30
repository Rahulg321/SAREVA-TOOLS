use axum::Json;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;

#[derive(Deserialize)]
pub struct Input {
    pub periods: Vec<Period>,
}

#[derive(Deserialize)]
pub struct Period {
    pub label: String,
    pub revenue: f64,
    pub gross_profit: f64,
    pub ebitda: f64,
    pub net_debt: f64,
    pub fcf: f64,
}

#[derive(Serialize)]
pub struct PeriodOut {
    pub label: String,
    pub revenue: f64,
    pub revenue_growth: Option<f64>,
    pub gross_margin: f64,
    pub ebitda_margin: f64,
    pub net_debt_to_ebitda: f64,
    pub fcf_conversion: f64,
}

#[derive(Serialize)]
pub struct Flag {
    pub label: String,
    pub level: &'static str,
    pub detail: String,
}

#[derive(Serialize)]
pub struct Output {
    pub periods: Vec<PeriodOut>,
    pub flags: Vec<Flag>,
}

pub async fn handler(Json(input): Json<Input>) -> Result<Json<Output>, ApiError> {
    calculate(&input).map(Json).map_err(ApiError::bad_request)
}

pub fn calculate(input: &Input) -> Result<Output, String> {
    if input.periods.is_empty() {
        return Err("at least one period is required".into());
    }

    let mut rows: Vec<PeriodOut> = Vec::with_capacity(input.periods.len());
    for (i, p) in input.periods.iter().enumerate() {
        if p.revenue <= 0.0 {
            return Err(format!("period '{}' must have positive revenue", p.label));
        }
        if p.ebitda == 0.0 {
            return Err(format!("period '{}' must have non-zero EBITDA", p.label));
        }
        let revenue_growth = if i == 0 {
            None
        } else {
            let prev = input.periods[i - 1].revenue;
            Some((p.revenue - prev) / prev)
        };
        rows.push(PeriodOut {
            label: p.label.clone(),
            revenue: p.revenue,
            revenue_growth,
            gross_margin: p.gross_profit / p.revenue,
            ebitda_margin: p.ebitda / p.revenue,
            net_debt_to_ebitda: p.net_debt / p.ebitda,
            fcf_conversion: p.fcf / p.ebitda,
        });
    }

    Ok(Output {
        flags: flags(&rows),
        periods: rows,
    })
}

fn flags(rows: &[PeriodOut]) -> Vec<Flag> {
    let mut flags = Vec::new();
    let (Some(last), Some(prev)) = (rows.last(), rows.iter().rev().nth(1)) else {
        return flags;
    };

    if last.ebitda_margin < prev.ebitda_margin {
        flags.push(Flag {
            label: "EBITDA margin declining".into(),
            level: "warn",
            detail: format!(
                "Fell from {:.1}% to {:.1}%",
                prev.ebitda_margin * 100.0,
                last.ebitda_margin * 100.0
            ),
        });
    }
    if let (Some(a), Some(b)) = (prev.revenue_growth, last.revenue_growth) {
        if b > a {
            flags.push(Flag {
                label: "Revenue growth accelerating".into(),
                level: "good",
                detail: format!("{:.1}% -> {:.1}%", a * 100.0, b * 100.0),
            });
        }
    }
    if last.net_debt_to_ebitda < prev.net_debt_to_ebitda {
        flags.push(Flag {
            label: "Leverage decreasing".into(),
            level: "good",
            detail: format!(
                "{:.2}x -> {:.2}x",
                prev.net_debt_to_ebitda, last.net_debt_to_ebitda
            ),
        });
    }
    if last.fcf_conversion < 0.6 {
        flags.push(Flag {
            label: "Weak cash conversion".into(),
            level: "warn",
            detail: format!("FCF/EBITDA at {:.0}%", last.fcf_conversion * 100.0),
        });
    }

    flags
}

#[cfg(test)]
mod tests {
    use super::*;

    fn period(label: &str, revenue: f64, ebitda: f64, net_debt: f64, fcf: f64) -> Period {
        Period {
            label: label.into(),
            revenue,
            gross_profit: revenue * 0.5,
            ebitda,
            net_debt,
            fcf,
        }
    }

    #[test]
    fn computes_metrics_and_flags() {
        let result = calculate(&Input {
            periods: vec![
                period("2023", 100e6, 20e6, 45e6, 14e6),
                period("2024", 120e6, 21.6e6, 40e6, 12e6),
            ],
        })
        .unwrap();

        assert_eq!(result.periods[0].revenue_growth, None);
        assert!((result.periods[1].revenue_growth.unwrap() - 0.20).abs() < 1e-9);
        assert!((result.periods[1].ebitda_margin - 0.18).abs() < 1e-9);

        let labels: Vec<&str> = result.flags.iter().map(|f| f.label.as_str()).collect();
        assert!(labels.contains(&"EBITDA margin declining"));
        assert!(labels.contains(&"Leverage decreasing"));
    }

    #[test]
    fn rejects_bad_input() {
        assert!(calculate(&Input { periods: vec![] }).is_err());
        assert!(calculate(&Input {
            periods: vec![period("X", 0.0, 1.0, 0.0, 0.0)]
        })
        .is_err());
    }
}
