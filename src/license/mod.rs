use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct License {
    pub key: String,
    pub email: String,
    pub issued_at: String,
    pub expires_at: String,
    pub signature: String,
}

pub mod load;
pub mod public_key;
pub mod verify;
