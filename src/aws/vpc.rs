use crate::{app::App, models::service_status::ServiceStatus};
use aws_sdk_ec2::Client;

#[derive(Debug, Clone)]
pub struct VpcSummary {
    pub vpc_count: u32,
    pub subnet_count: u32,
    pub status: ServiceStatus,
}

#[derive(Debug, Clone)]
pub struct VpcInfo {
    pub vpc_id: String,
    pub cidr: String,
    pub state: String,
    pub is_default: bool,
    pub subnet_count: u32,
}

pub async fn fetch_vpc_summary(app: &App) -> VpcSummary {
    let config = aws_config::defaults(aws_config::BehaviorVersion::v2025_08_07())
        .region(app.current_region().clone())
        .load()
        .await;
    let ec2 = Client::new(&config);

    // VPCs
    let vpcs_resp = match ec2.describe_vpcs().send().await {
        Ok(r) => r,
        Err(err) => {
            let msg = err.to_string();
            return if msg.contains("AccessDenied") {
                VpcSummary {
                    vpc_count: 0,
                    subnet_count: 0,
                    status: ServiceStatus::AccessDenied,
                }
            } else {
                VpcSummary {
                    vpc_count: 0,
                    subnet_count: 0,
                    status: ServiceStatus::Unavailable(msg),
                }
            };
        }
    };

    let vpc_count = vpcs_resp.vpcs().len() as u32;

    // Subnets (count only)
    let subnets_resp = match ec2.describe_subnets().send().await {
        Ok(r) => r,
        Err(_) => {
            // If subnets are denied but VPCs are allowed, keep status Ok and show 0.
            // If you'd rather surface a warning, model subnet_status separately.
            return VpcSummary {
                vpc_count,
                subnet_count: 0,
                status: ServiceStatus::Ok,
            };
        }
    };

    let subnet_count = subnets_resp.subnets().len() as u32;

    VpcSummary {
        vpc_count,
        subnet_count,
        status: ServiceStatus::Ok,
    }
}

pub async fn fetch_vpcs(app: &App) -> Vec<VpcInfo> {
    let config = aws_config::defaults(aws_config::BehaviorVersion::v2025_08_07())
        .region(app.current_region().clone())
        .load()
        .await;
    let ec2 = Client::new(&config);

    let vpcs_resp = match ec2.describe_vpcs().send().await {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    // Pull all subnets once, then count per VPC (fast + simple for MVP)
    let subnets_resp = ec2.describe_subnets().send().await.ok();
    let subnets = subnets_resp.as_ref().map(|r| r.subnets()).unwrap_or(&[]);

    let mut out = Vec::new();

    for v in vpcs_resp.vpcs() {
        let vpc_id = v.vpc_id().unwrap_or("-").to_string();

        let cidr = v.cidr_block().unwrap_or("-").to_string();

        let state = v
            .state()
            .map(|s| format!("{:?}", s))
            .unwrap_or_else(|| "-".into());

        let is_default = v.is_default().unwrap_or(false);

        let subnet_count = subnets
            .iter()
            .filter(|s| s.vpc_id().unwrap_or("") == vpc_id)
            .count() as u32;

        out.push(VpcInfo {
            vpc_id,
            cidr,
            state,
            is_default,
            subnet_count,
        });
    }

    // Sort defaults first, then by id
    out.sort_by(|a, b| {
        b.is_default
            .cmp(&a.is_default)
            .then(a.vpc_id.cmp(&b.vpc_id))
    });
    out
}
