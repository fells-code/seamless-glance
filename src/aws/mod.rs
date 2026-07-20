use std::future::Future;

use futures::StreamExt;

/// Cap on in-flight per-resource calls (target-group health, SQS queue
/// attributes, per-resource tag lookups). These fan out one request per
/// resource, so they run concurrently but bounded, to stay fast without
/// amplifying throttling.
pub const DESCRIBE_CONCURRENCY: usize = 10;

/// Run `call` over every item with at most [`DESCRIBE_CONCURRENCY`] in flight,
/// preserving input order.
pub async fn bounded_map<T, F, Fut, R>(items: impl IntoIterator<Item = T>, call: F) -> Vec<R>
where
    F: Fn(T) -> Fut,
    Fut: Future<Output = R>,
{
    futures::stream::iter(items)
        .map(call)
        .buffered(DESCRIBE_CONCURRENCY)
        .collect()
        .await
}

pub mod account;
pub mod apigateway;
pub mod clients;
pub mod cloudwatch;
pub mod cost;
pub mod ec2;
pub mod ecs;
pub mod elb;
pub mod lambda;
pub mod profiles;
pub mod rds;
pub mod regions;
pub mod secrets;
pub mod security_group;
pub mod sqs;
pub mod tags;
pub mod target_group;
pub mod vpc;
