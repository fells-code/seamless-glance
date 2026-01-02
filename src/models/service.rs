use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsServiceItem {
    pub name: String,
    pub resource_type: String,
    pub count: usize,
}
