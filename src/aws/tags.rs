//! Normalizing AWS SDK tag shapes into the shared [`Tags`] model.
//!
//! Each service crate generates its own `Tag` struct, so there is no common
//! type to borrow. These helpers cover the two shapes the SDKs actually return:
//! a list of key/value structs (EC2, RDS, Secrets Manager, ECS) and a plain map
//! (API Gateway).

use std::collections::HashMap;

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
