use std::collections::BTreeMap;

use aws_sdk_costexplorer::types::{DateInterval, Granularity, GroupDefinition, Metric};
use chrono::{Datelike, Months, NaiveDate, Utc};

use crate::{
    app::App,
    models::{service_status::ServiceStatus, BudgetInfo, ServiceCostInsight, UsageTypeCost},
};

fn today_exclusive() -> NaiveDate {
    Utc::now().date_naive()
}

fn first_day_of_month(date: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(date.year(), date.month(), 1).expect("valid first day of month")
}

fn first_day_of_next_month(date: NaiveDate) -> NaiveDate {
    if date.month() == 12 {
        NaiveDate::from_ymd_opt(date.year() + 1, 1, 1).expect("valid first day of next year")
    } else {
        NaiveDate::from_ymd_opt(date.year(), date.month() + 1, 1)
            .expect("valid first day of next month")
    }
}

fn interval_strings(start: NaiveDate, end_exclusive: NaiveDate) -> (String, String) {
    (
        start.format("%Y-%m-%d").to_string(),
        end_exclusive.format("%Y-%m-%d").to_string(),
    )
}

fn interval(start: NaiveDate, end_exclusive: NaiveDate) -> DateInterval {
    let (start, end) = interval_strings(start, end_exclusive);

    DateInterval::builder()
        .start(start)
        .end(end)
        .build()
        .expect("valid Cost Explorer interval")
}

fn current_month_dates() -> (NaiveDate, NaiveDate) {
    let today = today_exclusive();
    (first_day_of_month(today), today)
}

fn forecast_month_dates() -> (NaiveDate, NaiveDate) {
    let today = today_exclusive();
    (today, first_day_of_next_month(today))
}

fn trailing_six_month_dates() -> (NaiveDate, NaiveDate) {
    let today = today_exclusive();
    let start = first_day_of_month(
        today
            .checked_sub_months(Months::new(5))
            .expect("valid six month window"),
    );

    (start, today)
}

pub fn last_6_month_labels() -> Vec<String> {
    let now = today_exclusive();

    (0..6)
        .map(|i| {
            now.checked_sub_months(Months::new((5 - i) as u32))
                .expect("valid month")
                .format("%b")
                .to_string()
        })
        .collect()
}

fn metric_amount(metric_value: Option<&aws_sdk_costexplorer::types::MetricValue>) -> f64 {
    metric_value
        .and_then(|metric| metric.amount())
        .and_then(|amount| amount.parse::<f64>().ok())
        .unwrap_or(0.0)
}

fn metric_unit(metric_value: Option<&aws_sdk_costexplorer::types::MetricValue>) -> String {
    metric_value
        .and_then(|metric| metric.unit())
        .unwrap_or("")
        .to_string()
}

pub async fn fetch_service_cost_insights(app: &App) -> (Vec<ServiceCostInsight>, ServiceStatus) {
    let (start, end) = current_month_dates();
    let service_group = GroupDefinition::builder()
        .key("SERVICE")
        .r#type("DIMENSION".into())
        .build();
    let usage_type_group = GroupDefinition::builder()
        .key("USAGE_TYPE")
        .r#type("DIMENSION".into())
        .build();

    let response = match app
        .aws
        .ce
        .get_cost_and_usage()
        .time_period(interval(start, end))
        .granularity(Granularity::Monthly)
        .metrics(Metric::UnblendedCost.as_str())
        .metrics(Metric::UsageQuantity.as_str())
        .group_by(service_group)
        .group_by(usage_type_group)
        .send()
        .await
    {
        Ok(response) => response,
        Err(err) => return (vec![], ServiceStatus::from_sdk_error(&err)),
    };

    let mut usage_by_service = BTreeMap::<String, Vec<UsageTypeCost>>::new();

    if let Some(result) = response.results_by_time().first() {
        for group in result.groups() {
            let service = group
                .keys()
                .first()
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string());
            let usage_type = group
                .keys()
                .get(1)
                .cloned()
                .unwrap_or_else(|| "Unknown usage".to_string());

            let metrics = group.metrics();
            let monthly_cost =
                metric_amount(metrics.and_then(|values| values.get("UnblendedCost")));
            let usage_amount =
                metric_amount(metrics.and_then(|values| values.get("UsageQuantity")));
            let unit = metric_unit(metrics.and_then(|values| values.get("UsageQuantity")));

            if monthly_cost <= 0.0 && usage_amount <= 0.0 {
                continue;
            }

            usage_by_service
                .entry(service)
                .or_default()
                .push(UsageTypeCost {
                    usage_type,
                    monthly_cost,
                    usage_amount,
                    unit,
                });
        }
    }

    let insights = usage_by_service
        .into_iter()
        .filter_map(|(service, mut usage_lines)| {
            usage_lines.sort_by(|left, right| {
                right
                    .monthly_cost
                    .partial_cmp(&left.monthly_cost)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            let monthly_cost = usage_lines
                .iter()
                .map(|usage| usage.monthly_cost)
                .sum::<f64>();

            (monthly_cost > 0.0).then_some(ServiceCostInsight {
                service,
                monthly_cost,
                top_usage_types: usage_lines.into_iter().take(3).collect(),
            })
        })
        .collect();

    (insights, ServiceStatus::Ok)
}

pub async fn fetch_last_6_month_costs(app: &App) -> (Vec<f64>, ServiceStatus) {
    let (start, end) = trailing_six_month_dates();

    let response = match app
        .aws
        .ce
        .get_cost_and_usage()
        .time_period(interval(start, end))
        .granularity(Granularity::Monthly)
        .metrics(Metric::UnblendedCost.as_str())
        .send()
        .await
    {
        Ok(response) => response,
        Err(err) => return (vec![0.0; 6], ServiceStatus::from_sdk_error(&err)),
    };

    let mut values = response
        .results_by_time()
        .iter()
        .map(|time_bucket| {
            metric_amount(
                time_bucket
                    .total()
                    .and_then(|total| total.get("UnblendedCost")),
            )
        })
        .collect::<Vec<_>>();

    while values.len() < 6 {
        values.insert(0, 0.0);
    }

    values.truncate(6);
    (values, ServiceStatus::Ok)
}

pub async fn fetch_budget(app: &App) -> (BudgetInfo, ServiceStatus) {
    let (month_start, today) = current_month_dates();
    let (forecast_start, forecast_end) = forecast_month_dates();

    let actuals_response = match app
        .aws
        .ce
        .get_cost_and_usage()
        .time_period(interval(month_start, today))
        .granularity(Granularity::Monthly)
        .metrics(Metric::UnblendedCost.as_str())
        .send()
        .await
    {
        Ok(response) => response,
        Err(err) => {
            return (BudgetInfo::default(), ServiceStatus::from_sdk_error(&err));
        }
    };

    let month_to_date_cost = actuals_response
        .results_by_time()
        .first()
        .map(|time_bucket| {
            metric_amount(
                time_bucket
                    .total()
                    .and_then(|total| total.get("UnblendedCost")),
            )
        })
        .unwrap_or(0.0);

    let forecast_response = app
        .aws
        .ce
        .get_cost_forecast()
        .time_period(interval(forecast_start, forecast_end))
        .metric(Metric::UnblendedCost)
        .granularity(Granularity::Monthly)
        .prediction_interval_level(80)
        .send()
        .await;

    let (forecast, forecast_low, forecast_high) = match forecast_response {
        Ok(response) => {
            let remaining = metric_amount(response.total());
            let range = response.forecast_results_by_time().first();
            let low = range
                .and_then(|forecast| forecast.prediction_interval_lower_bound())
                .and_then(|value| value.parse::<f64>().ok())
                .map(|value| month_to_date_cost + value);
            let high = range
                .and_then(|forecast| forecast.prediction_interval_upper_bound())
                .and_then(|value| value.parse::<f64>().ok())
                .map(|value| month_to_date_cost + value);

            (month_to_date_cost + remaining, low, high)
        }
        // Forecast is non-fatal: keep the real month-to-date actuals and let the
        // view show the range as unavailable rather than failing the whole view.
        Err(_) => (month_to_date_cost, None, None),
    };

    (
        BudgetInfo {
            monthly_budget: 100.0,
            month_to_date_cost,
            forecast,
            forecast_low,
            forecast_high,
        },
        ServiceStatus::Ok,
    )
}
