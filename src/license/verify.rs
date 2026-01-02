use base64::{engine::general_purpose, Engine};
use chrono::NaiveDate;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};

use crate::license::public_key::PUBLIC_KEY_B64;
use crate::license::License;

pub fn verify_license(license: &License) -> Result<(), String> {
    let expires = NaiveDate::parse_from_str(&license.expires_at, "%Y-%m-%d")
        .map_err(|_| "Invalid expiration date")?;

    if chrono::Utc::now().date_naive() > expires {
        return Err("License expired".into());
    }

    let payload = format!(
        "{}|{}|{}|{}",
        license.key, license.email, license.issued_at, license.expires_at
    );

    let pubkey_bytes: [u8; 32] = general_purpose::STANDARD
        .decode(PUBLIC_KEY_B64)
        .map_err(|_| "Invalid public key encoding")?
        .try_into()
        .map_err(|_| "Public key must be 32 bytes")?;

    let pubkey = VerifyingKey::from_bytes(&pubkey_bytes).map_err(|_| "Invalid public key")?;

    let sig_bytes: [u8; 64] = general_purpose::STANDARD
        .decode(&license.signature)
        .map_err(|_| "Invalid signature encoding")?
        .try_into()
        .map_err(|_| "Invalid signature length")?;

    let sig = Signature::from_bytes(&sig_bytes);

    pubkey
        .verify(payload.as_bytes(), &sig)
        .map_err(|_| "License signature invalid")?;

    Ok(())
}
