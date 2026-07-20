//! On-disk cache of public list prices.
//!
//! Unlike the cost cache this holds no account data: list prices are the same
//! for everyone, so the file is not scoped by profile and does not need the
//! scope re-check that `cache::cost` performs. Prices change rarely, so the TTL
//! is long.

use std::{fs, path::PathBuf, time::Duration};

use serde::{Deserialize, Serialize};

use crate::aws::pricing::{PriceBook, PriceKey};

const CACHE_TTL: Duration = Duration::from_secs(60 * 60 * 24 * 30);
const CACHE_FILE: &str = "pricing.json";

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PricingCache {
    /// A `None` rate is a key that was looked up and had no published price.
    #[serde(default)]
    pub hourly_usd: Vec<(PriceKey, Option<f64>)>,
}

fn cache_path() -> Option<PathBuf> {
    Some(
        dirs::home_dir()?
            .join(".seamless-glance")
            .join("cache")
            .join(CACHE_FILE),
    )
}

pub fn load_if_fresh() -> Option<PriceBook> {
    let path = cache_path()?;
    let age = fs::metadata(&path).ok()?.modified().ok()?.elapsed().ok()?;

    if age > CACHE_TTL {
        return None;
    }

    let cache: PricingCache = serde_json::from_str(&fs::read_to_string(&path).ok()?).ok()?;

    let mut book = PriceBook::default();
    for (key, hourly) in cache.hourly_usd {
        book.insert(key, hourly);
    }

    Some(book)
}

pub fn save(book: &PriceBook) {
    let Some(path) = cache_path() else {
        return;
    };

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let cache = PricingCache {
        hourly_usd: book.entries(),
    };

    if let Ok(json) = serde_json::to_string_pretty(&cache) {
        let _ = fs::write(path, json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::pricing::LoadBalancerKind;

    #[test]
    fn a_book_survives_a_round_trip_through_the_cache_shape() {
        let ec2 = PriceKey::Ec2Instance {
            region: "us-east-1".into(),
            instance_type: "t3.medium".into(),
        };
        let lb = PriceKey::LoadBalancer {
            region: "us-east-1".into(),
            kind: LoadBalancerKind::Application,
        };

        let mut book = PriceBook::default();
        book.insert(ec2.clone(), Some(0.0416));
        book.insert(lb.clone(), None);

        let json = serde_json::to_string(&PricingCache {
            hourly_usd: book.entries(),
        })
        .expect("serializes");
        let restored: PricingCache = serde_json::from_str(&json).expect("deserializes");

        let mut rebuilt = PriceBook::default();
        for (key, hourly) in restored.hourly_usd {
            rebuilt.insert(key, hourly);
        }

        assert!(rebuilt.estimate(&ec2).is_some());
        // Still remembered as looked-up-and-unpriced, not as never looked up.
        assert!(rebuilt.contains(&lb));
        assert_eq!(rebuilt.estimate(&lb), None);
    }
}
