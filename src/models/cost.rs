use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetInfo {
    pub monthly_budget: f64,
    pub month_to_date_cost: f64,
    pub forecast: f64,
}
