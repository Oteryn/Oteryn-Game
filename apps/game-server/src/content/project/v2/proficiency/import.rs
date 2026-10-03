//! Data-only import of the committed catalogue. No Character state, activation or perk execution.

use super::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

const INDEX: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../content/proficiencies/index.json"
));
const SHARDS: [&str; 3] = [
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../content/proficiencies/proficiencies-00000-00149.json"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../content/proficiencies/proficiencies-00150-00299.json"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../content/proficiencies/proficiencies-00300-00442.json"
    )),
];
const BINDINGS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../content/proficiencies/bindings.json"
));
const RULES: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../rulesets/progression/weapon-proficiency/progression.json"
));

#[derive(Debug, Clone, Serialize)]
pub struct ImportedWeaponProficiency {
    pub identity: super::super::ProjectV2Identity,
    pub name: String,
    pub source: ImportedProficiencySource,
    pub levels: Vec<ProjectV2ProficiencyLevel>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ImportedProficiencySource {
    pub proficiency_id: u32,
    pub version: u16,
    pub source_record_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ImportedWeaponProficiencyBinding {
    pub item: ProjectV2DefinitionRef,
    pub profile_binding: ProjectV2DefinitionRef,
    pub threshold_class: ProjectV2ProficiencyThresholdClass,
}

/// Static definitions only; importing this value grants no runtime/admission authority.
#[derive(Debug, Serialize)]
pub struct ImportedWeaponProficiencyData {
    definitions: BTreeMap<String, ImportedWeaponProficiency>,
    bindings: BTreeMap<String, ImportedWeaponProficiencyBinding>,
    progress_thresholds: BTreeMap<String, [u32; 9]>,
    mastery_offset: u8,
    binding_source: SourceFile,
    excluded: Value,
    progression_ruleset: Value,
}

impl ImportedWeaponProficiencyData {
    pub fn definitions(&self) -> &BTreeMap<String, ImportedWeaponProficiency> {
        &self.definitions
    }
    pub fn bindings(&self) -> &BTreeMap<String, ImportedWeaponProficiencyBinding> {
        &self.bindings
    }
    pub fn progress_thresholds(&self) -> &BTreeMap<String, [u32; 9]> {
        &self.progress_thresholds
    }
    pub fn progression_ruleset(&self) -> &Value {
        &self.progression_ruleset
    }
    pub fn mastery_offset(&self) -> u8 {
        self.mastery_offset
    }
}

/// Import the repository's actual committed data, independent of unfinished gameplay consumers.
/// Source-order levels/perks and source provenance are retained; decimals become exact ratios.
pub fn import_committed_weapon_proficiency_data()
-> Result<ImportedWeaponProficiencyData, ProjectError> {
    import(INDEX, &SHARDS, BINDINGS, RULES)
}

fn invalid(reason: &'static str) -> ProjectError {
    ProjectError::InvalidProject(reason)
}
fn parse<T: serde::de::DeserializeOwned>(text: &str) -> Result<T, ProjectError> {
    serde_json::from_str(text).map_err(|error| ProjectError::InvalidJson(error.to_string()))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Index {
    schema: String,
    family: String,
    record_count: usize,
    shards: Vec<String>,
    authoring_source: Value,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Shard {
    schema: String,
    family: String,
    records: Vec<Record>,
    shard: ShardRange,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ShardRange {
    count: usize,
    end: usize,
    index: usize,
    start: usize,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SourceFile {
    path: String,
    sha256: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    definition: Definition,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Definition {
    identity: super::super::ProjectV2Identity,
    name: String,
    source: ImportedProficiencySource,
    levels: Value,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Bindings {
    schema: String,
    record_count: usize,
    records: Vec<ImportedWeaponProficiencyBinding>,
    excluded: Value,
    source: SourceFile,
}

fn decimal_ratio(value: &Value) -> Result<Value, ProjectError> {
    let number = value
        .as_number()
        .ok_or_else(|| invalid("proficiency value is not numeric"))?
        .to_string();
    let negative = number.starts_with('-');
    let digits = number.strip_prefix('-').unwrap_or(&number);
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    if whole.is_empty()
        || !whole
            .bytes()
            .chain(fraction.bytes())
            .all(|byte| byte.is_ascii_digit())
    {
        return Err(invalid("proficiency decimal encoding unsupported"));
    }
    let scale = u32::try_from(fraction.len()).map_err(|_| invalid("proficiency decimal scale"))?;
    let denominator = 10_u64
        .checked_pow(scale)
        .ok_or_else(|| invalid("proficiency decimal overflow"))?;
    let magnitude: u64 = format!("{whole}{fraction}")
        .parse()
        .map_err(|_| invalid("proficiency decimal overflow"))?;
    let (mut a, mut b) = (magnitude, denominator);
    while b != 0 {
        (a, b) = (b, a % b);
    }
    let divisor = a.max(1);
    let numerator =
        i64::try_from(magnitude / divisor).map_err(|_| invalid("proficiency decimal overflow"))?;
    Ok(
        serde_json::json!({"numerator": if negative { -numerator } else { numerator }, "denominator": denominator / divisor}),
    )
}

fn import(
    index: &str,
    shards: &[&str],
    bindings: &str,
    rules: &str,
) -> Result<ImportedWeaponProficiencyData, ProjectError> {
    let index: Index = parse(index)?;
    let expected = [
        "content/proficiencies/proficiencies-00000-00149.json",
        "content/proficiencies/proficiencies-00150-00299.json",
        "content/proficiencies/proficiencies-00300-00442.json",
    ];
    if index.schema != "OTERYN_FAMILY_INDEX/v1"
        || index.family != "Proficiency"
        || index.shards != expected
        || shards.len() != expected.len()
        || index.authoring_source["schema"] != "OTERYN_PROFICIENCY_AUTHORING_CATALOGUE/v1"
    {
        return Err(invalid("proficiency index mismatch"));
    }
    let limits = ProjectEvidenceLimits {
        max_documents: 12,
        max_document_bytes: 8 << 20,
        max_total_bytes: 16 << 20,
        max_json_depth: 32,
        max_decoded_fields: 1 << 20,
        max_string_bytes: 8192,
        max_locator_bytes: 160,
        max_locator_segments: 8,
        max_reference_records: 1024,
        max_import_records: 1024,
        max_reimport_states: 8,
    };
    let mut definitions = BTreeMap::new();
    for (shard_index, text) in shards.iter().enumerate() {
        let shard: Shard = parse(text)?;
        if shard.schema != "OTERYN_PROFICIENCY_SHARD/v1" || shard.family != "Proficiency" {
            return Err(invalid("proficiency shard mismatch"));
        }
        if shard.shard.index != shard_index
            || shard.shard.start != shard_index * 150
            || shard.shard.count != shard.records.len()
            || shard.shard.count == 0
            || shard.shard.start.checked_add(shard.shard.count - 1) != Some(shard.shard.end)
        {
            return Err(invalid("proficiency shard range/count mismatch"));
        }
        for record in shard.records {
            let mut definition = record.definition;
            if definition.identity.key
                != format!(
                    "oteryn:proficiency.tibia.p{}",
                    definition.source.proficiency_id
                )
                || definition.source.proficiency_id == 0
                || definition.source.version == 0
                || definition.identity.revision.is_empty()
                || definition.name.trim().is_empty()
                || definition.source.source_record_sha256.len() != 64
                || !definition
                    .source
                    .source_record_sha256
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            {
                return Err(invalid("proficiency identity/provenance mismatch"));
            }
            for level in definition
                .levels
                .as_array_mut()
                .ok_or_else(|| invalid("proficiency levels not array"))?
            {
                for perk in level["perks"]
                    .as_array_mut()
                    .ok_or_else(|| invalid("proficiency perks not array"))?
                {
                    for field in ["value", "probability", "multiplier"] {
                        if let Some(value) = perk.get(field).cloned() {
                            perk[field] = decimal_ratio(&value)?;
                        }
                    }
                }
            }
            let levels: Vec<ProjectV2ProficiencyLevel> = serde_json::from_value(definition.levels)
                .map_err(|error| ProjectError::InvalidJson(error.to_string()))?;
            validate_v2_proficiency_levels(&levels, limits)?;
            let imported = ImportedWeaponProficiency {
                identity: definition.identity,
                name: definition.name,
                source: definition.source,
                levels,
            };
            if definitions
                .insert(imported.identity.key.clone(), imported)
                .is_some()
            {
                return Err(invalid("duplicate proficiency definition"));
            }
        }
    }
    if definitions.len() != index.record_count {
        return Err(invalid("proficiency definition count mismatch"));
    }
    let input: Bindings = parse(bindings)?;
    if input.schema != "OTERYN_PROFICIENCY_ITEM_BINDINGS/v1" || !input.excluded.is_object() {
        return Err(invalid("proficiency bindings schema"));
    }
    let source_bytes = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tools/content-schema/item-authoring/samples/item-weapon-proficiency-15-30-7fea90ec.json"
    ));
    if input.source.path
        != "tools/content-schema/item-authoring/samples/item-weapon-proficiency-15-30-7fea90ec.json"
        || input.source.sha256 != crate::content::world_project_sha256(source_bytes)
    {
        return Err(invalid("proficiency binding source digest mismatch"));
    }
    let mut bindings = BTreeMap::new();
    for binding in input.records {
        let definition = definitions
            .get(&binding.profile_binding.key)
            .ok_or_else(|| invalid("unresolved proficiency binding"))?;
        let item_id = binding
            .item
            .key
            .strip_prefix("oteryn:item.tibia.i")
            .and_then(|id| id.parse::<u32>().ok())
            .filter(|id| *id > 0);
        if binding.item.family != ProjectV2Family::Item
            || binding.profile_binding.family != ProjectV2Family::Proficiency
            || binding.profile_binding.revision != definition.identity.revision
            || binding.item.revision.is_empty()
            || item_id.is_none_or(|id| binding.item.key != format!("oteryn:item.tibia.i{id}"))
        {
            return Err(invalid("proficiency binding identity/revision mismatch"));
        }
        if bindings.insert(binding.item.key.clone(), binding).is_some() {
            return Err(invalid("duplicate proficiency weapon binding"));
        }
    }
    if bindings.len() != input.record_count {
        return Err(invalid("proficiency binding count mismatch"));
    }
    let rules: Value = parse(rules)?;
    let progress_thresholds: BTreeMap<String, [u32; 9]> =
        serde_json::from_value(rules["levels"]["progress_thresholds"].clone())
            .map_err(|error| ProjectError::InvalidJson(error.to_string()))?;
    if rules["schema"] != "OTERYN_GAME_WEAPON_PROFICIENCY_PROGRESSION/v1"
        || rules["levels"]["mastery_offset"] != 2
        || rules["levels"]["perk_levels_max"] != 7
        || progress_thresholds.len() != 3
    {
        return Err(invalid("proficiency progression schema mismatch"));
    }
    for class in crate::domain::weapon_proficiency::ProficiencyThresholdClass::ALL {
        if progress_thresholds.get(class.as_str()) != Some(class.thresholds()) {
            return Err(invalid(
                "proficiency thresholds disagree with accepted domain rules",
            ));
        }
    }
    Ok(ImportedWeaponProficiencyData {
        definitions,
        bindings,
        progress_thresholds,
        mastery_offset: 2,
        binding_source: input.source,
        excluded: input.excluded,
        progression_ruleset: rules,
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    #[test]
    fn actual_full_catalogue_imports_and_resolves_every_weapon() {
        let data = import_committed_weapon_proficiency_data().expect("actual import");
        assert_eq!(data.definitions.len(), 443);
        assert_eq!(data.bindings.len(), 664);
        assert_eq!(
            data.definitions
                .values()
                .flat_map(|d| &d.levels)
                .map(|l| l.perks.len())
                .sum::<usize>(),
            3671
        );
        for binding in data.bindings.values() {
            assert_eq!(
                data.definitions[&binding.profile_binding.key]
                    .identity
                    .revision,
                binding.profile_binding.revision
            );
        }
        assert!(!data.bindings.contains_key("oteryn:item.tibia.i51666"));
        // Snowball 53855 has no admitted Item identity yet (PROF-SNOWBALL-REKEY-1).
        assert!(!data.bindings.contains_key("oteryn:item.tibia.i53855"));
        assert_eq!(
            data.definitions["oteryn:proficiency.tibia.p6"].levels[1].perks[0],
            ProjectV2ProficiencyPerk::AutoAttackCriticalExtraDamage {
                value: ProjectV2ExactRatio {
                    numerator: 1,
                    denominator: 10
                }
            }
        );
    }
    #[test]
    fn corrupted_shapes_perks_and_cross_references_refuse_import() {
        for mutation in 0..6 {
            let mut shard: Value = parse(SHARDS[0]).expect("shard");
            let mut bindings: Value = parse(BINDINGS).expect("bindings");
            match mutation {
                0 => shard["records"][0]["definition"]["levels"][0]["level"] = 2.into(),
                1 => {
                    shard["records"][0]["definition"]["levels"][0]["perks"][0]["kind"] =
                        "unmapped".into()
                }
                2 => {
                    let duplicate = shard["records"][0].clone();
                    shard["records"][1] = duplicate;
                }
                3 => bindings["records"][0]["profile_binding"]["revision"] = "wrong".into(),
                4 => bindings["records"][0]["profile_binding"]["key"] = "missing".into(),
                _ => {
                    let duplicate = bindings["records"][0].clone();
                    bindings["records"][1] = duplicate;
                }
            }
            let shard = shard.to_string();
            assert!(
                import(
                    INDEX,
                    &[&shard, SHARDS[1], SHARDS[2]],
                    &bindings.to_string(),
                    RULES
                )
                .is_err(),
                "mutation {mutation}"
            );
        }
    }
    #[test]
    fn index_counts_rules_and_malformed_decimals_refuse_import() {
        let mut index: Value = parse(INDEX).expect("index");
        index["record_count"] = 444.into();
        assert!(import(&index.to_string(), &SHARDS, BINDINGS, RULES).is_err());
        let mut rules: Value = parse(RULES).expect("rules");
        rules["levels"]["progress_thresholds"]["standard"][0] = 1751.into();
        assert!(import(INDEX, &SHARDS, BINDINGS, &rules.to_string()).is_err());
        let mut bindings: Value = parse(BINDINGS).expect("bindings");
        bindings["source"]["sha256"] = "0".repeat(64).into();
        assert!(import(INDEX, &SHARDS, &bindings.to_string(), RULES).is_err());
        let mut shard: Value = parse(SHARDS[0]).expect("shard");
        shard["shard"]["count"] = 149.into();
        assert!(
            import(
                INDEX,
                &[&shard.to_string(), SHARDS[1], SHARDS[2]],
                BINDINGS,
                RULES
            )
            .is_err()
        );
        assert!(decimal_ratio(&Value::String("0.1".into())).is_err());
        assert!(decimal_ratio(&serde_json::json!(1e100)).is_err());
        assert_eq!(
            decimal_ratio(&serde_json::json!(-1.5)).expect("signed"),
            serde_json::json!({"numerator": -3, "denominator": 2})
        );
    }
}
