//! Optional W1 content payload. Gameplay rules stay with world_event::changes;
//! this envelope binds the exact authored recipes/layers into the World digest.
use crate::Error;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const CAPABILITY: &str = "world_change_variants_v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Package {
    pub recipes: String,
    pub layers: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub counter_bindings: String,
    pub digest: [u8; 32],
}

impl Package {
    pub fn new(recipes: String, layers: String, counter_bindings: String) -> Self {
        let digest = fingerprint(&recipes, &layers, &counter_bindings);
        Self {
            recipes,
            layers,
            counter_bindings,
            digest,
        }
    }

    pub fn validate(&self) -> Result<(), Error> {
        if self.recipes.is_empty()
            || self.recipes.len() > 4 << 20
            || self.layers.is_empty()
            || self.layers.len() > 8 << 20
            || self.counter_bindings.len() > 1 << 20
            || self.digest != fingerprint(&self.recipes, &self.layers, &self.counter_bindings)
        {
            return Err(Error::Format("invalid World change package".into()));
        }
        Ok(())
    }
}

/// One fingerprint shared by the compiler envelope and the stored server decision.
pub fn fingerprint(recipes: &str, layers: &str, counter_bindings: &str) -> [u8; 32] {
    Sha256::new()
        .chain_update(b"oteryn:wvar:recipes:v1\0")
        .chain_update(recipes.as_bytes())
        .chain_update(layers.as_bytes())
        .chain_update(counter_bindings.as_bytes())
        .finalize()
        .into()
}

#[cfg(test)]
mod tests {
    use super::Package;

    #[test]
    fn counter_binding_tamper_cannot_reuse_a_world_package_digest() {
        let package = Package::new("[]".into(), "{\"layers\":[]}".into(), "{}".into());
        assert!(package.validate().is_ok());
        let mut tampered = package.clone();
        tampered.counter_bindings.push(' ');
        assert!(tampered.validate().is_err());
        let changed = Package::new(package.recipes, package.layers, tampered.counter_bindings);
        assert_ne!(changed.digest, package.digest);
    }

    #[test]
    fn counter_binding_envelope_round_trips_all_fields() -> Result<(), Box<dyn std::error::Error>> {
        let package = Package::new("[]".into(), "{\"layers\":[]}".into(), "{}".into());
        let json = serde_json::to_value(&package)?;
        assert_eq!(json["counter_bindings"], "{}");
        let restored: Package = serde_json::from_value(json)?;
        assert_eq!(restored, package);
        assert!(restored.validate().is_ok());
        Ok(())
    }

    #[test]
    fn empty_legacy_counter_bindings_preserve_the_v1_digest()
    -> Result<(), Box<dyn std::error::Error>> {
        let package = Package::new("[]".into(), "{\"layers\":[]}".into(), String::new());
        assert_eq!(
            package.digest,
            [
                0x2c, 0x16, 0xd6, 0xfa, 0x00, 0xb7, 0xf9, 0x3c, 0xe1, 0x4f, 0x7f, 0xdc, 0x6c, 0xa0,
                0x4b, 0xa0, 0x14, 0x6f, 0xa1, 0xad, 0xf2, 0x52, 0xb1, 0xbf, 0xed, 0x21, 0xfb, 0x16,
                0x52, 0xf3, 0xd3, 0x70,
            ]
        );
        let json = serde_json::to_value(&package)?;
        assert!(json.get("counter_bindings").is_none());
        let restored: Package = serde_json::from_value(json)?;
        assert!(restored.counter_bindings.is_empty());
        assert_eq!(restored, package);
        assert!(restored.validate().is_ok());
        Ok(())
    }
}
