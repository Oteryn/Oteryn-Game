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
    pub digest: [u8; 32],
}

impl Package {
    pub fn new(recipes: String, layers: String) -> Self {
        let digest = fingerprint(&recipes, &layers);
        Self {
            recipes,
            layers,
            digest,
        }
    }

    pub fn validate(&self) -> Result<(), Error> {
        if self.recipes.is_empty()
            || self.recipes.len() > 4 << 20
            || self.layers.is_empty()
            || self.layers.len() > 8 << 20
            || self.digest != fingerprint(&self.recipes, &self.layers)
        {
            return Err(Error::Format("invalid World change package".into()));
        }
        Ok(())
    }
}

fn fingerprint(recipes: &str, layers: &str) -> [u8; 32] {
    Sha256::new()
        .chain_update(b"oteryn:wvar:recipes:v1\0")
        .chain_update(recipes.as_bytes())
        .chain_update(layers.as_bytes())
        .finalize()
        .into()
}
