//! Pinned Character progression content (ARCH-PROGRESSION-SOURCE-0 section 1.1-1.3).
//!
//! The `progression` section of the native gameplay pin is one canonical JSON document that
//! holds the decoded experience table, death policy, reward policy revision, declared
//! differences revision and the revisions that admission pins. It is decoded once, here,
//! under the canonical JSON profile; the runtime never reads the `rulesets/` files.
#![allow(
    dead_code,
    reason = "PROGRESSION-OWNER-1 consumes the decoded content; this packet only pins it"
)]
use super::digest::sha256;
use super::model::ContentError;
use crate::domain::progression::{
    FiniteProgressionPolicy, LevelThreshold, ProgressionRevisionContext, validate_policy,
};
use oteryn_simulation_determinism::{ExactI64, RoundingMode};
use serde::Deserialize;
use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fmt;

/// `N` of the death path and the kill reward path; a table of any other length is refused.
pub(crate) const CHARACTER_EXPERIENCE_TABLE_LEVELS: usize = 2000;
pub(crate) const PROGRESSION_SECTION_SCHEMA: &str = "OTERYN_NATIVE_PROGRESSION/v1";
pub(crate) const PROGRESSION_SIMULATION_REVISION: &str =
    "oteryn-simulation-determinism-exact-i64-v1";
/// Upper bound of the section inside the artifact (`1..=256 KiB`).
pub(crate) const MAX_PROGRESSION_SECTION_BYTES: usize = 256 * 1024;
const MAX_SAFE_INTEGER: u64 = (1 << 53) - 1;

const TABLE_SCHEMA: &str = "OTERYN_GAME_CHARACTER_EXPERIENCE_TABLE/v1";
const DEATH_SCHEMA: &str = "OTERYN_GAME_CHARACTER_DEATH_POLICY/v1";
const REWARD_SCHEMA: &str = "OTERYN_GAME_CHARACTER_REWARD_POLICY/v1";
const DIFFERENCES_SCHEMA: &str = "OTERYN_GAME_CHARACTER_PROGRESSION_DIFFERENCES/v1";
const PREFIX_TABLE: &str = "character-experience-v1-";
const PREFIX_DEATH: &str = "character-death-v1-";
const PREFIX_REWARD: &str = "character-reward-v1-";
const PREFIX_DIFFERENCES: &str = "character-progression-differences-v1-";
const PREFIX_POLICY: &str = "character-progression-policy-v1-";
const PREFIX_EVIDENCE: &str = "experience-table-evidence-20261005-";

fn invalid(reason: &'static str) -> ContentError {
    ContentError::InvalidArtifact(reason)
}

/// A JSON value under the canonical profile. Its `Deserialize` refuses a duplicate member
/// name (a map would keep only one of two equal names), a float, a negative number, `null`
/// and a non-ASCII member name while parsing, before any value is built.
#[derive(Debug)]
struct StrictJson(Value);

impl<'de> Deserialize<'de> for StrictJson {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Value;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a canonical-profile JSON value")
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Value, E> {
                Ok(Value::Bool(v))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Value, E> {
                if v > MAX_SAFE_INTEGER {
                    return Err(E::custom("integer above 2^53-1"));
                }
                Ok(Value::from(v))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Value, E> {
                u64::try_from(v)
                    .map_err(|_| E::custom("negative number"))
                    .and_then(|v| self.visit_u64(v))
            }
            fn visit_f64<E: de::Error>(self, _: f64) -> Result<Value, E> {
                Err(E::custom("float"))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Value, E> {
                Ok(Value::String(v.to_owned()))
            }
            fn visit_string<E: de::Error>(self, v: String) -> Result<Value, E> {
                Ok(Value::String(v))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Value, E> {
                Err(E::custom("null"))
            }
            fn visit_none<E: de::Error>(self) -> Result<Value, E> {
                Err(E::custom("null"))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
                let mut items = Vec::new();
                while let Some(StrictJson(item)) = seq.next_element()? {
                    items.push(item);
                }
                Ok(Value::Array(items))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
                let mut members = BTreeMap::new();
                while let Some(name) = map.next_key::<String>()? {
                    if !name.is_ascii() {
                        return Err(de::Error::custom("non-ASCII member name"));
                    }
                    let StrictJson(value) = map.next_value()?;
                    if members.insert(name, value).is_some() {
                        return Err(de::Error::custom("duplicate member name"));
                    }
                }
                Ok(Value::Object(members.into_iter().collect::<Map<_, _>>()))
            }
        }
        deserializer.deserialize_any(V).map(StrictJson)
    }
}

/// Parses one document under the canonical profile (shared with the four ruleset documents).
fn strict_parse(bytes: &[u8]) -> Result<Value, ContentError> {
    serde_json::from_slice::<StrictJson>(bytes)
        .map(|StrictJson(value)| value)
        .map_err(|_| invalid("progression canonical JSON profile"))
}

/// RFC 8785 bytes under the profile: sorted maps (no `preserve_order`), no whitespace, only
/// `"`, `\` and U+0000..U+001F escaped.
fn canonical(value: &Value) -> Result<Vec<u8>, ContentError> {
    serde_json::to_vec(value).map_err(|_| invalid("progression canonical encoding"))
}

fn sha32(bytes: &[u8]) -> String {
    sha256(bytes)
        .iter()
        .take(16)
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn valid_revision(value: &str) -> bool {
    let mut chars = value.chars();
    value.len() <= 128
        && chars.next().is_some_and(|c| c.is_ascii_alphanumeric())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | ':' | '-'))
}

/// `<prefix><sha256-32>` of the document with its own top-level `revision` removed.
fn document_revision(prefix: &str, document: &Value) -> Result<String, ContentError> {
    let mut without = document
        .as_object()
        .ok_or_else(|| invalid("progression document is not an object"))?
        .clone();
    without.remove("revision");
    Ok(format!(
        "{prefix}{}",
        sha32(&canonical(&Value::Object(without))?)
    ))
}

fn policy_revision(table_revision: &str, death_revision: &str) -> Result<String, ContentError> {
    let pair = serde_json::json!({
        "death_policy": death_revision,
        "experience_table": table_revision,
    });
    Ok(format!("{PREFIX_POLICY}{}", sha32(&canonical(&pair)?)))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Section {
    schema: String,
    experience_table: Value,
    death_policy: Value,
    reward_policy: Value,
    declared_differences: Value,
    revisions: StatedRevisions,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StatedRevisions {
    policy_revision: String,
    experience_table_revision: String,
    death_policy_revision: String,
    reward_revision: String,
    declaration: String,
    evidence: String,
    simulation: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TableDocument {
    schema: String,
    revision: String,
    evidence_revision: String,
    evidence_coverage: Coverage,
    levels: Vec<LevelRow>,
    terminal_exclusive_experience: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Coverage {
    first_level: u32,
    last_level: u32,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LevelRow {
    level: u32,
    minimum_experience: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeathDocument {
    schema: String,
    revision: String,
    loss_numerator: u64,
    loss_denominator: u64,
    rounding: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RewardDocument {
    schema: String,
    revision: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DifferencesDocument {
    schema: String,
    revision: String,
    records: Vec<DifferenceRecord>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DifferenceRecord {
    id: String,
    reference: String,
    oteryn: String,
}

fn typed<T: for<'de> Deserialize<'de>>(value: &Value) -> Result<T, ContentError> {
    T::deserialize(value).map_err(|_| invalid("progression document members"))
}

fn experience(text: &str) -> Result<ExactI64, ContentError> {
    let canonical_decimal = !text.is_empty()
        && text.bytes().all(|b| b.is_ascii_digit())
        && (text == "0" || !text.starts_with('0'));
    let value = text
        .parse::<i64>()
        .ok()
        .filter(|_| canonical_decimal)
        .ok_or_else(|| invalid("progression experience value"))?;
    Ok(ExactI64::new(value))
}

/// The decoded, immutable Character progression content of one World pin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CharacterProgressionContent {
    policy_revision: String,
    experience_table_revision: String,
    death_policy_revision: String,
    reward_revision: String,
    declaration: String,
    evidence: String,
    simulation: String,
    thresholds: Vec<LevelThreshold>,
    terminal_exclusive_experience: ExactI64,
    evidence_last_level: u32,
    difference_ids: Vec<String>,
}

impl CharacterProgressionContent {
    pub(crate) fn policy_revision(&self) -> &str {
        &self.policy_revision
    }
    pub(crate) fn experience_table_revision(&self) -> &str {
        &self.experience_table_revision
    }
    pub(crate) fn death_policy_revision(&self) -> &str {
        &self.death_policy_revision
    }
    pub(crate) fn reward_revision(&self) -> &str {
        &self.reward_revision
    }
    pub(crate) fn declaration(&self) -> &str {
        &self.declaration
    }
    pub(crate) fn evidence(&self) -> &str {
        &self.evidence
    }
    pub(crate) fn simulation(&self) -> &str {
        &self.simulation
    }
    pub(crate) fn evidence_last_level(&self) -> u32 {
        self.evidence_last_level
    }
    pub(crate) fn difference_ids(&self) -> &[String] {
        &self.difference_ids
    }

    /// The policy with the three Character-root revisions of one admission.
    pub(crate) fn policy_for(
        &self,
        profile: &str,
        ruleset: &str,
        content: &str,
    ) -> Result<FiniteProgressionPolicy<String, CHARACTER_EXPERIENCE_TABLE_LEVELS>, ContentError>
    {
        let thresholds: [LevelThreshold; CHARACTER_EXPERIENCE_TABLE_LEVELS] = self
            .thresholds
            .clone()
            .try_into()
            .map_err(|_| invalid("progression table length"))?;
        Ok(FiniteProgressionPolicy {
            context: ProgressionRevisionContext {
                profile: profile.to_owned(),
                ruleset: ruleset.to_owned(),
                content: content.to_owned(),
                simulation: self.simulation.clone(),
                evidence: self.evidence.clone(),
                declaration: self.declaration.clone(),
            },
            policy_revision: self.policy_revision.clone(),
            reward_revision: self.reward_revision.clone(),
            death_policy_revision: self.death_policy_revision.clone(),
            declared_difference_revision: self.declaration.clone(),
            thresholds,
            terminal_exclusive_experience: self.terminal_exclusive_experience,
            death_loss_numerator: 1,
            death_loss_denominator: 1,
            death_loss_rounding: RoundingMode::Floor,
        })
    }

    /// Strictly decodes the section bytes of a native gameplay pin. Any malformed, tampered
    /// or non-canonical section refuses the manifest.
    pub(crate) fn decode(bytes: &[u8]) -> Result<Self, ContentError> {
        if bytes.is_empty() || bytes.len() > MAX_PROGRESSION_SECTION_BYTES {
            return Err(invalid("progression section bounds"));
        }
        let value = strict_parse(bytes)?;
        if canonical(&value)? != bytes {
            return Err(invalid("progression section is not canonical"));
        }
        let section: Section = typed(&value)?;
        if section.schema != PROGRESSION_SECTION_SCHEMA {
            return Err(invalid("progression section schema"));
        }
        let stated = &section.revisions;
        let table: TableDocument = typed(&section.experience_table)?;
        let death: DeathDocument = typed(&section.death_policy)?;
        let reward: RewardDocument = typed(&section.reward_policy)?;
        let differences: DifferencesDocument = typed(&section.declared_differences)?;
        // Each document states its own revision; the section copies it and nothing else.
        for (prefix, document, own) in [
            (PREFIX_TABLE, &section.experience_table, &table.revision),
            (PREFIX_DEATH, &section.death_policy, &death.revision),
            (PREFIX_REWARD, &section.reward_policy, &reward.revision),
            (
                PREFIX_DIFFERENCES,
                &section.declared_differences,
                &differences.revision,
            ),
        ] {
            if document_revision(prefix, document)? != *own {
                return Err(invalid("progression document revision"));
            }
        }
        if table.schema != TABLE_SCHEMA
            || death.schema != DEATH_SCHEMA
            || reward.schema != REWARD_SCHEMA
            || differences.schema != DIFFERENCES_SCHEMA
            || differences.records.is_empty()
            || differences
                .records
                .iter()
                .any(|r| r.id.is_empty() || r.reference.is_empty() || r.oteryn.is_empty())
        {
            return Err(invalid("progression document schema"));
        }
        if death.loss_numerator != 1 || death.loss_denominator != 1 || death.rounding != "floor" {
            return Err(invalid("progression death policy"));
        }
        if !table
            .evidence_revision
            .strip_prefix(PREFIX_EVIDENCE)
            .is_some_and(|digest| {
                digest.len() == 32
                    && digest
                        .bytes()
                        .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
            })
            || table.evidence_coverage.first_level != 1
            || !(1..=CHARACTER_EXPERIENCE_TABLE_LEVELS as u32)
                .contains(&table.evidence_coverage.last_level)
            || table.levels.len() != CHARACTER_EXPERIENCE_TABLE_LEVELS
        {
            return Err(invalid("progression experience table shape"));
        }
        let mut thresholds = Vec::with_capacity(CHARACTER_EXPERIENCE_TABLE_LEVELS);
        for (index, row) in table.levels.iter().enumerate() {
            if row.level as usize != index + 1 {
                return Err(invalid("progression table levels"));
            }
            thresholds.push(LevelThreshold {
                level: row.level,
                minimum_experience: experience(&row.minimum_experience)?,
            });
        }
        if stated.experience_table_revision != table.revision
            || stated.death_policy_revision != death.revision
            || stated.reward_revision != reward.revision
            || stated.declaration != differences.revision
            || stated.evidence != table.evidence_revision
            || stated.simulation != PROGRESSION_SIMULATION_REVISION
            || stated.policy_revision != policy_revision(&table.revision, &death.revision)?
        {
            return Err(invalid("progression stated revisions"));
        }
        for revision in [
            &stated.policy_revision,
            &stated.experience_table_revision,
            &stated.death_policy_revision,
            &stated.reward_revision,
            &stated.declaration,
            &stated.evidence,
            &stated.simulation,
        ] {
            if !valid_revision(revision) {
                return Err(invalid("progression revision syntax"));
            }
        }
        let content = Self {
            policy_revision: stated.policy_revision.clone(),
            experience_table_revision: stated.experience_table_revision.clone(),
            death_policy_revision: stated.death_policy_revision.clone(),
            reward_revision: stated.reward_revision.clone(),
            declaration: stated.declaration.clone(),
            evidence: stated.evidence.clone(),
            simulation: stated.simulation.clone(),
            thresholds,
            terminal_exclusive_experience: experience(&table.terminal_exclusive_experience)?,
            evidence_last_level: table.evidence_coverage.last_level,
            difference_ids: differences.records.into_iter().map(|r| r.id).collect(),
        };
        // The same checks as `validate_policy`, on a template context.
        let template = content.policy_for("template", "template", "template")?;
        validate_policy(&template).map_err(|_| invalid("progression policy validation"))?;
        Ok(content)
    }
}

#[cfg(test)]
#[path = "character_progression_content_tests.rs"]
pub(crate) mod tests;
