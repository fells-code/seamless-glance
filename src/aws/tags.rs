//! Normalizing AWS SDK tag shapes into the shared [`Tags`] model.
//!
//! Each service crate generates its own `Tag` struct, so there is no common
//! type to borrow. These helpers cover the two shapes the SDKs actually return:
//! a list of key/value structs (EC2, RDS, Secrets Manager, ECS) and a plain map
//! (API Gateway).
//!
//! Some services do not return tags on their list or describe response at all
//! and need a separate call per resource. Those lookups can fail on their own,
//! which is what [`Tags::Unavailable`] records.

use std::collections::HashMap;
use std::future::Future;

use crate::aws::bounded_map;
use crate::models::tags::Tags;

/// Collect a service's `Vec<Tag>` into [`Tags`].
///
/// Callers pass an iterator of already-extracted key/value pairs because the
/// `Tag` types are per-crate and share no trait.
pub fn from_pairs<'a>(pairs: impl Iterator<Item = (Option<&'a str>, Option<&'a str>)>) -> Tags {
    Tags::loaded(pairs.filter_map(|(key, value)| Some((key?, value.unwrap_or_default()))))
}

/// Collect the map shape returned by the API Gateway crates.
pub fn from_map(map: Option<&HashMap<String, String>>) -> Tags {
    match map {
        Some(entries) => Tags::loaded(entries.iter().map(|(k, v)| (k.as_str(), v.as_str()))),
        // The field is omitted rather than empty when the resource has no tags.
        None => Tags::empty(),
    }
}

/// Look tags up one resource at a time, bounded, for services that do not
/// return them inline.
///
/// A lookup that fails yields [`Tags::Unavailable`] for that resource rather
/// than an empty map, so a throttled call is never mistaken for an untagged
/// resource. The rest of the fetch is unaffected: missing tags degrade one
/// resource's ownership data, they do not fail the service.
pub async fn lookup_each<T, F, Fut>(items: impl IntoIterator<Item = T>, lookup: F) -> Vec<Tags>
where
    F: Fn(T) -> Fut,
    Fut: Future<Output = Option<Tags>>,
{
    bounded_map(items, lookup)
        .await
        .into_iter()
        .map(|tags| tags.unwrap_or(Tags::Unavailable))
        .collect()
}

/// Max ARNs accepted by one ELBv2 `DescribeTags` call.
const ELB_DESCRIBE_TAGS_CHUNK: usize = 20;

/// Tags for ELBv2 load balancers and target groups, keyed by ARN.
///
/// `DescribeTags` takes up to 20 ARNs per call, so this batches rather than
/// fanning out per resource. A chunk that fails leaves its ARNs absent from the
/// map, and callers resolve a missing ARN to [`Tags::Unavailable`].
pub async fn for_elb_arns(
    client: &aws_sdk_elasticloadbalancingv2::Client,
    arns: &[String],
) -> HashMap<String, Tags> {
    let chunks = arns
        .chunks(ELB_DESCRIBE_TAGS_CHUNK)
        .map(|chunk| chunk.to_vec())
        .collect::<Vec<_>>();

    let responses = bounded_map(chunks, |chunk| async move {
        client
            .describe_tags()
            .set_resource_arns(Some(chunk))
            .send()
            .await
            .ok()
    })
    .await;

    responses
        .into_iter()
        .flatten()
        .flat_map(|resp| {
            resp.tag_descriptions()
                .iter()
                .filter_map(|desc| {
                    let arn = desc.resource_arn()?.to_string();
                    let tags = from_pairs(desc.tags().iter().map(|t| (t.key(), t.value())));
                    Some((arn, tags))
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tag_without_a_key_is_dropped() {
        let tags = from_pairs([(Some("Name"), Some("web")), (None, Some("orphan"))].into_iter());

        assert_eq!(tags.value("Name"), Some("web"));
        assert_eq!(tags.value("orphan"), None);
    }

    #[test]
    fn a_key_with_no_value_is_kept_as_empty() {
        let tags = from_pairs([(Some("Owner"), None)].into_iter());

        // Present-but-blank, which `Tags::value` then treats as absent.
        assert_eq!(tags.get("Owner"), Some(""));
        assert_eq!(tags.value("Owner"), None);
    }

    #[test]
    fn an_absent_tag_map_means_untagged_not_unreadable() {
        assert!(from_map(None).is_available());
        assert_eq!(from_map(None).missing(&["Client"]), Some(vec!["Client"]));

        let map = HashMap::from([("Client".to_string(), "acme".to_string())]);
        assert_eq!(from_map(Some(&map)).value("Client"), Some("acme"));
    }
}
