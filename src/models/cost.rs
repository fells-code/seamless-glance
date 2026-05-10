use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BudgetInfo {
    pub monthly_budget: f64,
    pub month_to_date_cost: f64,
    pub forecast: f64,
    #[serde(default)]
    pub forecast_low: Option<f64>,
    #[serde(default)]
    pub forecast_high: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UsageTypeCost {
    pub usage_type: String,
    pub monthly_cost: f64,
    pub usage_amount: f64,
    pub unit: String,
}

impl UsageTypeCost {
    pub fn summary(&self) -> String {
        if self.unit.trim().is_empty() {
            format!("{} ${:.2}", self.usage_type, self.monthly_cost)
        } else {
            format!(
                "{} {:.1} {} (${:.2})",
                self.usage_type, self.usage_amount, self.unit, self.monthly_cost
            )
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServiceCostInsight {
    pub service: String,
    pub monthly_cost: f64,
    #[serde(default)]
    pub top_usage_types: Vec<UsageTypeCost>,
}

impl ServiceCostInsight {
    pub fn primary_usage_summary(&self) -> String {
        self.top_usage_types
            .first()
            .map(UsageTypeCost::summary)
            .unwrap_or_else(|| "No usage detail".into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SavingsRoute {
    Ec2,
    Lambda,
    Apigateway,
    LoadBalancers,
}

#[derive(Debug, Clone)]
pub struct CostSavingsOpportunity {
    pub title: String,
    pub service: String,
    pub monthly_cost: f64,
    pub estimated_monthly_savings: f64,
    pub evidence: String,
    pub usage_context: String,
    pub recommendation: String,
    pub route: SavingsRoute,
}
