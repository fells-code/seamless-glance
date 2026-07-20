//! Cost estimates attached to waste findings.
//!
//! Estimates are AWS public list price for the resource as configured, not
//! what the account is actually billed. Discounts, Savings Plans, Reserved
//! Instances, and usage-based components are not reflected, so the number is an
//! upper bound on the standing charge and is labelled as list price everywhere
//! it is shown.

use serde::{Deserialize, Serialize};

/// Billable hours in a month, the convention AWS uses in its own pricing pages.
pub const HOURS_PER_MONTH: f64 = 730.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CostBasis {
    /// On-demand public list price for a resource that is billed by the hour
    /// simply for existing. Accurate as an order of magnitude for an idle
    /// resource, which is what waste findings are about.
    OnDemandListPrice,
}

impl CostBasis {
    pub fn label(&self) -> &'static str {
        match self {
            CostBasis::OnDemandListPrice => "list",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CostEstimate {
    pub monthly_usd: f64,
    pub basis: CostBasis,
}

impl CostEstimate {
    pub fn from_hourly(hourly_usd: f64, basis: CostBasis) -> Self {
        CostEstimate {
            monthly_usd: hourly_usd * HOURS_PER_MONTH,
            basis,
        }
    }

    /// Rendered for the findings table, always carrying its basis so the number
    /// is never mistaken for billed spend.
    pub fn label(&self) -> String {
        format!("~${:.0}/mo {}", self.monthly_usd, self.basis.label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_hourly_rate_becomes_a_monthly_figure() {
        let estimate = CostEstimate::from_hourly(0.0416, CostBasis::OnDemandListPrice);

        assert!((estimate.monthly_usd - 30.368).abs() < 0.001);
    }

    /// The number is list price, not billed spend, so the label has to say so
    /// wherever it appears.
    #[test]
    fn the_label_always_names_its_basis() {
        let estimate = CostEstimate::from_hourly(0.0225, CostBasis::OnDemandListPrice);

        assert_eq!(estimate.label(), "~$16/mo list");
    }
}
