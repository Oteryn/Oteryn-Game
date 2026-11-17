//! Authored grant identities and compatibility, admitted with the cosmetics document.
use super::{CosmeticsDiagnostic, Mount, Outfit, Result, text};
use crate::domain::appearance::{EarnedCosmetic, Sex, UnlockFact};
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GrantMapping {
    pub(super) grant_key: String,
    target: Target,
    compatible_provenance: Vec<String>,
    #[serde(default)]
    outfit_aliases: Vec<String>,
    #[serde(default)]
    sex: Option<GrantSex>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum GrantSex {
    Female,
    Male,
}
impl GrantSex {
    fn matches(self, sex: Sex) -> bool {
        matches!(
            (self, sex),
            (Self::Female, Sex::Female) | (Self::Male, Sex::Male)
        )
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
enum Target {
    Outfit { outfit_key: String },
    Addon { outfit_key: String, addon: u8 },
    Mount { mount_key: String },
}

/// The authored primary target stays distinct from compatible historical aliases.
pub(super) fn primary_target(mapping: &GrantMapping) -> EarnedCosmetic<'_> {
    match &mapping.target {
        Target::Outfit { outfit_key } => EarnedCosmetic::Outfit(outfit_key),
        Target::Addon { outfit_key, addon } => EarnedCosmetic::Addon(outfit_key, *addon),
        Target::Mount { mount_key } => EarnedCosmetic::Mount(mount_key),
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
        };
        let mut aliases = BTreeSet::new();
        for key in &mapping.outfit_aliases {
            let (Target::Outfit { outfit_key } | Target::Addon { outfit_key, .. }) =
                &mapping.target
            else {
                return Err(invalid());
            };
            let original = outfits
                .iter()
                .find(|o| o.key == *outfit_key)
                .ok_or_else(invalid)?;
            let alias = outfits.iter().find(|o| o.key == *key).ok_or_else(invalid)?;
            if mapping.outfit_aliases.len() > 16
                || key == outfit_key
                || !aliases.insert(key)
                || alias.female.id != original.female.id
                || alias.male.id != original.male.id
                || alias.female.source_record_sha256 != original.female.source_record_sha256
                || alias.male.source_record_sha256 != original.male.source_record_sha256
                || matches!(mapping.target, Target::Addon { addon, .. } if addon > alias.addon_count)
            {
                return Err(invalid());
            }
        }
        if !exists || (mapping.sex.is_some() && matches!(mapping.target, Target::Mount { .. })) {
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
    resolve_for_sex(mapping, provenance, target, None)
}

pub(super) fn resolve_for_sex(
    mapping: Option<&GrantMapping>,
    provenance: &str,
    target: EarnedCosmetic<'_>,
    sex: Option<Sex>,
) -> UnlockFact {
    let Some(mapping) = mapping else {
        return UnlockFact::Absent;
    };
    if mapping
        .sex
        .is_some_and(|required| sex.is_none_or(|sex| !required.matches(sex)))
    {
        return UnlockFact::Absent;
    }
    let matches = match (&mapping.target, target) {
        (Target::Outfit { outfit_key }, EarnedCosmetic::Outfit(key)) => {
            outfit_key == key || mapping.outfit_aliases.iter().any(|alias| alias == key)
        }
        (Target::Addon { outfit_key, addon }, EarnedCosmetic::Addon(key, bit)) => {
            (outfit_key == key || mapping.outfit_aliases.iter().any(|alias| alias == key))
                && *addon == bit
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

#[cfg(test)]
mod sex_qualified_tests {
    use super::*;

    #[test]
    fn aliases_require_identical_source_looks_and_supported_addons()
    -> std::result::Result<(), serde_json::Error> {
        let author: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../tools/content-schema/cosmetics-authoring/nx5-authoring.json"
        )))?;
        let mut outfits: Vec<Outfit> =
            serde_json::from_value(author["document"]["outfits"].clone())?;
        let mut alias: Outfit = serde_json::from_value(author["document"]["outfits"][0].clone())?;
        alias.key.push_str(".alias");
        let alias_key = alias.key.clone();
        let mapping: GrantMapping = serde_json::from_value(serde_json::json!({
            "grant_key": "test", "target": {"kind":"Addon", "outfit_key":outfits[0].key, "addon":1},
            "compatible_provenance":["pinned"], "outfit_aliases":[alias_key]
        }))?;
        assert!(
            matches!(primary_target(&mapping), EarnedCosmetic::Addon(key, 1)
            if key == outfits[0].key)
        );
        assert_eq!(
            resolve(
                Some(&mapping),
                "pinned",
                EarnedCosmetic::Addon(&alias_key, 1)
            ),
            UnlockFact::Compatible
        );
        assert_eq!(
            resolve(
                Some(&mapping),
                "wrong",
                EarnedCosmetic::Addon(&alias_key, 1)
            ),
            UnlockFact::Incompatible
        );
        outfits.push(alias);
        let last = outfits.len() - 1;
        assert!(validate(std::slice::from_ref(&mapping), &outfits, &[]).is_ok());
        outfits[last].female.id += 1;
        assert!(validate(std::slice::from_ref(&mapping), &outfits, &[]).is_err());
        outfits[last].female.id -= 1;
        outfits[last].female.source_record_sha256 = "1".repeat(64);
        assert!(validate(std::slice::from_ref(&mapping), &outfits, &[]).is_err());
        outfits[last].female.source_record_sha256 = outfits[0].female.source_record_sha256.clone();
        outfits[last].addon_count = 0;
        assert!(validate(std::slice::from_ref(&mapping), &outfits, &[]).is_err());
        Ok(())
    }

    #[test]
    fn qualified_grant_requires_matching_sex_target_and_provenance()
    -> std::result::Result<(), serde_json::Error> {
        let mut mapping: GrantMapping = serde_json::from_value(serde_json::json!({
            "grant_key":"test", "target":{"kind":"Addon","outfit_key":"hunter","addon":2},
            "compatible_provenance":["pinned"],"sex":"female"
        }))?;
        let target = EarnedCosmetic::Addon("hunter", 2);
        assert_eq!(
            resolve_for_sex(Some(&mapping), "pinned", target, Some(Sex::Female)),
            UnlockFact::Compatible
        );
        assert_eq!(
            resolve_for_sex(Some(&mapping), "pinned", target, Some(Sex::Male)),
            UnlockFact::Absent
        );
        assert_eq!(
            resolve(Some(&mapping), "pinned", target),
            UnlockFact::Absent
        );
        assert_eq!(
            resolve_for_sex(Some(&mapping), "stale", target, Some(Sex::Female)),
            UnlockFact::Incompatible
        );
        assert_eq!(
            resolve_for_sex(
                Some(&mapping),
                "pinned",
                EarnedCosmetic::Addon("hunter", 1),
                Some(Sex::Female)
            ),
            UnlockFact::Absent
        );
        mapping.outfit_aliases.push("legacy-hunter".into());
        assert_eq!(
            resolve_for_sex(
                Some(&mapping),
                "pinned",
                EarnedCosmetic::Addon("legacy-hunter", 2),
                Some(Sex::Female)
            ),
            UnlockFact::Compatible
        );
        assert_eq!(
            resolve_for_sex(
                Some(&mapping),
                "pinned",
                EarnedCosmetic::Addon("legacy-hunter", 2),
                Some(Sex::Male)
            ),
            UnlockFact::Absent
        );
        assert_eq!(
            resolve_for_sex(
                Some(&mapping),
                "stale",
                EarnedCosmetic::Addon("legacy-hunter", 2),
                Some(Sex::Female)
            ),
            UnlockFact::Incompatible
        );
        mapping.sex = None;
        assert_eq!(
            resolve(Some(&mapping), "pinned", target),
            UnlockFact::Compatible
        );
        assert_eq!(
            resolve_for_sex(Some(&mapping), "pinned", target, Some(Sex::Male)),
            UnlockFact::Compatible
        );
        Ok(())
    }
}
