//! Pinned source configuration imported into the actual native gameplay artifact.
//! These defaults become known only through that activated artifact; no missing setting fallback.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourcePin {
    repository: String,
    revision: String,
    config_sha256: String,
    vip_implementation_sha256: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceConfig {
    schema: String,
    source: SourcePin,
    familiar_minutes: i64,
    cooldown_rate_decimal: String,
    vip_enabled: bool,
    vip_reduction_minutes: i64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompiledFamiliarConfig {
    source_digest: [u8; 32],
    config: SourceConfig,
}
impl CompiledFamiliarConfig {
    /// Data-only binding after Content independently qualifies its enclosing artifact.
    /// The validated source settings and file pins remain unchanged.
    pub(crate) fn bind_qualified_outer_artifact(
        &mut self,
        digest: [u8; 32],
    ) -> Result<(), &'static str> {
        if digest == [0; 32] {
            return Err("unqualified familiar outer artifact");
        }
        self.source_digest = digest;
        Ok(())
    }
    pub(crate) fn from_active_artifact(
        bytes: &[u8],
        source_digest: [u8; 32],
    ) -> Result<Self, &'static str> {
        if source_digest == [0; 32] || bytes.len() > 4096 {
            return Err("unqualified familiar config artifact");
        }
        let config: SourceConfig =
            serde_json::from_slice(bytes).map_err(|_| "invalid familiar config")?;
        let canonical: SourceConfig = serde_json::from_slice(include_bytes!(
            "../../../../tools/content-schema/native-gameplay/familiar-config.json"
        ))
        .map_err(|_| "invalid embedded source qualification")?;
        if config != canonical {
            return Err("unqualified familiar configuration/source pin");
        }
        Ok(Self {
            source_digest,
            config,
        })
    }
    pub(crate) fn source_digest(&self) -> [u8; 32] {
        self.source_digest
    }
    pub(crate) fn familiar_minutes(&self) -> i64 {
        self.config.familiar_minutes
    }
    pub(crate) fn cooldown_rate(&self) -> f32 {
        1.0
    } // Exact qualified source decimal above; never a fallback.
    pub(crate) fn vip_enabled(&self) -> bool {
        self.config.vip_enabled
    }
    pub(crate) fn vip_reduction_minutes(&self) -> i64 {
        self.config.vip_reduction_minutes
    }
}
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    const SOURCE: &[u8] =
        include_bytes!("../../../../tools/content-schema/native-gameplay/familiar-config.json");
    #[test]
    fn source_defaults_are_known_only_with_complete_active_pin_and_exact_configuration() {
        let mut config = CompiledFamiliarConfig::from_active_artifact(SOURCE, [7; 32]).unwrap();
        assert_eq!(config.familiar_minutes(), 30);
        assert_eq!(config.cooldown_rate(), 1.0);
        assert!(!config.vip_enabled());
        assert_eq!(config.vip_reduction_minutes(), 0);
        assert!(config.bind_qualified_outer_artifact([0; 32]).is_err());
        assert_eq!(config.source_digest(), [7; 32]);
        config.bind_qualified_outer_artifact([8; 32]).unwrap();
        assert_eq!(config.source_digest(), [8; 32]);
        assert_eq!(config.familiar_minutes(), 30);
        assert!(CompiledFamiliarConfig::from_active_artifact(SOURCE, [0; 32]).is_err());
        let mut changed: serde_json::Value = serde_json::from_slice(SOURCE).unwrap();
        changed["cooldown_rate_decimal"] = "2.0".into();
        assert!(
            CompiledFamiliarConfig::from_active_artifact(
                &serde_json::to_vec(&changed).unwrap(),
                [7; 32]
            )
            .is_err()
        );
        changed["cooldown_rate_decimal"] = "1.0".into();
        changed["source"]["revision"] = "main".into();
        assert!(
            CompiledFamiliarConfig::from_active_artifact(
                &serde_json::to_vec(&changed).unwrap(),
                [7; 32]
            )
            .is_err()
        );
    }
}
