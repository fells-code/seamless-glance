use crate::{
    app::App,
    models::{
        service_status::ServiceStatus,
        vpc::{VpcInfo, VpcSummary},
    },
};

pub async fn fetch_vpc_summary(app: &App) -> VpcSummary {
    let vpcs_resp = match app.aws.ec2.describe_vpcs().send().await {
        Ok(r) => r,
        Err(err) => {
            return VpcSummary {
                vpc_count: 0,
                subnet_count: 0,
                status: ServiceStatus::from_sdk_error(&err),
            };
        }
    };

    let vpc_count = vpcs_resp.vpcs().len() as u32;

    // Subnets (count only)
    let subnets_resp = match app.aws.ec2.describe_subnets().send().await {
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

pub async fn fetch_vpcs(app: &App) -> (Vec<VpcInfo>, ServiceStatus) {
    let vpcs_resp = match app.aws.ec2.describe_vpcs().send().await {
        Ok(r) => r,
        Err(err) => return (vec![], ServiceStatus::from_sdk_error(&err)),
    };

    // Pull all subnets once, then count per VPC (fast + simple for MVP)
    let subnets_resp = app.aws.ec2.describe_subnets().send().await.ok();
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
    (out, ServiceStatus::Ok)
}
