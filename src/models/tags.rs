//! Resource tags, carried uniformly across service models.
//!
//! Tags drive ownership attribution and the per-client focus filter, so "this
//! resource has no tags" and "we could not read this resource's tags" have to
//! stay distinguishable. Collapsing them would let a throttled tag lookup drop
//! a resource out of a focus filter as if it were genuinely untagged.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Tags {
    /// Tags were read successfully. An empty map means the resource really is
    /// untagged.
    Loaded(BTreeMap<String, String>),
    /// The tag lookup did not run or did not succeed, so nothing can be
    /// concluded about this resource's tags.
    #[default]
    Unavailable,
}

impl Tags {
    pub fn loaded<K, V>(pairs: impl IntoIterator<Item = (K, V)>) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        Tags::Loaded(
            pairs
                .into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
        )
    }

    pub fn empty() -> Self {
        Tags::Loaded(BTreeMap::new())
    }

    pub fn is_available(&self) -> bool {
        matches!(self, Tags::Loaded(_))
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        match self {
            Tags::Loaded(entries) => entries.get(key).map(String::as_str),
            Tags::Unavailable => None,
        }
    }

    /// The tag value for `key`, treating a whitespace-only value as absent.
    ///
    /// AWS accepts a tag whose value is blank, which carries no more ownership
    /// information than omitting the tag entirely.
    pub fn value(&self, key: &str) -> Option<&str> {
        self.get(key)
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }

    /// Which of `required` are missing, or `None` when tags could not be read
    /// and coverage therefore cannot be judged.
    pub fn missing(&self, required: &[&'static str]) -> Option<Vec<&'static str>> {
        if !self.is_available() {
            return None;
        }

        Some(
            required
                .iter()
                .copied()
                .filter(|key| self.value(key).is_none())
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blank_value_reads_as_absent() {
        let tags = Tags::loaded([("Owner", "   "), ("Environment", "prod")]);

        assert_eq!(tags.value("Owner"), None);
        assert_eq!(tags.value("Environment"), Some("prod"));
        // get() is the raw view and still reports what AWS returned.
        assert_eq!(tags.get("Owner"), Some("   "));
    }

    #[test]
    fn an_untagged_resource_is_not_an_unreadable_one() {
        assert_eq!(Tags::empty().missing(&["Name"]), Some(vec!["Name"]));
        assert_eq!(Tags::Unavailable.missing(&["Name"]), None);

        assert!(Tags::empty().is_available());
        assert!(!Tags::Unavailable.is_available());
    }

    #[test]
    fn missing_reports_only_absent_keys() {
        let tags = Tags::loaded([("Name", "web-1"), ("Owner", "")]);

        assert_eq!(
            tags.missing(&["Name", "Owner", "Environment"]),
            Some(vec!["Owner", "Environment"])
        );
    }

    #[test]
    fn unavailable_tags_yield_nothing() {
        assert_eq!(Tags::Unavailable.get("Name"), None);
        assert_eq!(Tags::Unavailable.value("Name"), None);
        assert_eq!(Tags::loaded([("a", "b")]).value("a"), Some("b"));
    }
}
