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
    Outfit {
        outfit_key: String,
    },
    Addon {
        outfit_key: String,
        addon: u8,
    },
    Mount {
        mount_key: String,
    },
    /// Crystal addOutfit/addOutfitAddon owns this exact look; 0 means base only.
    LookAddon {
        outfit_key: String,
        sex: SourceSex,
        look_type: u32,
        addon: u8,
    },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SourceSex {
    Female,
    Male,
}
impl SourceSex {
    fn matches(&self, sex: crate::domain::appearance::Sex) -> bool {
        matches!(
            (self, sex),
            (Self::Female, crate::domain::appearance::Sex::Female)
                | (Self::Male, crate::domain::appearance::Sex::Male)
        )
    }
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
            Target::LookAddon {
                outfit_key,
                sex,
                look_type,
                addon,
            } => outfits.iter().any(|o| {
                o.key == *outfit_key
                    && *addon <= o.addon_count
                    && match sex {
                        SourceSex::Female => o.female.id == *look_type,
                        SourceSex::Male => o.male.id == *look_type,
                    }
            }),
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

fn compatibility(mapping: &GrantMapping, provenance: &str) -> UnlockFact {
    if mapping
        .compatible_provenance
        .iter()
        .any(|p| p == provenance)
    {
        UnlockFact::Compatible
    } else {
        UnlockFact::Incompatible
    }
}
pub(super) fn resolve_for_sex(
    mapping: Option<&GrantMapping>,
    provenance: &str,
    target: EarnedCosmetic<'_>,
    sex: crate::domain::appearance::Sex,
) -> UnlockFact {
    let Some(mapping) = mapping else {
        return UnlockFact::Absent;
    };
    if let Target::LookAddon {
        outfit_key,
        sex: wanted,
        addon,
        ..
    } = &mapping.target
    {
        // addOutfitAddon creates the source look entry too (Crystal player.cpp6840–6850).
        let matches = wanted.matches(sex)
            && match target {
                EarnedCosmetic::Outfit(key) => outfit_key == key,
                EarnedCosmetic::Addon(key, bit) => outfit_key == key && *addon == bit && bit > 0,
                EarnedCosmetic::Mount(_) => false,
            };
        if matches {
            compatibility(mapping, provenance)
        } else {
            UnlockFact::Absent
        }
    } else {
        resolve(Some(mapping), provenance, target)
    }
}
pub(super) fn resolve_source_look(
    mapping: Option<&GrantMapping>,
    provenance: &str,
    look: u32,
    addon: u8,
    outfits: &[Outfit],
) -> UnlockFact {
    let Some(mapping) = mapping else {
        return UnlockFact::Absent;
    };
    let matches = match &mapping.target {
        Target::LookAddon {
            look_type,
            addon: bit,
            ..
        } => *look_type == look && (addon == 0 || *bit == addon),
        Target::Outfit { outfit_key } => {
            addon == 0
                && outfits
                    .iter()
                    .any(|o| o.key == *outfit_key && (o.female.id == look || o.male.id == look))
        }
        Target::Addon {
            outfit_key,
            addon: bit,
        } => {
            (addon == 0 || *bit == addon)
                && outfits
                    .iter()
                    .any(|o| o.key == *outfit_key && (o.female.id == look || o.male.id == look))
        }
        Target::Mount { .. } => false,
    };
    if matches {
        compatibility(mapping, provenance)
    } else {
        UnlockFact::Absent
    }
}
