use crate::{
    app::App,
    models::{
        service_status::ServiceStatus,
        vpc::{VpcInfo, VpcSummary},
    },
};

pub async fn fetch_vpc_summary(app: &App) -> VpcSummary {
    let mut vpc_pages = app.aws.ec2.describe_vpcs().into_paginator().items().send();
    let mut vpc_count = 0u32;
    while let Some(item) = vpc_pages.next().await {
        match item {
            Ok(_) => vpc_count += 1,
            Err(err) => {
                return VpcSummary {
                    vpc_count: 0,
                    subnet_count: 0,
                    status: ServiceStatus::from_sdk_error(&err),
                };
            }
        }
    }

    // Subnets (count only). If subnets are denied but VPCs are allowed, keep
    // status Ok and show 0.
    let mut subnet_pages = app
        .aws
        .ec2
        .describe_subnets()
        .into_paginator()
        .items()
        .send();
    let mut subnet_count = 0u32;
    while let Some(item) = subnet_pages.next().await {
        match item {
            Ok(_) => subnet_count += 1,
            Err(_) => {
                return VpcSummary {
                    vpc_count,
                    subnet_count: 0,
                    status: ServiceStatus::Ok,
                };
            }
        }
    }

    VpcSummary {
        vpc_count,
        subnet_count,
        status: ServiceStatus::Ok,
    }
}

pub async fn fetch_vpcs(app: &App) -> (Vec<VpcInfo>, ServiceStatus) {
    let mut vpc_pages = app.aws.ec2.describe_vpcs().into_paginator().items().send();
    let mut vpcs = Vec::new();
    while let Some(item) = vpc_pages.next().await {
        match item {
            Ok(v) => vpcs.push(v),
            Err(err) => return (vec![], ServiceStatus::from_sdk_error(&err)),
        }
    }

    // Pull all subnets once (best effort), then count per VPC.
    let mut subnets = Vec::new();
    let mut subnet_pages = app
        .aws
        .ec2
        .describe_subnets()
        .into_paginator()
        .items()
        .send();
    while let Some(item) = subnet_pages.next().await {
        match item {
            Ok(s) => subnets.push(s),
            Err(_) => break,
        }
    }

    let mut out = Vec::new();

    for v in &vpcs {
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
