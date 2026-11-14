//! Imported reference data, loaded on server boot without activating Wheel/Gem gameplay.
//!
//! The offline importer owns complete closed-schema and source qualification. This reader
//! binds the embedded files to that export and refuses mixed, corrupt or active catalogues.
//! No Character state, effect calculation, item materialization or protocol is created here.

use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

const WHEEL: &str = include_str!("../../../../rulesets/progression/wheel-of-destiny/wheel.json");
const GEMS: &str = include_str!("../../../../rulesets/progression/wheel-of-destiny/gems.json");
const MANIFEST: &str =
    include_str!("../../../../rulesets/progression/wheel-of-destiny/import-manifest.json");
const SOURCE: &str = "tools/content-schema/wheel-authoring/samples/wheel-candidate.json";
const VOCATIONS: [&str; 5] = ["druid", "knight", "monk", "paladin", "sorcerer"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WheelGemDataError(&'static str);

impl std::fmt::Display for WheelGemDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for WheelGemDataError {}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DataFile {
    schema: String,
    revision: String,
    runtime_admitted: bool,
    source_candidate_sha256: String,
    data: Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema: String,
    revision: String,
    runtime_admitted: bool,
    source_candidate_sha256: String,
    source: String,
    files: BTreeMap<String, String>,
    coverage: BTreeMap<String, usize>,
    pending_runtime: Vec<String>,
}

/// Immutable, data-only catalogue. Loading it grants no Wheel stages or Atelier operation.
#[derive(Debug)]
pub struct WheelGemData {
    wheel: DataFile,
    gems: DataFile,
}

fn reject<T>(reason: &'static str) -> Result<T, WheelGemDataError> {
    Err(WheelGemDataError(reason))
}

fn sha256_hex(bytes: &str) -> String {
    Sha256::digest(bytes.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

impl WheelGemData {
    pub fn embedded() -> Result<Self, WheelGemDataError> {
        Self::from_files(WHEEL, GEMS, MANIFEST)
    }

    fn from_files(wheel: &str, gems: &str, manifest: &str) -> Result<Self, WheelGemDataError> {
        let manifest: Manifest = serde_json::from_str(manifest)
            .map_err(|_| WheelGemDataError("malformed import manifest"))?;
        if manifest.schema != "OTERYN_WHEEL_GEM_DATA_MANIFEST/v1"
            || manifest.runtime_admitted
            || manifest.source != SOURCE
            || manifest.revision.is_empty()
            || manifest.pending_runtime.is_empty()
            || manifest.files.len() != 2
            || manifest.source_candidate_sha256.len() != 64
            || !manifest
                .source_candidate_sha256
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return reject("invalid data-only import manifest");
        }
        let mut parsed = Vec::new();
        for (name, bytes, schema) in [
            ("wheel.json", wheel, "OTERYN_WHEEL_DATA_IMPORT/v1"),
            ("gems.json", gems, "OTERYN_GEM_DATA_IMPORT/v1"),
        ] {
            let digest = sha256_hex(bytes);
            if manifest.files.get(name) != Some(&digest) {
                return reject("import file digest mismatch");
            }
            let data: DataFile = serde_json::from_str(bytes)
                .map_err(|_| WheelGemDataError("malformed imported data"))?;
            if data.schema != schema
                || data.runtime_admitted
                || data.revision != manifest.revision
                || data.source_candidate_sha256 != manifest.source_candidate_sha256
            {
                return reject("mixed or active import envelope");
            }
            let Some(vocations) = data.data["vocations"].as_object() else {
                return reject("missing vocation catalogue");
            };
            if vocations.len() != 5 || VOCATIONS.iter().any(|v| !vocations.contains_key(*v)) {
                return reject("incomplete vocation catalogue");
            }
            parsed.push(data);
        }
        let mut gems = parsed.pop().ok_or(WheelGemDataError("missing Gem file"))?;
        let wheel = parsed
            .pop()
            .ok_or(WheelGemDataError("missing Wheel file"))?;
        let count = |data: &Value, key: &str| data[key].as_array().map_or(0, Vec::len);
        let topology = &wheel.data["topology"];
        if count(&wheel.data, "topology") != 36
            || topology.as_array().is_none_or(|rows| {
                rows.iter()
                    .enumerate()
                    .any(|(i, row)| row["state_slot"].as_u64() != Some(i as u64 + 1))
            })
            || VOCATIONS.iter().any(|v| {
                count(&wheel.data["vocations"][v], "slots") != 36
                    || count(&wheel.data["vocations"][v], "revelations") != 4
            })
            || count(&gems.data["catalogue"], "basic_mods") != 46
            || count(&gems.data["catalogue"], "supreme_mods") != 94
            || manifest.coverage
                != BTreeMap::from([
                    ("vocations".into(), 5),
                    ("slots".into(), 180),
                    ("revelations".into(), 20),
                    ("basic_mods".into(), 46),
                    ("supreme_mods".into(), 94),
                ])
        {
            return reject("incomplete imported data");
        }
        // W06: owner-selected Winter test-server catalogue. Validate the original
        // import above before applying this central overlay, preserving paid revisions.
        let winter: Value = serde_json::from_str(include_str!(
            "../../../../docs/reference/wheel-of-destiny-testserver-20261113/new-supreme-mods.json"
        ))
        .map_err(|_| WheelGemDataError("malformed Winter modifier evidence"))?;
        for row in winter["new_mods"]
            .as_array()
            .ok_or(WheelGemDataError("Winter modifiers"))?
        {
            let id = row["source_id"]
                .as_u64()
                .ok_or(WheelGemDataError("Winter id"))?;
            let vocation = row["vocation"]
                .as_str()
                .ok_or(WheelGemDataError("Winter vocation"))?;
            let spell = row["spell_name"]
                .as_str()
                .ok_or(WheelGemDataError("Winter spell"))?;
            let kind = row["effect"]
                .as_str()
                .ok_or(WheelGemDataError("Winter effect"))?;
            let values = row["grade_values"]
                .as_array()
                .ok_or(WheelGemDataError("Winter grades"))?;
            if !(94..=113).contains(&id) || values.len() != 4 {
                return reject("invalid Winter modifier grades");
            }
            let grades: Vec<Value> = values.iter().enumerate().map(|(grade, value)| {
                let mut effects = vec![serde_json::json!({
                    "kind": kind, "target": format!("Augmented {spell}"),
                    "value": value.as_f64().map(f64::abs), "unit": row["unit"]
                })];
                if id == 97 {
                    effects.push(serde_json::json!({"kind": "momentum_chance",
                        "target": format!("Augmented {spell}"), "unit": "percent_points",
                        "value": ([0.0, 0.33, 0.66, 1.0][grade])}));
                }
                serde_json::json!({"grade": grade, "reference_text": format!("{spell}: {value}"),
                    "numeric_effects": effects})
            }).collect();
            let template_id = match kind {
                "base_damage_bonus" => 10,
                "critical_extra_damage" => 29,
                "base_healing_bonus" => 4,
                "cooldown_reduction" => 60,
                _ => return reject("unknown Winter modifier effect"),
            };
            let catalogue = gems.data["catalogue"]["supreme_mods"]
                .as_array_mut()
                .ok_or(WheelGemDataError("supreme catalogue"))?;
            let mut modifier = catalogue
                .iter()
                .find(|entry| entry["source_id"] == template_id)
                .ok_or(WheelGemDataError("supreme template"))?
                .clone();
            modifier["source_id"] = Value::from(id);
            modifier["name"] = Value::from(format!("Augmented {spell}"));
            modifier["summary_name"] = modifier["name"].clone();
            modifier["grades"] = Value::from(grades);
            modifier["icon"]["source_index"] = Value::from(id);
            catalogue.push(modifier);
            gems.data["vocations"][vocation]["supreme_mods"]
                .as_array_mut()
                .ok_or(WheelGemDataError("supreme pool"))?
                .push(Value::from(id));
        }
        Ok(Self { wheel, gems })
    }

    pub fn revision(&self) -> &str {
        &self.wheel.revision
    }
    pub fn source_candidate_sha256(&self) -> &str {
        &self.wheel.source_candidate_sha256
    }
    pub const fn wheel(&self) -> &Value {
        &self.wheel.data
    }
    pub const fn gems(&self) -> &Value {
        &self.gems.data
    }

    /// The active Wheel ruleset of this catalogue (WHEEL-W1): its revision and the slot and
    /// perk tables the allocation owner reads. Loading it activates no effect.
    pub fn wheel_ruleset(
        &self,
    ) -> Result<crate::wheel::WheelRuleset, crate::wheel::WheelRulesetError> {
        crate::wheel::WheelRuleset::from_catalogue(&self.wheel.revision, &self.wheel.data)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "tests use static, qualified fixtures")]
mod tests {
    use super::*;

    fn changed_file(change: impl FnOnce(&mut Value)) -> (String, String) {
        let mut data: Value = serde_json::from_str(GEMS).expect("fixture");
        change(&mut data);
        let data = serde_json::to_string(&data).expect("fixture");
        let mut manifest: Value = serde_json::from_str(MANIFEST).expect("fixture");
        manifest["files"]["gems.json"] = Value::String(sha256_hex(&data));
        (data, serde_json::to_string(&manifest).expect("fixture"))
    }

    #[test]
    fn full_catalogue_is_readable_without_gameplay_admission() {
        let data = WheelGemData::embedded().expect("qualified import");
        assert_eq!(
            data.wheel()["vocations"]["monk"]["slots"]
                .as_array()
                .map(Vec::len),
            Some(36)
        );
        assert_eq!(
            data.gems()["catalogue"]["grade_costs"][1]["supreme"]["gold"],
            12_000_000
        );
        assert_eq!(
            data.gems()["catalogue"]["grade_costs"][1]["supreme"]["fragments"],
            15
        );
        assert_eq!(
            data.gems()["catalogue"]["atelier"]["operation_policy"]["revealed_gem_limit"],
            225
        );
        assert!(!data.revision().is_empty());
        assert_eq!(data.source_candidate_sha256().len(), 64);
        assert_eq!(
            data.wheel_ruleset().expect("Wheel ruleset").revision(),
            data.revision()
        );
    }

    #[test]
    fn corrupted_or_missing_file_refuses_the_catalogue() {
        for bytes in ["{}", ""] {
            assert_eq!(
                WheelGemData::from_files(WHEEL, bytes, MANIFEST).expect_err("corrupt"),
                WheelGemDataError("import file digest mismatch")
            );
        }
    }

    #[test]
    fn even_rehashed_active_or_mixed_data_is_refused() {
        for field in ["runtime_admitted", "revision", "source_candidate_sha256"] {
            let (data, manifest) = changed_file(|data| {
                data[field] = if field == "runtime_admitted" {
                    Value::Bool(true)
                } else {
                    Value::String("foreign".into())
                };
            });
            assert_eq!(
                WheelGemData::from_files(WHEEL, &data, &manifest).expect_err("mixed"),
                WheelGemDataError("mixed or active import envelope")
            );
        }
    }

    #[test]
    fn rehashed_missing_vocation_refuses_partial_import() {
        let (data, manifest) = changed_file(|data| {
            data["data"]["vocations"]
                .as_object_mut()
                .expect("fixture")
                .remove("monk");
        });
        assert_eq!(
            WheelGemData::from_files(WHEEL, &data, &manifest).expect_err("partial"),
            WheelGemDataError("incomplete vocation catalogue")
        );
    }
}
