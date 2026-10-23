//! Authored grant identities and compatibility, admitted with the cosmetics document.
use super::{CosmeticsDiagnostic, Mount, Outfit, Result, text};
use crate::domain::appearance::{EarnedCosmetic, UnlockFact};
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GrantMapping {
    pub(super) grant_key: String,
    target: Target,
    compatible_provenance: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
enum Target {
    Outfit { outfit_key: String },
    Addon { outfit_key: String, addon: u8 },
    Mount { mount_key: String },
}

pub(super) fn validate(
    mappings: &[GrantMapping],
    outfits: &[Outfit],
    mounts: &[Mount],
) -> Result<()> {
    let invalid = || CosmeticsDiagnostic::Definition {
        family: "GrantMapping",
        index: 0,
    };
    if mappings.len() > 4096 {
        return Err(invalid());
    }
    let mut keys = BTreeSet::new();
    for mapping in mappings {
        let mut provenance = BTreeSet::new();
        if !text(&mapping.grant_key, 512)
            || !keys.insert(&mapping.grant_key)
            || mapping.compatible_provenance.is_empty()
            || mapping.compatible_provenance.len() > 256
            || mapping
                .compatible_provenance
                .iter()
                .any(|p| !text(p, 128) || !provenance.insert(p))
        {
            return Err(invalid());
        }
        let exists = match &mapping.target {
            Target::Outfit { outfit_key } => outfits.iter().any(|o| o.key == *outfit_key),
            Target::Addon { outfit_key, addon } => outfits
                .iter()
                .any(|o| o.key == *outfit_key && (1..=o.addon_count).contains(addon)),
            Target::Mount { mount_key } => mounts.iter().any(|m| m.key == *mount_key),
        };
        if !exists {
            return Err(invalid());
        }
    }
    Ok(())
}

pub(super) fn resolve(
    mapping: Option<&GrantMapping>,
    provenance: &str,
    target: EarnedCosmetic<'_>,
) -> UnlockFact {
    let Some(mapping) = mapping else {
        return UnlockFact::Absent;
    };
    let matches = match (&mapping.target, target) {
        (Target::Outfit { outfit_key }, EarnedCosmetic::Outfit(key)) => outfit_key == key,
        (Target::Addon { outfit_key, addon }, EarnedCosmetic::Addon(key, bit)) => {
            outfit_key == key && *addon == bit
        }
        (Target::Mount { mount_key }, EarnedCosmetic::Mount(key)) => mount_key == key,
        _ => false,
    };
    if !matches {
        UnlockFact::Absent
    } else if mapping
        .compatible_provenance
        .iter()
        .any(|p| p == provenance)
    {
        UnlockFact::Compatible
    } else {
        UnlockFact::Incompatible
    }
}
