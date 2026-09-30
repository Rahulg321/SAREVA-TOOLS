use axum::Json;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;

#[derive(Deserialize)]
pub struct Input {
    pub mandate: Mandate,
    pub deal: Deal,
}

#[derive(Deserialize, Default)]
pub struct Mandate {
    #[serde(default)]
    pub revenue_min: Option<f64>,
    #[serde(default)]
    pub revenue_max: Option<f64>,
    #[serde(default)]
    pub ebitda_min: Option<f64>,
    #[serde(default)]
    pub min_ebitda_margin: Option<f64>,
    #[serde(default)]
    pub min_growth: Option<f64>,
    #[serde(default)]
    pub max_net_debt_to_ebitda: Option<f64>,
}

#[derive(Deserialize)]
pub struct Deal {
    pub revenue: f64,
    pub ebitda: f64,
    pub growth: f64,
    pub debt: f64,
}

#[derive(Serialize)]
pub struct Criterion {
    pub label: String,
    pub deal_value: f64,
    pub format: &'static str,
    pub requirement: String,
    pub passed: bool,
}

#[derive(Serialize)]
pub struct Output {
    pub qualified: bool,
    pub passed: u32,
    pub total: u32,
    pub ebitda_margin: f64,
    pub net_debt_to_ebitda: f64,
    pub criteria: Vec<Criterion>,
}

pub async fn handler(Json(input): Json<Input>) -> Result<Json<Output>, ApiError> {
    evaluate(&input).map(Json).map_err(ApiError::bad_request)
}

pub fn evaluate(input: &Input) -> Result<Output, String> {
    let deal = &input.deal;
    if deal.revenue <= 0.0 {
        return Err("revenue must be greater than 0".into());
    }
    if deal.ebitda == 0.0 {
        return Err("ebitda must be non-zero".into());
    }

    let ebitda_margin = deal.ebitda / deal.revenue;
    let net_debt_to_ebitda = deal.debt / deal.ebitda;
    let m = &input.mandate;
    let mut criteria = Vec::new();

    if let Some(min) = m.revenue_min {
        criteria.push(criterion(
            "Revenue",
            deal.revenue,
            "money",
            format!(">= {}", money(min)),
            deal.revenue >= min,
        ));
    }
    if let Some(max) = m.revenue_max {
        criteria.push(criterion(
            "Revenue",
            deal.revenue,
            "money",
            format!("<= {}", money(max)),
            deal.revenue <= max,
        ));
    }
    if let Some(min) = m.ebitda_min {
        criteria.push(criterion(
            "EBITDA",
            deal.ebitda,
            "money",
            format!(">= {}", money(min)),
            deal.ebitda >= min,
        ));
    }
    if let Some(min) = m.min_ebitda_margin {
        criteria.push(criterion(
            "EBITDA margin",
            ebitda_margin,
            "percent",
            format!(">= {}", percent(min)),
            ebitda_margin >= min,
        ));
    }
    if let Some(min) = m.min_growth {
        criteria.push(criterion(
            "Revenue growth",
            deal.growth,
            "percent",
            format!(">= {}", percent(min)),
            deal.growth >= min,
        ));
    }
    if let Some(max) = m.max_net_debt_to_ebitda {
        criteria.push(criterion(
            "Net debt / EBITDA",
            net_debt_to_ebitda,
            "multiple",
            format!("<= {max:.1}x"),
            net_debt_to_ebitda <= max,
        ));
    }

    let passed = criteria.iter().filter(|c| c.passed).count() as u32;

    Ok(Output {
        qualified: criteria.iter().all(|c| c.passed),
        passed,
        total: criteria.len() as u32,
        ebitda_margin,
        net_debt_to_ebitda,
        criteria,
    })
}

fn criterion(
    label: &str,
    deal_value: f64,
    format: &'static str,
    requirement: String,
    passed: bool,
) -> Criterion {
    Criterion {
        label: label.into(),
        deal_value,
        format,
        requirement,
        passed,
    }
}

fn money(value: f64) -> String {
    let abs = value.abs();
    let sign = if value < 0.0 { "-" } else { "" };
    if abs >= 1e9 {
        format!("{sign}${:.2}B", abs / 1e9)
    } else if abs >= 1e6 {
        format!("{sign}${:.1}M", abs / 1e6)
    } else if abs >= 1e3 {
        format!("{sign}${:.1}K", abs / 1e3)
    } else {
        format!("{sign}${abs:.0}")
    }
}

fn percent(value: f64) -> String {
    format!("{:.1}%", value * 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deal() -> Deal {
        Deal {
            revenue: 50_000_000.0,
            ebitda: 11_000_000.0,
            growth: 0.25,
            debt: 20_000_000.0,
        }
    }

    fn sample_mandate() -> Mandate {
        Mandate {
            revenue_min: Some(10_000_000.0),
            revenue_max: Some(100_000_000.0),
            ebitda_min: Some(5_000_000.0),
            min_ebitda_margin: Some(0.10),
            min_growth: Some(0.05),
            max_net_debt_to_ebitda: Some(4.0),
        }
    }

    #[test]
    fn deal_passes_all_criteria() {
        let result = evaluate(&Input {
            mandate: sample_mandate(),
            deal: deal(),
        })
        .unwrap();
        assert!(result.qualified);
        assert_eq!(result.total, 6);
        assert_eq!(result.passed, 6);
        assert!((result.ebitda_margin - 0.22).abs() < 1e-9);
    }

    #[test]
    fn fails_on_margin_when_below_mandate() {
        let result = evaluate(&Input {
            mandate: sample_mandate(),
            deal: Deal {
                ebitda: 3_000_000.0,
                ..deal()
            },
        })
        .unwrap();
        assert!(!result.qualified);
        let margin = result
            .criteria
            .iter()
            .find(|c| c.label == "EBITDA margin")
            .unwrap();
        assert!(!margin.passed);
        assert_eq!(margin.requirement, ">= 10.0%");
    }

    #[test]
    fn fails_on_leverage() {
        let result = evaluate(&Input {
            mandate: sample_mandate(),
            deal: Deal {
                debt: 60_000_000.0,
                ..deal()
            },
        })
        .unwrap();
        assert!(!result.qualified);
        assert!(result.criteria.iter().any(|c| !c.passed));
    }

    #[test]
    fn only_applied_criteria_are_evaluated() {
        let result = evaluate(&Input {
            mandate: Mandate {
                max_net_debt_to_ebitda: Some(3.0),
                ..Default::default()
            },
            deal: deal(),
        })
        .unwrap();
        assert_eq!(result.total, 1);
        assert!(result.qualified);
    }

    #[test]
    fn empty_mandate_qualifies() {
        let result = evaluate(&Input {
            mandate: Mandate::default(),
            deal: deal(),
        })
        .unwrap();
        assert!(result.qualified);
        assert_eq!(result.total, 0);
    }

    #[test]
    fn rejects_bad_deal() {
        assert!(evaluate(&Input {
            mandate: Mandate::default(),
            deal: Deal {
                revenue: 0.0,
                ..deal()
            }
        })
        .is_err());
        assert!(evaluate(&Input {
            mandate: Mandate::default(),
            deal: Deal {
                ebitda: 0.0,
                ..deal()
            }
        })
        .is_err());
    }
}
