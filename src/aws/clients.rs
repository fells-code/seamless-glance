use aws_sdk_apigateway::Client as RestClient;
use aws_sdk_apigatewayv2::Client as V2Client;
use aws_sdk_cloudwatch::Client as CloudWatchClient;
use aws_sdk_costexplorer::Client as CeClient;
use aws_sdk_ec2::Client as Ec2Client;
use aws_sdk_ecs::Client as EcsClient;
use aws_sdk_elasticloadbalancingv2::Client as ElbClient;
use aws_sdk_lambda::Client as LambdaClient;
use aws_sdk_rds::Client as RdsClient;
use aws_sdk_secretsmanager::Client as SecretsClient;
use aws_sdk_sqs::Client as SqsClient;
use aws_sdk_sts::Client as StsClient;

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
        }
    }
}
