use aws_sdk_costexplorer::{
    types::{DateInterval, GroupDefinition},
    Client as CeClient,
};

use chrono::{Months, Utc};

use crate::models::BudgetInfo;

fn ce_date_range(days: i64) -> (String, String) {
    let end = Utc::now()
        .date_naive()
        .pred_opt() // yesterday
        .expect("valid date");

    let start = end - chrono::Duration::days(days);

    (
        start.format("%Y-%m-%d").to_string(),
        end.format("%Y-%m-%d").to_string(),
    )
}

pub fn last_6_month_labels() -> Vec<String> {
    let now = Utc::now();

    (0..6)
        .map(|i| {
            let month = now.checked_sub_months(Months::new((6 - i) as u32)).unwrap();

            month.format("%b").to_string()
        })
        .collect()
}

pub async fn fetch_service_cost_breakdown() -> Vec<(String, f64)> {
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::v2025_08_07()).await;

    let ce = CeClient::new(&config);

    let (start_str, end_str) = ce_date_range(30);

    let interval = DateInterval::builder()
        .start(&start_str)
        .end(&end_str)
        .build()
        .unwrap();

    let group_def = GroupDefinition::builder()
        .key("SERVICE")
        .r#type("DIMENSION".into())
        .build();

    let resp = ce
        .get_cost_and_usage()
        .time_period(interval)
        .granularity("MONTHLY".into())
        .metrics("UnblendedCost")
        .group_by(group_def)
        .send()
        .await
        .unwrap();

    let mut values = vec![];

    if let Some(result) = resp.results_by_time().first() {
        for group in result.groups() {
            let name = group
                .keys()
                .first()
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string());

            let amount = group
                .metrics()
                .expect("REASON")
                .get("UnblendedCost")
                .and_then(|m| m.amount())
                .and_then(|a| a.parse::<f64>().ok())
                .unwrap_or(0.0);

            values.push((name, amount));
        }
    }

    values
}

pub async fn fetch_last_6_month_costs() -> Vec<f64> {
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::v2025_08_07()).await;

    let ce = CeClient::new(&config);

    let (start_str, end_str) = ce_date_range(180);

    let interval = DateInterval::builder()
        .start(&start_str)
        .end(&end_str)
        .build();

    let resp = ce
        .get_cost_and_usage()
        .time_period(interval.expect("REASON"))
        .granularity("MONTHLY".into())
        .metrics("UnblendedCost")
        .send()
        .await
        .unwrap();

    resp.results_by_time()
        .iter()
        .map(|t| {
            t.total()
                .expect("REASON")
                .get("UnblendedCost")
                .and_then(|c| c.amount())
                .and_then(|a| a.parse::<f64>().ok())
                .unwrap_or(0.0)
        })
        .collect()
}

pub async fn fetch_budget() -> BudgetInfo {
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::v2025_08_07()).await;

    let ce = CeClient::new(&config);

    // Date interval
    let (start_str, end_str) = ce_date_range(30);

    let interval = DateInterval::builder()
        .start(&start_str)
        .end(&end_str)
        .build();

    let resp = match ce
        .get_cost_and_usage()
        .time_period(interval.expect("REASON"))
        .granularity("MONTHLY".into())
        .metrics("UnblendedCost")
        .send()
        .await
    {
        Ok(r) => r,
        Err(err) => {
            eprintln!("Cost Explorer error: {:?}", err);
            return BudgetInfo {
                monthly_budget: 0.0,
                month_to_date_cost: 0.0,
                forecast: 0.0,
            };
        }
    };

    let month_to_date_cost = resp
        .results_by_time()
        .first()
        .and_then(|t| t.total()?.get("UnblendedCost"))
        .and_then(|c| c.amount())
        .and_then(|a| a.parse::<f64>().ok())
        .unwrap_or(0.0);

    let forecast = month_to_date_cost * 1.12;

    BudgetInfo {
        monthly_budget: 100.0,
        month_to_date_cost,
        forecast,
    }
}
