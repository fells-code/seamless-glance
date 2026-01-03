use aws_sdk_secretsmanager::Client;

pub async fn fetch_secrets() -> Vec<SecretInfo> {
    let config = aws_config::defaults(aws_config::BehaviorVersion::v2025_08_07())
        .region(app.current_region().clone())
        .load()
        .await;
    let client = Client::new(&config);

    let resp = match client.list_secrets().send().await {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    resp.secret_list()
        .iter()
        .map(|s| SecretInfo {
            name: s.name().unwrap_or("").into(),
            last_rotated: s.last_rotated_date().map(|d| d.to_string()),
            rotation_enabled: s.rotation_enabled().unwrap_or(false),
        })
        .collect()
}
