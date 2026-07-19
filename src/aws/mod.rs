/// Cap on in-flight per-resource describe calls (target-group health, SQS queue
/// attributes). These fan out one request per resource, so they run
/// concurrently but bounded, to stay fast without amplifying throttling.
pub const DESCRIBE_CONCURRENCY: usize = 10;

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
pub mod target_group;
pub mod vpc;
