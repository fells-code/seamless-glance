use aws_types::region::Region;
use futures::future::join_all;
use std::future::Future;

use crate::models::service_status::ServiceStatus;

/// Run a per-region fetch across every region concurrently and combine the
/// results into one list plus a single status.
///
/// Status follows the "show partial availability honestly" rule: if any region
/// answered, the result is `Ok` (partial data beats none), otherwise a denial
/// wins over a generic failure so the operator sees the actionable reason.
/// `unavailable_message` is the fallback when nothing succeeded and no region
/// reported a reason.
pub async fn fetch_all_regions<T, F, Fut>(
    regions: &[Region],
    unavailable_message: &str,
    fetcher: F,
) -> (Vec<T>, ServiceStatus)
where
    T: Send + 'static,
    F: Fn(Region) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Vec<T>, ServiceStatus>> + Send,
{
    let results = join_all(regions.iter().cloned().map(fetcher)).await;

    let mut all = Vec::new();
    let mut any_success = false;
    let mut saw_access_denied = false;
    let mut first_unavailable: Option<String> = None;

    for result in results {
        match result {
            Ok(mut items) => {
                any_success = true;
                all.append(&mut items);
            }
            Err(ServiceStatus::AccessDenied) => saw_access_denied = true,
            Err(ServiceStatus::Unavailable(message)) => {
                if first_unavailable.is_none() {
                    first_unavailable = Some(message);
                }
            }
            Err(ServiceStatus::Ok) => {}
        }
    }

    let status = if any_success {
        ServiceStatus::Ok
    } else if saw_access_denied {
        ServiceStatus::AccessDenied
    } else {
        ServiceStatus::Unavailable(
            first_unavailable.unwrap_or_else(|| unavailable_message.to_string()),
        )
    };

    (all, status)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn regions(count: usize) -> Vec<Region> {
        (0..count)
            .map(|i| Region::new(format!("region-{i}")))
            .collect()
    }

    #[tokio::test]
    async fn a_single_reachable_region_still_reports_ok() {
        let (items, status) =
            fetch_all_regions(&regions(3), "nothing reachable", |region| async move {
                if region.as_ref() == "region-1" {
                    Ok(vec![region.as_ref().to_string()])
                } else {
                    Err(ServiceStatus::AccessDenied)
                }
            })
            .await;

        assert_eq!(items, vec!["region-1".to_string()]);
        assert!(
            matches!(status, ServiceStatus::Ok),
            "partial data must not read as a failure"
        );
    }

    #[tokio::test]
    async fn a_denial_outranks_a_generic_failure_when_nothing_succeeded() {
        let (items, status) =
            fetch_all_regions(&regions(2), "nothing reachable", |region| async move {
                if region.as_ref() == "region-0" {
                    Err::<Vec<String>, _>(ServiceStatus::Unavailable("throttled".into()))
                } else {
                    Err(ServiceStatus::AccessDenied)
                }
            })
            .await;

        assert!(items.is_empty());
        assert!(matches!(status, ServiceStatus::AccessDenied));
    }

    #[tokio::test]
    async fn the_first_reason_is_kept_when_every_region_failed() {
        let (_, status) =
            fetch_all_regions::<String, _, _>(&regions(2), "nothing reachable", |_| async move {
                Err(ServiceStatus::Unavailable("throttled".into()))
            })
            .await;

        match status {
            ServiceStatus::Unavailable(message) => assert_eq!(message, "throttled"),
            other => panic!("expected Unavailable, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn no_regions_falls_back_to_the_supplied_message() {
        let (_, status) = fetch_all_regions::<String, _, _>(
            &[],
            "nothing reachable",
            |_| async move { Ok(vec![]) },
        )
        .await;

        match status {
            ServiceStatus::Unavailable(message) => assert_eq!(message, "nothing reachable"),
            other => panic!("expected Unavailable, got {other:?}"),
        }
    }
}
