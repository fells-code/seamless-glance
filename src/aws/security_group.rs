use crate::{app::App, models::security_group::SecurityGroupInfo};

pub async fn fetch_security_groups(app: &App) -> Vec<SecurityGroupInfo> {
    let resp = match app.aws.ec2.describe_security_groups().send().await {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    resp.security_groups()
        .iter()
        .map(|sg| {
            let inbound = sg.ip_permissions().len();
            let outbound = sg.ip_permissions_egress().len();

            let open_to_world = sg.ip_permissions().iter().any(|perm| {
                perm.ip_ranges()
                    .iter()
                    .any(|r| r.cidr_ip().map(|cidr| cidr == "0.0.0.0/0").unwrap_or(false))
            });

            SecurityGroupInfo {
                id: sg.group_id().unwrap_or_default().to_string(),
                name: sg.group_name().unwrap_or("unknown").to_string(),
                vpc_id: sg.vpc_id().unwrap_or("unknown").to_string(),
                inbound_rules: inbound,
                outbound_rules: outbound,
                open_to_world,
            }
        })
        .collect()
}
