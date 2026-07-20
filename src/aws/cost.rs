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

/// Month to date: the first of `today`'s month up to, but not including, today.
///
/// Cost Explorer treats the end of an interval as exclusive and rejects an
/// interval that starts and ends on the same day. On the first of the month
/// those coincide, so the window is widened to cover today, which is the only
/// spend there is to report at that point.
fn month_to_date_from(today: NaiveDate) -> (NaiveDate, NaiveDate) {
    let start = first_day_of_month(today);

    if start == today {
        return (start, start.succ_opt().unwrap_or(today));
    }

    (start, today)
}

fn current_month_dates() -> (NaiveDate, NaiveDate) {
    month_to_date_from(today_exclusive())
}

/// The rest of `today`'s month: today up to the first of next month.
fn remainder_of_month_from(today: NaiveDate) -> (NaiveDate, NaiveDate) {
    (today, first_day_of_next_month(today))
}

fn forecast_month_dates() -> (NaiveDate, NaiveDate) {
    remainder_of_month_from(today_exclusive())
}

/// Six calendar months ending today: the first of the month five months back,
/// up to today. The window starts at a month boundary so the first bucket is a
/// whole month rather than a partial one.
fn trailing_six_months_from(today: NaiveDate) -> (NaiveDate, NaiveDate) {
    let start = first_day_of_month(
        today
            .checked_sub_months(Months::new(5))
            .expect("valid six month window"),
    );

    (start, today)
}

fn trailing_six_month_dates() -> (NaiveDate, NaiveDate) {
    trailing_six_months_from(today_exclusive())
}

/// Short month names for the trailing six months, oldest first, so the labels
/// line up with the buckets `trailing_six_months_from` asks for.
fn six_month_labels_from(today: NaiveDate) -> Vec<String> {
    (0..6)
        .map(|i| {
            today
                .checked_sub_months(Months::new((5 - i) as u32))
                .expect("valid month")
                .format("%b")
                .to_string()
        })
        .collect()
}

pub fn last_6_month_labels() -> Vec<String> {
    six_month_labels_from(today_exclusive())
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

#[cfg(test)]
mod tests {
    use super::*;
    use aws_sdk_costexplorer::types::MetricValue;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).expect("valid test date")
    }

    #[test]
    fn month_to_date_runs_from_the_first_up_to_today() {
        let (start, end) = month_to_date_from(date(2026, 7, 20));

        assert_eq!(start, date(2026, 7, 1));
        // Exclusive end, so today's partial spend is not requested.
        assert_eq!(end, date(2026, 7, 20));
    }

    /// Cost Explorer rejects an interval whose start and end are the same day
    /// ("Start date (and hour) should be before end date (and hour)"), so on
    /// the first of the month the window has to cover today instead of
    /// collapsing. Left as-is this made every cost view report unavailable on
    /// the first of each month.
    #[test]
    fn the_first_of_the_month_is_never_an_empty_window() {
        let (start, end) = month_to_date_from(date(2026, 7, 1));

        assert_eq!(start, date(2026, 7, 1));
        assert_eq!(end, date(2026, 7, 2));
        assert!(start < end, "Cost Explorer requires a non-empty interval");
    }

    /// The same collapse would happen on the first of January, where the widened
    /// end must stay inside the new year.
    #[test]
    fn the_first_of_january_widens_within_the_new_year() {
        let (start, end) = month_to_date_from(date(2026, 1, 1));

        assert_eq!(start, date(2026, 1, 1));
        assert_eq!(end, date(2026, 1, 2));
    }

    #[test]
    fn every_day_of_a_month_yields_a_usable_interval() {
        for day in 1..=31 {
            let today = date(2026, 7, day);
            let (start, end) = month_to_date_from(today);

            assert!(start < end, "empty interval for {today}");
        }
    }

    #[test]
    fn the_forecast_window_runs_to_the_start_of_next_month() {
        let (start, end) = remainder_of_month_from(date(2026, 7, 20));

        assert_eq!(start, date(2026, 7, 20));
        assert_eq!(end, date(2026, 8, 1));
    }

    /// December has to roll the year, which plain month arithmetic gets wrong.
    #[test]
    fn december_rolls_into_the_next_year() {
        assert_eq!(first_day_of_next_month(date(2026, 12, 9)), date(2027, 1, 1));
        assert_eq!(
            remainder_of_month_from(date(2026, 12, 31)).1,
            date(2027, 1, 1)
        );
    }

    #[test]
    fn the_first_of_a_month_is_found_from_any_day_in_it() {
        assert_eq!(first_day_of_month(date(2026, 7, 20)), date(2026, 7, 1));
        assert_eq!(first_day_of_month(date(2026, 7, 1)), date(2026, 7, 1));
        // A leap day is still just a day in February.
        assert_eq!(first_day_of_month(date(2024, 2, 29)), date(2024, 2, 1));
    }

    #[test]
    fn the_six_month_window_starts_on_a_month_boundary() {
        let (start, end) = trailing_six_months_from(date(2026, 7, 20));

        // Five months back from July is February, snapped to the 1st so the
        // oldest bucket is a whole month.
        assert_eq!(start, date(2026, 2, 1));
        assert_eq!(end, date(2026, 7, 20));
    }

    #[test]
    fn the_six_month_window_crosses_a_year_boundary() {
        let (start, end) = trailing_six_months_from(date(2026, 3, 15));

        assert_eq!(start, date(2025, 10, 1));
        assert_eq!(end, date(2026, 3, 15));
    }

    /// Subtracting months from a 31-day date lands on a shorter month. The
    /// window is snapped to the 1st afterwards, so the clamp cannot shift which
    /// month the window starts in.
    #[test]
    fn a_month_end_date_still_starts_the_window_on_the_first() {
        let (start, _) = trailing_six_months_from(date(2026, 7, 31));

        assert_eq!(start, date(2026, 2, 1));
    }

    #[test]
    fn labels_run_oldest_first_and_match_the_window() {
        let labels = six_month_labels_from(date(2026, 7, 20));

        assert_eq!(labels, vec!["Feb", "Mar", "Apr", "May", "Jun", "Jul"]);
        assert_eq!(labels.len(), 6);
    }

    #[test]
    fn labels_cross_a_year_boundary_in_order() {
        let labels = six_month_labels_from(date(2026, 2, 5));

        assert_eq!(labels, vec!["Sep", "Oct", "Nov", "Dec", "Jan", "Feb"]);
    }

    /// The label list and the requested window have to agree, or the chart
    /// axis is offset from the data it plots.
    #[test]
    fn the_first_label_is_the_month_the_window_starts_in() {
        for today in [date(2026, 7, 20), date(2026, 1, 3), date(2026, 12, 31)] {
            let (start, _) = trailing_six_months_from(today);
            let labels = six_month_labels_from(today);

            assert_eq!(
                labels[0],
                start.format("%b").to_string(),
                "window and labels disagree for {today}"
            );
        }
    }

    #[test]
    fn an_interval_is_formatted_as_cost_explorer_expects() {
        let (start, end) = interval_strings(date(2026, 2, 1), date(2026, 7, 20));

        assert_eq!(start, "2026-02-01");
        assert_eq!(end, "2026-07-20");
    }

    #[test]
    fn a_metric_amount_is_parsed_from_its_string() {
        let value = MetricValue::builder().amount("123.45").unit("USD").build();

        assert_eq!(metric_amount(Some(&value)), 123.45);
        assert_eq!(metric_unit(Some(&value)), "USD");
    }

    /// Cost Explorer omits metrics for a period with no spend. That is zero,
    /// not an error, and must not poison the total.
    #[test]
    fn a_missing_metric_reads_as_zero() {
        assert_eq!(metric_amount(None), 0.0);
        assert_eq!(metric_unit(None), "");

        let empty = MetricValue::builder().build();
        assert_eq!(metric_amount(Some(&empty)), 0.0);
    }

    #[test]
    fn an_unparseable_amount_reads_as_zero_rather_than_panicking() {
        let broken = MetricValue::builder().amount("not-a-number").build();

        assert_eq!(metric_amount(Some(&broken)), 0.0);
    }
}
