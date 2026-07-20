//! On-demand list prices for the resource types waste findings report on.
//!
//! The Price List API is only served from a few regions and returns prices for
//! every region, so the client is pinned to one endpoint and the region of
//! interest is a filter rather than the client's region.
//!
//! Prices are public list prices and change rarely, so a lookup is cached on
//! disk for a month. Nothing here reads account-specific data.

use std::collections::HashMap;

use aws_sdk_pricing::types::{Filter, FilterType};
use aws_sdk_pricing::Client;
use serde::{Deserialize, Serialize};

use crate::models::cost_estimate::{CostBasis, CostEstimate};

/// The Price List API has endpoints in only a few regions. This one is always
/// available and serves prices for every region.
pub const PRICING_ENDPOINT_REGION: &str = "us-east-1";

/// What a price was looked up for. Serialized as the on-disk cache key.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PriceKey {
    Ec2Instance {
        region: String,
        instance_type: String,
    },
    LoadBalancer {
        region: String,
        kind: LoadBalancerKind,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LoadBalancerKind {
    Application,
    Network,
}

impl LoadBalancerKind {
    /// Map the type string carried on `LoadBalancerInfo`, which comes from the
    /// ELBv2 enum's debug form.
    pub fn from_lb_type(lb_type: &str) -> Option<Self> {
        match lb_type.to_ascii_lowercase().as_str() {
            t if t.contains("application") => Some(LoadBalancerKind::Application),
            t if t.contains("network") => Some(LoadBalancerKind::Network),
            // Gateway and the retired Classic balancers price differently.
            _ => None,
        }
    }

    fn product_family(&self) -> &'static str {
        match self {
            LoadBalancerKind::Application => "Load Balancer-Application",
            LoadBalancerKind::Network => "Load Balancer-Network",
        }
    }
}

/// Hourly on-demand list prices, keyed by what they were looked up for.
///
/// A key that is absent was never looked up; a key mapped to `None` was looked
/// up and had no published price, so it is not retried within a session.
#[derive(Debug, Clone, Default)]
pub struct PriceBook {
    hourly_usd: HashMap<PriceKey, Option<f64>>,
}

impl PriceBook {
    pub fn estimate(&self, key: &PriceKey) -> Option<CostEstimate> {
        let hourly = (*self.hourly_usd.get(key)?)?;

        Some(CostEstimate::from_hourly(
            hourly,
            CostBasis::OnDemandListPrice,
        ))
    }

    pub fn insert(&mut self, key: PriceKey, hourly_usd: Option<f64>) {
        self.hourly_usd.insert(key, hourly_usd);
    }

    pub fn contains(&self, key: &PriceKey) -> bool {
        self.hourly_usd.contains_key(key)
    }

    /// Flattened for persistence. A `HashMap` with a struct key does not
    /// survive JSON, which keys objects by string only.
    pub fn entries(&self) -> Vec<(PriceKey, Option<f64>)> {
        self.hourly_usd
            .iter()
            .map(|(key, hourly)| (key.clone(), *hourly))
            .collect()
    }
}

fn term_match(field: &str, value: &str) -> Option<Filter> {
    Filter::builder()
        .field(field)
        .value(value)
        .r#type(FilterType::TermMatch)
        .build()
        .ok()
}

/// Pull the on-demand USD rate out of a Price List product document.
///
/// The shape is `terms.OnDemand.<offer>.priceDimensions.<rate>.pricePerUnit.USD`,
/// where both levels are keyed by opaque codes, so this walks the first entry
/// at each rather than naming them. A product with tiered dimensions would have
/// more than one rate; the resource types here are billed at a flat hourly rate.
fn on_demand_usd(document: &str) -> Option<f64> {
    let parsed: serde_json::Value = serde_json::from_str(document).ok()?;
    let offers = parsed.get("terms")?.get("OnDemand")?.as_object()?;
    let dimensions = offers
        .values()
        .next()?
        .get("priceDimensions")?
        .as_object()?;

    dimensions
        .values()
        .next()?
        .get("pricePerUnit")?
        .get("USD")?
        .as_str()?
        .parse()
        .ok()
}

async fn lookup(client: &Client, key: &PriceKey) -> Option<f64> {
    let request = match key {
        PriceKey::Ec2Instance {
            region,
            instance_type,
        } => client
            .get_products()
            .service_code("AmazonEC2")
            .filters(term_match("regionCode", region)?)
            .filters(term_match("instanceType", instance_type)?)
            // Without these the query matches many SKUs that differ only by
            // licensing and tenancy, and the first hit would be arbitrary.
            .filters(term_match("operatingSystem", "Linux")?)
            .filters(term_match("tenancy", "Shared")?)
            .filters(term_match("preInstalledSw", "NA")?)
            .filters(term_match("capacitystatus", "Used")?),
        PriceKey::LoadBalancer { region, kind } => client
            .get_products()
            .service_code("AWSELB")
            .filters(term_match("regionCode", region)?)
            .filters(term_match("productFamily", kind.product_family())?)
            .filters(term_match("usagetype", "LoadBalancerUsage")?),
    };

    let response = request.max_results(1).send().await.ok()?;

    on_demand_usd(response.price_list().first()?)
}

/// Look up every key not already priced, leaving what is already known alone.
///
/// A lookup that fails is recorded as "no published price" rather than retried,
/// so one unavailable SKU cannot make every refresh re-request it.
pub async fn fill(client: &Client, book: &mut PriceBook, keys: impl IntoIterator<Item = PriceKey>) {
    let wanted = keys
        .into_iter()
        .filter(|key| !book.contains(key))
        .collect::<Vec<_>>();

    for key in wanted {
        let hourly = lookup(client, &key).await;
        book.insert(key, hourly);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EC2_DOCUMENT: &str = r#"{
        "product": {"sku": "ABC"},
        "terms": {"OnDemand": {"ABC.JRTCKXETXF": {"priceDimensions": {
            "ABC.JRTCKXETXF.6YS6EN2CT7": {"unit": "Hrs", "pricePerUnit": {"USD": "0.0416000000"}}
        }}}}
    }"#;

    #[test]
    fn the_on_demand_rate_is_read_from_opaque_keys() {
        assert_eq!(on_demand_usd(EC2_DOCUMENT), Some(0.0416));
    }

    #[test]
    fn a_document_without_on_demand_terms_yields_nothing() {
        let reserved_only = r#"{"terms": {"Reserved": {}}}"#;

        assert_eq!(on_demand_usd(reserved_only), None);
        assert_eq!(on_demand_usd("not json"), None);
        assert_eq!(on_demand_usd("{}"), None);
    }

    #[test]
    fn a_priced_key_becomes_a_monthly_estimate() {
        let key = PriceKey::Ec2Instance {
            region: "us-east-1".into(),
            instance_type: "t3.medium".into(),
        };
        let mut book = PriceBook::default();
        book.insert(key.clone(), Some(0.0416));

        let estimate = book.estimate(&key).expect("priced key has an estimate");
        assert!((estimate.monthly_usd - 30.368).abs() < 0.001);
        assert_eq!(estimate.basis, CostBasis::OnDemandListPrice);
    }

    /// A key looked up and found to have no published price is distinct from
    /// one never looked up, so a failed lookup is not retried every refresh.
    #[test]
    fn an_unpriced_key_is_remembered_as_unpriced() {
        let key = PriceKey::LoadBalancer {
            region: "us-east-1".into(),
            kind: LoadBalancerKind::Application,
        };
        let mut book = PriceBook::default();
        book.insert(key.clone(), None);

        assert!(book.contains(&key));
        assert_eq!(book.estimate(&key), None);
    }

    #[test]
    fn an_unknown_key_has_no_estimate() {
        let book = PriceBook::default();

        assert_eq!(
            book.estimate(&PriceKey::Ec2Instance {
                region: "us-east-1".into(),
                instance_type: "t3.medium".into(),
            }),
            None
        );
    }

    #[test]
    fn only_application_and_network_balancers_are_priced() {
        assert_eq!(
            LoadBalancerKind::from_lb_type("Application"),
            Some(LoadBalancerKind::Application)
        );
        assert_eq!(
            LoadBalancerKind::from_lb_type("Network"),
            Some(LoadBalancerKind::Network)
        );
        assert_eq!(LoadBalancerKind::from_lb_type("Gateway"), None);
    }
}
