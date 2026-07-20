use std::time::Duration;

use aws_config::{retry::RetryConfig, timeout::TimeoutConfig, BehaviorVersion, Region, SdkConfig};
use aws_sdk_apigateway::Client as RestClient;
use aws_sdk_apigatewayv2::Client as V2Client;
use aws_sdk_cloudwatch::Client as CloudWatchClient;
use aws_sdk_costexplorer::Client as CeClient;
use aws_sdk_ec2::Client as Ec2Client;
use aws_sdk_ecs::Client as EcsClient;
use aws_sdk_elasticloadbalancingv2::Client as ElbClient;
use aws_sdk_lambda::Client as LambdaClient;
use aws_sdk_pricing::Client as PricingClient;
use aws_sdk_rds::Client as RdsClient;
use aws_sdk_secretsmanager::Client as SecretsClient;
use aws_sdk_sqs::Client as SqsClient;
use aws_sdk_sts::Client as StsClient;

/// Retry and timeout tuning for a dashboard that fans out across every enabled
/// region and issues a describe per resource. Adaptive retry adds client-side
/// rate limiting that backs off when AWS starts throttling, which matters most
/// on large accounts; the timeouts keep an unreachable or disabled region from
/// stalling a refresh.
const MAX_ATTEMPTS: u32 = 5;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const ATTEMPT_TIMEOUT: Duration = Duration::from_secs(15);
const OPERATION_TIMEOUT: Duration = Duration::from_secs(60);

/// Build an `SdkConfig` for the given region, optionally pinned to a named AWS
/// shared-config profile. Centralizing this keeps region and profile switching
/// on one code path so a profile selection survives region changes, and applies
/// the retry and timeout policy to every client the app builds.
fn retry_config() -> RetryConfig {
    RetryConfig::adaptive().with_max_attempts(MAX_ATTEMPTS)
}

fn timeout_config() -> TimeoutConfig {
    TimeoutConfig::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .operation_attempt_timeout(ATTEMPT_TIMEOUT)
        .operation_timeout(OPERATION_TIMEOUT)
        .build()
}

pub async fn build_sdk_config(region: Region, profile: Option<&str>) -> SdkConfig {
    let mut loader = aws_config::defaults(BehaviorVersion::v2026_01_12())
        .region(region)
        .retry_config(retry_config())
        .timeout_config(timeout_config());

    if let Some(name) = profile {
        loader = loader.profile_name(name);
    }

    loader.load().await
}

/// Build the client bundle for one region, honoring the active profile. Global
/// (multi-region) fetches call this per region; every service shares this one
/// definition so a region fan-out cannot drift from the central config policy.
pub async fn clients_for_region(region: &Region, profile: Option<&str>) -> AwsClients {
    let sdk_config = build_sdk_config(region.clone(), profile).await;
    AwsClients::new(&sdk_config)
}

#[derive(Clone)]
pub struct AwsClients {
    pub ec2: Ec2Client,
    pub rds: RdsClient,
    pub lambda: LambdaClient,
    pub ecs: EcsClient,
    pub cw: CloudWatchClient,
    pub apigw: RestClient,
    pub apigwv2: V2Client,
    pub elb: ElbClient,
    pub sm: SecretsClient,
    pub sts: StsClient,
    pub sqs: SqsClient,
    pub ce: CeClient,
    /// Pinned to the Price List API endpoint region rather than the active
    /// region, since prices for every region are served from there.
    pub pricing: PricingClient,
}

impl AwsClients {
    pub fn new(config: &aws_config::SdkConfig) -> Self {
        Self {
            ec2: Ec2Client::new(config),
            ecs: EcsClient::new(config),
            rds: RdsClient::new(config),
            lambda: LambdaClient::new(config),
            cw: CloudWatchClient::new(config),
            apigw: RestClient::new(config),
            apigwv2: V2Client::new(config),
            elb: ElbClient::new(config),
            sm: SecretsClient::new(config),
            sts: StsClient::new(config),
            sqs: SqsClient::new(config),
            ce: CeClient::new(config),
            pricing: PricingClient::from_conf(
                aws_sdk_pricing::config::Builder::from(config)
                    .region(Region::new(crate::aws::pricing::PRICING_ENDPOINT_REGION))
                    .build(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_config::retry::RetryMode;

    #[test]
    fn retry_is_adaptive_so_throttling_backs_off() {
        let retry = retry_config();
        assert_eq!(retry.mode(), RetryMode::Adaptive);
        assert_eq!(retry.max_attempts(), MAX_ATTEMPTS);
    }

    #[test]
    fn timeouts_bound_connect_attempt_and_overall_operation() {
        let timeouts = timeout_config();
        assert_eq!(timeouts.connect_timeout(), Some(CONNECT_TIMEOUT));
        assert_eq!(timeouts.operation_attempt_timeout(), Some(ATTEMPT_TIMEOUT));
        assert_eq!(timeouts.operation_timeout(), Some(OPERATION_TIMEOUT));
        assert!(
            ATTEMPT_TIMEOUT < OPERATION_TIMEOUT,
            "a single attempt must be able to finish inside the overall budget"
        );
    }
}
