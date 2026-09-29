use chrono::{DateTime, Utc};

use crate::{
    app::App,
    aws::{bounded_map, tags},
    models::{
        ecr::{EcrImageStats, EcrRepositoryInfo},
        service_status::ServiceStatus,
        tags::Tags,
    },
};
use aws_sdk_ecr::types::Repository;

fn to_utc(dt: &aws_smithy_types::DateTime) -> Option<DateTime<Utc>> {
    DateTime::<Utc>::from_timestamp(dt.secs(), dt.subsec_nanos())
}

/// Page through every image in the repository and fold it into one rollup.
/// Returns `None` if any page fails, since partial counts would understate
/// buildup and misreport staleness.
async fn image_stats(app: &App, repository: &str) -> Option<EcrImageStats> {
    let mut pages = app
        .aws
        .ecr
        .describe_images()
        .repository_name(repository)
        .into_paginator()
        .items()
        .send();

    let mut stats = EcrImageStats::default();

    while let Some(item) = pages.next().await {
        let image = item.ok()?;

        stats.count += 1;
        if image.image_tags().is_empty() {
            stats.untagged += 1;
        }
        stats.total_bytes += image.image_size_in_bytes().unwrap_or(0);
        stats.last_pushed = stats
            .last_pushed
            .max(image.image_pushed_at().and_then(to_utc));
        stats.last_pulled = stats
            .last_pulled
            .max(image.last_recorded_pull_time().and_then(to_utc));
    }

    Some(stats)
}

async fn has_lifecycle_policy(app: &App, repository: &str) -> Option<bool> {
    match app
        .aws
        .ecr
        .get_lifecycle_policy()
        .repository_name(repository)
        .send()
        .await
    {
        Ok(_) => Some(true),
        Err(err)
            if err
                .as_service_error()
                .is_some_and(|e| e.is_lifecycle_policy_not_found_exception()) =>
        {
            Some(false)
        }
        Err(_) => None,
    }
}

async fn repository_tags(app: &App, arn: Option<&str>) -> Tags {
    let Some(arn) = arn else {
        return Tags::Unavailable;
    };

    match app
        .aws
        .ecr
        .list_tags_for_resource()
        .resource_arn(arn)
        .send()
        .await
    {
        Ok(resp) => tags::from_pairs(resp.tags().iter().map(|t| (Some(t.key()), Some(t.value())))),
        Err(_) => Tags::Unavailable,
    }
}

pub async fn fetch_ecr_repositories(app: &App) -> (Vec<EcrRepositoryInfo>, ServiceStatus) {
    let mut pages = app
        .aws
        .ecr
        .describe_repositories()
        .into_paginator()
        .items()
        .send();

    let mut repositories: Vec<Repository> = Vec::new();
    while let Some(item) = pages.next().await {
        match item {
            Ok(repository) => repositories.push(repository),
            Err(err) => return (vec![], ServiceStatus::from_sdk_error(&err)),
        }
    }

    // Images, lifecycle policy, and tags are each a separate call per
    // repository. They run one after another inside the bounded fan-out so the
    // in-flight cap still holds.
    let repositories = bounded_map(repositories, |repository| async move {
        let name = repository
            .repository_name()
            .unwrap_or("unknown")
            .to_string();

        let images = image_stats(app, &name).await;
        let has_lifecycle_policy = has_lifecycle_policy(app, &name).await;
        let tags = repository_tags(app, repository.repository_arn()).await;

        EcrRepositoryInfo {
            registry_id: repository.registry_id().unwrap_or_default().to_string(),
            name,
            has_lifecycle_policy,
            images,
            tags,
        }
    })
    .await;

    (repositories, ServiceStatus::Ok)
}
