use crate::{app::App, models::target_group::TargetGroupInfo};

pub async fn fetch_target_groups(app: &App) -> Vec<TargetGroupInfo> {
    let resp = match app.aws.elb.describe_target_groups().send().await {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    let mut groups = Vec::new();

    for tg in resp.target_groups() {
        let arn = match tg.target_group_arn() {
            Some(a) => a.to_string(),
            None => continue,
        };

        let health = match app
            .aws
            .elb
            .describe_target_health()
            .target_group_arn(&arn)
            .send()
            .await
        {
            Ok(h) => h,
            Err(_) => continue,
        };

        let total = health.target_health_descriptions().len();
        let unhealthy = health
            .target_health_descriptions()
            .iter()
            .filter(|d| {
                d.target_health()
                    .and_then(|s| s.state())
                    .map(|s| s.as_str() != "healthy")
                    .unwrap_or(true)
            })
            .count();

        groups.push(TargetGroupInfo {
            arn,
            name: tg.target_group_name().unwrap_or("unknown").to_string(),
            protocol: tg
                .protocol()
                .map(|p| format!("{:?}", p))
                .unwrap_or_else(|| "unknown".into()),
            port: tg.port().unwrap_or(0),
            target_type: tg
                .target_type()
                .map(|t| format!("{:?}", t))
                .unwrap_or_else(|| "unknown".into()),
            attached_load_balancer_arns: tg
                .load_balancer_arns()
                .iter()
                .map(|arn| arn.to_string())
                .collect(),
            total_targets: total,
            unhealthy_targets: unhealthy,
        });
    }

    groups
}
