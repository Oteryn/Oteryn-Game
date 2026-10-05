//! Explicit Outfit definitions for temporary spell conditions. The source importer verifies
//! pinned git blobs and the exact Outfit_t default closure; this closed document is retained
//! inside the approved active artifact. Numeric appearance membership alone cannot create it.
#![allow(
    dead_code,
    reason = "spell import candidate; awaits its production owner caller"
)]
use super::ContentError;
use super::project::{ProjectV2DefinitionRef, ProjectV2Family};
use crate::domain::appearance::AppearanceSelection;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
const PIN: &str = "99902524e052f37574194466c2949c576e4ab269";
const DEFAULT_SHA: &str = "dd403b4c2291b66bd5df49d1a59b45667698d8a916ecfb8fa87801688e1ca622";
const CRYSTAL_PIN: &str = "ff7ede593c69d4c658b382c97443e8155926924a";
const STAG_PATH: &str = "data-global/monster/winter_update_2025/stag.lua";
const STAG_BLOB: &str = "0756e605384b018f75c498bad89ddc72b961c0ca";
const STAG_SHA: &str = "e3ec6434cabac687b502bb69d0a5abd3f40a11fc24ffa92c04d3cef45e777aeb";
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SpellAppearanceDefaultSource {
    pub(crate) revision: String,
    pub(crate) path: String,
    pub(crate) git_blob: String,
    pub(crate) sha256: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SpellAppearanceSource {
    pub(crate) revision: String,
    pub(crate) path: String,
    pub(crate) git_blob: String,
    pub(crate) sha256: String,
    pub(crate) explicit_fields: BTreeMap<String, u32>,
    pub(crate) defaulted_fields: Vec<String>,
    // Canary uses the enclosing document's existing default closure. The
    // separately imported Crystal Stag must carry its actual Crystal header.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) default_source: Option<SpellAppearanceDefaultSource>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SpellAppearanceRecord {
    pub(crate) outfit_key: String,
    pub(crate) outfit_revision: String,
    pub(crate) look_type: u32,
    pub(crate) colours: [u16; 4],
    pub(crate) addons: u8,
    pub(crate) mount_key: Option<String>,
    pub(crate) creature: Option<ProjectV2DefinitionRef>,
    pub(crate) source: SpellAppearanceSource,
    pub(crate) qualification_sha256: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SpellAppearancesDocument {
    pub(crate) schema: String,
    pub(crate) default_source_revision: String,
    pub(crate) default_source_path: String,
    pub(crate) default_source_sha256: String,
    pub(crate) records: Vec<SpellAppearanceRecord>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompiledSpellAppearances {
    digest: [u8; 32],
    records: Vec<SpellAppearanceRecord>,
}
#[derive(Debug, Clone, Copy)]
pub(crate) struct QualifiedSpellAppearance<'a> {
    digest: [u8; 32],
    record: &'a SpellAppearanceRecord,
}
impl QualifiedSpellAppearance<'_> {
    pub(crate) const fn source_digest(self) -> [u8; 32] {
        self.digest
    }
    pub(crate) const fn look_type(self) -> u32 {
        self.record.look_type
    }
    pub(crate) fn selection(self) -> AppearanceSelection<String> {
        AppearanceSelection {
            outfit_key: self.record.outfit_key.clone(),
            colours: self.record.colours,
            addons: self.record.addons,
            mount_key: self.record.mount_key.clone(),
        }
    }
}
impl CompiledSpellAppearances {
    /// Data-only outer artifact binding. The staging owner separately verifies
    /// the independently issued enclosing artifact before calling this.
    pub(crate) fn bind_qualified_outer_artifact(
        &mut self,
        digest: [u8; 32],
    ) -> Result<(), ContentError> {
        if digest == [0; 32] {
            return Err(ContentError::InvalidArtifact(
                "zero outer appearance artifact pin",
            ));
        }
        self.digest = digest;
        Ok(())
    }
    pub(crate) const fn source_digest(&self) -> [u8; 32] {
        self.digest
    }
    pub(crate) fn for_creature(
        &self,
        key: &str,
        revision: &str,
    ) -> Option<QualifiedSpellAppearance<'_>> {
        let record = self.records.iter().find(|r| {
            r.creature
                .as_ref()
                .is_some_and(|c| c.key == key && c.revision == revision)
        })?;
        Some(QualifiedSpellAppearance {
            digest: self.digest,
            record,
        })
    }
    pub(crate) fn for_look_type(&self, look_type: u32) -> Option<QualifiedSpellAppearance<'_>> {
        let record = self
            .records
            .iter()
            .find(|r| r.creature.is_none() && r.look_type == look_type)?;
        Some(QualifiedSpellAppearance {
            digest: self.digest,
            record,
        })
    }
    pub(crate) fn creature_links(&self) -> impl Iterator<Item = (&ProjectV2DefinitionRef, u32)> {
        self.records
            .iter()
            .filter_map(|r| r.creature.as_ref().map(|c| (c, r.look_type)))
    }
}
fn invalid() -> ContentError {
    ContentError::InvalidArtifact("native spell appearance qualification")
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn hash(value: &str, width: usize) -> bool {
    value.len() == width
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn qualified_crystal_stag(record: &SpellAppearanceRecord) -> bool {
    let Some(creature) = &record.creature else {
        return false;
    };
    let Some(defaults) = &record.source.default_source else {
        return false;
    };
    record.source.revision == CRYSTAL_PIN
        && creature.family == ProjectV2Family::Creature
        && creature.key == "oteryn:creature.stag"
        && creature.revision == "definition-r1"
        && record.outfit_key == "oteryn:outfit.source.crystal.look1913"
        && record.look_type == 1913
        && record.source.path == STAG_PATH
        && record.source.git_blob == STAG_BLOB
        && record.source.sha256 == STAG_SHA
        && defaults.revision == CRYSTAL_PIN
        && defaults.path == "src/creatures/creatures_definitions.hpp"
        && defaults.git_blob == "fc4ae0cf45a56ee7c363b0f452acfb14cbd3c63a"
        && defaults.sha256 == "8fbc70bf2416b6e76f17cce3dc1b075f8a5451f0f85b522b0a4ae34cf2cccf94"
        && record.source.defaulted_fields == ["lookTypeEx"]
        && record.source.explicit_fields.len() == 7
        && record.source.explicit_fields.iter().all(|(key, value)| {
            if key == "lookType" {
                *value == 1913
            } else {
                *value == 0
                    && matches!(
                        key.as_str(),
                        "lookHead"
                            | "lookBody"
                            | "lookLegs"
                            | "lookFeet"
                            | "lookAddons"
                            | "lookMount"
                    )
            }
        })
}
pub(crate) fn compile(
    bytes: &[u8],
    outer_content_digest: [u8; 32],
) -> Result<CompiledSpellAppearances, ContentError> {
    if bytes.len() > 8 * 1024 * 1024 || outer_content_digest == [0; 32] {
        return Err(invalid());
    }
    let document: SpellAppearancesDocument =
        serde_json::from_slice(bytes).map_err(|_| invalid())?;
    if document.schema != "OTERYN_NATIVE_SPELL_APPEARANCES/v1"
        || document.default_source_revision != PIN
        || document.default_source_path != "src/creatures/creatures_definitions.hpp"
        || document.default_source_sha256 != DEFAULT_SHA
        || document.records.is_empty()
        || document.records.len() > 4096
    {
        return Err(invalid());
    }
    let fields = [
        "lookAddons",
        "lookBody",
        "lookFeet",
        "lookHead",
        "lookLegs",
        "lookMount",
        "lookType",
        "lookTypeEx",
    ];
    let mut owners = BTreeSet::new();
    let mut fixed = BTreeSet::new();
    for record in &document.records {
        let crystal_stag = qualified_crystal_stag(record);
        let canary = record.source.revision == PIN && record.source.default_source.is_none();
        if (!canary && !crystal_stag)
            || (canary
                && record.outfit_key
                    != format!("oteryn:outfit.source.canary.look{}", record.look_type))
            || record.outfit_revision != "source-outfit-r1"
            || record.look_type == 0
            || record.look_type > u32::from(u16::MAX)
            || record.mount_key.is_some()
            || !hash(&record.source.git_blob, 40)
            || !hash(&record.source.sha256, 64)
            || record.source.path.starts_with('/')
            || record
                .source
                .path
                .split('/')
                .any(|p| p.is_empty() || p == "..")
            || !record.source.path.ends_with(".lua")
            || record
                .source
                .defaulted_fields
                .windows(2)
                .any(|w| w[0] >= w[1])
        {
            return Err(invalid());
        }
        if let Some(creature) = &record.creature {
            if creature.family != ProjectV2Family::Creature
                || !owners.insert((creature.key.clone(), creature.revision.clone()))
                || (!crystal_stag
                    && !record
                        .source
                        .path
                        .starts_with("data-otservbr-global/monster/"))
            {
                return Err(invalid());
            }
        } else if !fixed.insert(record.look_type)
            || !record
                .source
                .path
                .starts_with("data/scripts/spells/support/avatar_of_")
        {
            return Err(invalid());
        }
        if record
            .source
            .explicit_fields
            .keys()
            .any(|key| !fields.contains(&key.as_str()))
            || record.source.defaulted_fields.iter().any(|key| {
                !fields.contains(&key.as_str()) || record.source.explicit_fields.contains_key(key)
            })
            || fields.iter().any(|key| {
                !record.source.explicit_fields.contains_key(*key)
                    && !record.source.defaulted_fields.iter().any(|s| s == key)
            })
        {
            return Err(invalid());
        }
        let value = |key: &str| record.source.explicit_fields.get(key).copied().unwrap_or(0);
        if value("lookType") != record.look_type
            || value("lookTypeEx") != 0
            || value("lookMount") != 0
            || value("lookAddons") != u32::from(record.addons)
            || [
                value("lookHead"),
                value("lookBody"),
                value("lookLegs"),
                value("lookFeet"),
            ] != record.colours.map(u32::from)
            || record.colours.iter().any(|v| *v > 255)
        {
            return Err(invalid());
        }
        let mut qualified = serde_json::to_value(record).map_err(|_| invalid())?;
        qualified
            .as_object_mut()
            .ok_or_else(invalid)?
            .remove("qualification_sha256");
        let canonical = serde_json::to_vec(&qualified).map_err(|_| invalid())?;
        if record.qualification_sha256 != hex(&super::digest::sha256(&canonical)) {
            return Err(invalid());
        }
    }
    Ok(CompiledSpellAppearances {
        digest: outer_content_digest,
        records: document.records,
    })
}
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    fn fixture() -> Vec<u8> {
        include_bytes!("../../../../tools/content-schema/native-gameplay/spell_appearances.json")
            .to_vec()
    }
    fn requalify(record: &mut SpellAppearanceRecord) {
        let mut value = serde_json::to_value(&*record).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .remove("qualification_sha256");
        record.qualification_sha256 = hex(&super::super::digest::sha256(
            &serde_json::to_vec(&value).unwrap(),
        ));
    }
    fn crystal_fixture() -> SpellAppearancesDocument {
        let mut document: SpellAppearancesDocument = serde_json::from_slice(&fixture()).unwrap();
        let mut stag: SpellAppearanceRecord = serde_json::from_value(serde_json::json!({
            "outfit_key":"oteryn:outfit.source.crystal.look1913", "outfit_revision":"source-outfit-r1",
            "look_type":1913, "colours":[0,0,0,0], "addons":0, "mount_key":null,
            "creature":{"family":"Creature","key":"oteryn:creature.stag","revision":"definition-r1"},
            "source":{"revision":"ff7ede593c69d4c658b382c97443e8155926924a",
                "path":"data-global/monster/winter_update_2025/stag.lua",
                "git_blob":"0756e605384b018f75c498bad89ddc72b961c0ca",
                "sha256":"e3ec6434cabac687b502bb69d0a5abd3f40a11fc24ffa92c04d3cef45e777aeb",
                "explicit_fields":{"lookType":1913,"lookHead":0,"lookBody":0,"lookLegs":0,"lookFeet":0,"lookAddons":0,"lookMount":0},
                "defaulted_fields":["lookTypeEx"],
                "default_source":{"revision":"ff7ede593c69d4c658b382c97443e8155926924a",
                    "path":"src/creatures/creatures_definitions.hpp",
                    "git_blob":"fc4ae0cf45a56ee7c363b0f452acfb14cbd3c63a",
                    "sha256":"8fbc70bf2416b6e76f17cce3dc1b075f8a5451f0f85b522b0a4ae34cf2cccf94"}},
            "qualification_sha256":""
        })).unwrap();
        requalify(&mut stag);
        document.records.push(stag);
        document
    }
    #[test]
    fn crystal_stag_uses_its_exact_primary_and_default_source() {
        let document = crystal_fixture();
        let compiled = compile(&serde_json::to_vec(&document).unwrap(), [7; 32]).unwrap();
        let stag = compiled
            .for_creature("oteryn:creature.stag", "definition-r1")
            .unwrap();
        assert_eq!(stag.look_type(), 1913);
        assert_eq!(
            stag.selection().outfit_key,
            "oteryn:outfit.source.crystal.look1913"
        );
        assert_eq!(stag.source_digest(), [7; 32]);
        assert!(
            compiled
                .for_creature("canary:creature/stag", "definition-r1")
                .is_none()
        );
    }
    #[test]
    fn crystal_primary_default_and_identity_substitutions_refuse_after_rehash() {
        let mutations: &[fn(&mut SpellAppearanceRecord)] = &[
            |r| r.source.revision = PIN.into(),
            |r| r.creature.as_mut().unwrap().key = "oteryn:creature.rat".into(),
            |r| r.creature.as_mut().unwrap().revision = "definition-r2".into(),
            |r| r.source.path = "data-global/monster/winter_update_2025/other.lua".into(),
            |r| r.source.git_blob.replace_range(0..1, "1"),
            |r| r.source.sha256.replace_range(0..1, "1"),
            |r| r.source.default_source = None,
            |r| r.source.default_source.as_mut().unwrap().revision = PIN.into(),
            |r| r.source.default_source.as_mut().unwrap().path = "other.hpp".into(),
            |r| {
                r.source
                    .default_source
                    .as_mut()
                    .unwrap()
                    .git_blob
                    .replace_range(0..1, "1")
            },
            |r| {
                r.source
                    .default_source
                    .as_mut()
                    .unwrap()
                    .sha256
                    .replace_range(0..1, "1")
            },
            |r| {
                r.source.defaulted_fields.clear();
                r.source.explicit_fields.insert("lookTypeEx".into(), 0);
            },
        ];
        for (index, change) in mutations.iter().enumerate() {
            let mut document = crystal_fixture();
            let stag = document.records.last_mut().unwrap();
            change(stag);
            requalify(stag);
            assert!(
                compile(&serde_json::to_vec(&document).unwrap(), [7; 32]).is_err(),
                "mutation {index}"
            );
        }
    }
    #[test]
    fn actual_source_selections_bind_active_generation() {
        let compiled = compile(&fixture(), [7; 32]).unwrap();
        let rat = compiled
            .for_creature("canary:creature/rat", "canary-47dfd51f")
            .unwrap();
        assert_eq!(rat.look_type(), 21);
        assert_eq!(rat.source_digest(), [7; 32]);
        assert_eq!(rat.selection().colours, [0; 4]);
        assert!(
            compiled
                .for_creature("canary:creature/rat", "tampered")
                .is_none()
        );
        assert!(compiled.for_look_type(1593).is_some());
    }
    #[test]
    fn changed_selection_missing_source_and_unknown_generation_refuse() {
        assert!(compile(&fixture(), [0; 32]).is_err());
        let mut document: serde_json::Value = serde_json::from_slice(&fixture()).unwrap();
        document["records"][0]["colours"][0] = serde_json::json!(33);
        assert!(compile(&serde_json::to_vec(&document).unwrap(), [7; 32]).is_err());
        document["default_source_sha256"] = serde_json::json!("unqualified");
        assert!(compile(&serde_json::to_vec(&document).unwrap(), [7; 32]).is_err());
    }
}
