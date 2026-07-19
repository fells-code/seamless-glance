use crate::{
    app::App,
    models::{security_group::SecurityGroupInfo, service_status::ServiceStatus},
};

const SENSITIVE_PORTS: [i32; 4] = [22, 3389, 5432, 3306];

fn permission_open_to_world(perm: &aws_sdk_ec2::types::IpPermission) -> bool {
    let ipv4_world = perm
        .ip_ranges()
        .iter()
        .any(|r| r.cidr_ip().map(|cidr| cidr == "0.0.0.0/0").unwrap_or(false));

    let ipv6_world = perm
        .ipv6_ranges()
        .iter()
        .any(|r| r.cidr_ipv6().map(|cidr| cidr == "::/0").unwrap_or(false));

    ipv4_world || ipv6_world
}

fn permission_includes_port(perm: &aws_sdk_ec2::types::IpPermission, port: i32) -> bool {
    if perm.ip_protocol().unwrap_or("") == "-1" {
        return true;
    }

    matches!(
        (perm.from_port(), perm.to_port()),
        (Some(from), Some(to)) if from <= port && port <= to
    )
}

pub async fn fetch_security_groups(app: &App) -> (Vec<SecurityGroupInfo>, ServiceStatus) {
    let resp = match app.aws.ec2.describe_security_groups().send().await {
        Ok(r) => r,
        Err(err) => return (vec![], ServiceStatus::from_sdk_error(&err)),
    };

    let security_groups = resp
        .security_groups()
        .iter()
        .map(|sg| {
            let inbound = sg.ip_permissions().len();
            let outbound = sg.ip_permissions_egress().len();

            let open_to_world = sg.ip_permissions().iter().any(permission_open_to_world);

            let mut sensitive_public_ports = SENSITIVE_PORTS
                .iter()
                .copied()
                .filter(|port| {
                    sg.ip_permissions().iter().any(|perm| {
                        permission_open_to_world(perm) && permission_includes_port(perm, *port)
                    })
                })
                .collect::<Vec<_>>();

            sensitive_public_ports.sort_unstable();
            sensitive_public_ports.dedup();

            SecurityGroupInfo {
                id: sg.group_id().unwrap_or_default().to_string(),
                name: sg.group_name().unwrap_or("unknown").to_string(),
                vpc_id: sg.vpc_id().unwrap_or("unknown").to_string(),
                inbound_rules: inbound,
                outbound_rules: outbound,
                open_to_world,
                sensitive_public_ports,
            }
        })
        .collect();

    (security_groups, ServiceStatus::Ok)
}
