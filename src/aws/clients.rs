use aws_sdk_ec2::Client as Ec2Client;
use aws_sdk_lambda::Client as LambdaClient;
use aws_sdk_rds::Client as RdsClient;
// add more as needed

pub struct AwsClients {
    pub ec2: Ec2Client,
    pub rds: RdsClient,
    pub lambda: LambdaClient,
}

impl AwsClients {
    pub fn new(config: &aws_config::SdkConfig) -> Self {
        Self {
            ec2: Ec2Client::new(config),
            rds: RdsClient::new(config),
            lambda: LambdaClient::new(config),
        }
    }
}
