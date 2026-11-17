//! Versioned, gameplay-independent admin control-plane wire contract.
mod catalog;
mod envelope;
mod json;
mod schema;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
pub use catalog::*;
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
pub use envelope::*;
pub use json::parse_json;
pub use schema::schemas;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

pub const CONTRACT: &str = "oteryn.admin-control.v1";
pub type WireError = Box<dyn std::error::Error + Send + Sync>;

/// JCS uses ECMAScript number formatting and UTF-16 property ordering.
/// Integers outside the interoperable range must be encoded as decimal strings.
pub fn canonical<T: Serialize>(value: &T) -> Result<Vec<u8>, WireError> {
    let bytes = serde_jcs::to_vec(value)?;
    let value = serde_json::to_value(value)?;
    fn check(v: &Value) -> Result<(), WireError> {
        match v {
            Value::Number(n) if n.is_i64() || n.is_u64() => {
                if n.as_f64().is_none_or(|f| f.abs() > 9_007_199_254_740_991.0) {
                    return Err("integer outside JCS interoperable range".into());
                }
            }
            Value::Array(a) => {
                for v in a {
                    check(v)?;
                }
            }
            Value::Object(o) => {
                for v in o.values() {
                    check(v)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    check(&value)?;
    Ok(bytes)
}

pub fn signed_bytes<T: Serialize>(envelope: &T) -> Result<Vec<u8>, WireError> {
    let mut value = parse_json(&canonical(envelope)?)?;
    value
        .as_object_mut()
        .ok_or("envelope must be an object")?
        .remove("signature");
    canonical(&value)
}
pub fn sign<T: Serialize>(envelope: &T, key: &SigningKey) -> Result<String, WireError> {
    Ok(URL_SAFE_NO_PAD.encode(key.sign(&signed_bytes(envelope)?).to_bytes()))
}
pub fn verify_signature<T: Serialize>(
    envelope: &T,
    signature: &str,
    key: &VerifyingKey,
) -> Result<(), WireError> {
    let signature = Signature::from_slice(&URL_SAFE_NO_PAD.decode(signature)?)?;
    key.verify_strict(&signed_bytes(envelope)?, &signature)?;
    Ok(())
}
pub fn digest<T: Serialize>(value: &T) -> Result<String, WireError> {
    Ok(URL_SAFE_NO_PAD.encode(Sha256::digest(canonical(value)?)))
}
