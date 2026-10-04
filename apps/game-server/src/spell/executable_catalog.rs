//! Complete candidate compiler input, bound to an artifact digest by the caller.
//!
//! Digest equality proves which bytes were compiled; it is not current Channel
//! authority or activation. All 246 identities survive compilation. Runtime
//! representability and contradictory spoken aliases are explicit results, and
//! a book cannot be built by silently skipping either kind of unresolved row.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::authoring::{self, AuthoringError};
use super::native::{self, CompiledNativeSpell};
use super::{SpellBook, SpellDefinition, Vocation};

const SCHEMA: &str = "OTERYN_EXECUTABLE_SPELL_CATALOG/v1";
const REVISION: &str = "spell-p2-r20";
const MAX_ARTIFACT_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Error(pub(crate) String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub(crate) struct Identity {
    pub(crate) key: String,
    pub(crate) revision: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub(crate) struct DefinitionRef {
    pub(crate) family: String,
    pub(crate) key: String,
    pub(crate) revision: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Requirements {
    pub(crate) vocations: Vec<String>,
    pub(crate) level: u32,
    pub(crate) premium: bool,
    pub(crate) learning_required: bool,
    pub(crate) wheel_unlock: Option<bool>,
}
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Costs {
    pub(crate) mana: Option<u32>,
    pub(crate) mana_percent: Option<u32>,
    pub(crate) soul: u32,
}
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Group {
    pub(crate) group: String,
    pub(crate) cooldown_ms: u32,
}
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Targeting {
    pub(crate) aggressive: bool,
    pub(crate) self_target: bool,
    pub(crate) needs_target: bool,
    pub(crate) needs_direction: bool,
    pub(crate) target_or_direction: Option<bool>,
    pub(crate) block_walls: bool,
    pub(crate) allow_on_self: bool,
    pub(crate) check_floor: bool,
    pub(crate) parameter: String,
    pub(crate) range_tiles: Option<u32>,
    pub(crate) aim_at_target: Option<bool>,
    pub(crate) cast_at_position: Option<bool>,
    pub(crate) allowed_targets: Option<String>,
}
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Blocking {
    pub(crate) solid: bool,
    pub(crate) creature: bool,
}
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Rune {
    pub(crate) item: DefinitionRef,
    pub(crate) charges: u32,
    pub(crate) magic_level: u32,
    pub(crate) allow_far_use: bool,
    pub(crate) blocking: Blocking,
}
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct CastPresentation {
    pub(crate) cast_cue: Option<String>,
    pub(crate) impact_cue: Option<String>,
}
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Conjure {
    pub(crate) reagent: Option<DefinitionRef>,
    pub(crate) result: DefinitionRef,
    pub(crate) count: u32,
    pub(crate) effect_asset_binding: Option<String>,
}
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct NativeBehavior {
    pub(crate) key: String,
    pub(crate) parameters: Value,
}
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct PartyParameters {
    pub(crate) area: Vec<String>,
    pub(crate) same_floor: bool,
    pub(crate) min_affected: u32,
    pub(crate) requires_party: bool,
    pub(crate) mana: PartyManaProfile,
    pub(crate) effect: DefinitionRef,
}
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum PartyManaProfile {
    Fixed {
        base: u32,
    },
    Scaled {
        base: u32,
        falloff: f64,
        rounding: String,
    },
}
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct AuthoredExecution {
    pub(crate) ability: Option<DefinitionRef>,
    pub(crate) conjure: Option<Conjure>,
    pub(crate) native_behavior: Option<NativeBehavior>,
}
/// Every authored operational cast field is retained, including guards that the
/// current owner cannot yet execute. No default silently weakens those guards.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct AuthoredSpell {
    pub(crate) identity: Identity,
    pub(crate) name: String,
    pub(crate) carrier: String,
    pub(crate) words: Option<String>,
    pub(crate) reference_spell_id: Option<u32>,
    pub(crate) library_text: Option<String>,
    pub(crate) requirements: Requirements,
    pub(crate) costs: Costs,
    pub(crate) cooldown_ms: u32,
    pub(crate) groups: Vec<Group>,
    pub(crate) targeting: Targeting,
    pub(crate) pz_locks_caster: bool,
    pub(crate) needs_weapon: bool,
    pub(crate) needs_shield: Option<bool>,
    pub(crate) presentation: Option<CastPresentation>,
    pub(crate) base_power: Option<i64>,
    pub(crate) execution: AuthoredExecution,
    pub(crate) rune: Option<Rune>,
    pub(crate) harmony_role: Option<String>,
}
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Bundle {
    spell: AuthoredSpell,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Area {
    pub(crate) matrix: OrientedMatrix,
}
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct OrientedMatrix {
    pub(crate) north: Vec<String>,
    pub(crate) diagonal: Option<Vec<String>>,
}
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Chain {
    pub(crate) max_targets: u32,
    pub(crate) range_tiles: u32,
    pub(crate) backtracking: bool,
    pub(crate) chain_asset_binding: Option<String>,
    pub(crate) damage_step_percent: Option<i32>,
    pub(crate) initial_range_tiles: Option<u32>,
    pub(crate) shape: Option<String>,
}
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct AbilityProfile {
    pub(crate) identity: Identity,
    pub(crate) kind: String,
    pub(crate) range_tiles: u32,
    pub(crate) needs_target: bool,
    pub(crate) needs_direction: bool,
    #[serde(default)]
    pub(crate) effects: Vec<DefinitionRef>,
    pub(crate) variants: Option<Vec<DefinitionRef>>,
    pub(crate) area: Option<Area>,
    pub(crate) chain: Option<Chain>,
    pub(crate) target_selection: Option<String>,
    pub(crate) zero_damage_health_path: Option<bool>,
}
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct EffectPresentation {
    pub(crate) impact_asset_binding: Option<String>,
    pub(crate) projectile_asset_binding: Option<String>,
    pub(crate) caster_effect_asset_binding: Option<String>,
    pub(crate) caster_effect_timing: Option<String>,
}
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct AttributeModifier {
    pub(crate) attribute: String,
    pub(crate) mode: String,
    pub(crate) value: i64,
}
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Regeneration {
    pub(crate) health_gain: Option<u32>,
    pub(crate) health_interval_ms: Option<u32>,
    pub(crate) mana_gain: Option<u32>,
    pub(crate) mana_interval_ms: Option<u32>,
}
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Light {
    pub(crate) level: u32,
    pub(crate) color: u32,
}
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct FixedTick {
    pub(crate) count: u32,
    pub(crate) interval_ms: u32,
    pub(crate) amount: i64,
}
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct DamageOverTime {
    pub(crate) tick_profile: String,
    pub(crate) first_tick: String,
    pub(crate) fixed_ticks: Vec<FixedTick>,
}
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConditionProfile {
    #[serde(rename = "type")]
    pub(crate) kind: String,
    pub(crate) lifetime: String,
    pub(crate) buff_spell: Option<bool>,
    pub(crate) attribute_modifiers: Option<Vec<AttributeModifier>>,
    pub(crate) regeneration: Option<Regeneration>,
    pub(crate) light: Option<Light>,
    pub(crate) speed_formula: Option<DefinitionRef>,
    pub(crate) damage_over_time: Option<DamageOverTime>,
}
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct DurationRange {
    pub(crate) minimum: u32,
    pub(crate) maximum: u32,
}
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct EffectProfile {
    pub(crate) identity: Identity,
    pub(crate) operation: String,
    pub(crate) damage_type: Option<String>,
    pub(crate) formula: Option<DefinitionRef>,
    pub(crate) condition: Option<ConditionProfile>,
    pub(crate) presentation: Option<EffectPresentation>,
    pub(crate) mitigated_by: Option<Vec<String>>,
    pub(crate) removed_condition: Option<String>,
    pub(crate) duration_ms: Option<u32>,
    pub(crate) created_item: Option<DefinitionRef>,
    pub(crate) pvp_safe_item: Option<DefinitionRef>,
    pub(crate) duration_range_ms: Option<DurationRange>,
    pub(crate) duration_selection: Option<String>,
    pub(crate) safe_world_type: Option<String>,
    pub(crate) refuse_on: Option<Vec<String>>,
    pub(crate) description_template: Option<String>,
}
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Fraction {
    pub(crate) numerator: i64,
    pub(crate) denominator: u64,
}
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct SpeedProfile {
    pub(crate) minimum_multiplier: Fraction,
    pub(crate) minimum_offset: i64,
    pub(crate) maximum_multiplier: Fraction,
    pub(crate) maximum_offset: i64,
}
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct FormulaProfile {
    pub(crate) identity: Identity,
    pub(crate) kind: String,
    pub(crate) inputs: Option<String>,
    pub(crate) minimum: Option<Value>,
    pub(crate) maximum: Option<Value>,
    pub(crate) speed: Option<SpeedProfile>,
}
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct DependencyProfile {
    pub(crate) abilities: Vec<AbilityProfile>,
    pub(crate) effects: Vec<EffectProfile>,
    pub(crate) formulas: Vec<FormulaProfile>,
}
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OperationalProfile {
    pub(crate) header: AuthoredSpell,
    pub(crate) dependencies: DependencyProfile,
    pub(crate) ability: Option<AbilityProfile>,
}

/// The common reader uses the same closed AST as the artifact compiler. Local
/// Ability/Effect/Formula references resolve by family, key AND revision; item
/// membership is separately bound by the outer compiler's catalog scope.
pub(crate) fn profile_from_bundle(
    bundle: &Value,
    dependencies: &Value,
) -> Result<OperationalProfile, Error> {
    nonnullable_profile(bundle, false)?;
    nonnullable_profile(dependencies, false)?;
    let parsed_bundle: Bundle = parsed(bundle.clone())?;
    let deps: DependencyProfile = parsed(dependencies.clone())?;
    validate_profile(&parsed_bundle.spell, &deps)?;
    let mut scope = BTreeSet::new();
    for (family, rows) in [
        ("Ability", &dependencies["abilities"]),
        ("Effect", &dependencies["effects"]),
        ("Formula", &dependencies["formulas"]),
    ] {
        for row in rows
            .as_array()
            .ok_or_else(|| Error("missing local dependency family".into()))?
        {
            let id: Identity = parsed(row["identity"].clone())?;
            require(
                scope.insert(DefinitionRef {
                    family: family.into(),
                    key: id.key,
                    revision: id.revision,
                }),
                "duplicate local identity",
            )?;
        }
    }
    let mut refs = Vec::new();
    references(bundle, &mut refs)?;
    references(dependencies, &mut refs)?;
    for reference in refs {
        if matches!(reference.family.as_str(), "Ability" | "Effect" | "Formula") {
            require(
                scope.contains(&reference),
                "unresolved exact local operational reference",
            )?;
        }
    }
    let ability = parsed_bundle
        .spell
        .execution
        .ability
        .as_ref()
        .map(|reference| {
            deps.abilities
                .iter()
                .find(|profile| {
                    profile.identity.key == reference.key
                        && profile.identity.revision == reference.revision
                })
                .cloned()
                .ok_or_else(|| Error("unresolved exact execution Ability".into()))
        })
        .transpose()?;
    Ok(OperationalProfile {
        header: parsed_bundle.spell,
        dependencies: deps,
        ability,
    })
}
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct CatalogRefs {
    definitions: Vec<DefinitionRef>,
}
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct SourceIdentity {
    pub(crate) bundle_id: String,
    pub(crate) identity: Identity,
    pub(crate) manifest_sha256: String,
    pub(crate) sources: Vec<Value>,
}
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct RemovedIdentity {
    pub(crate) name: String,
    pub(crate) carrier: String,
    pub(crate) policy: String,
    pub(crate) evidence: String,
    pub(crate) source_identity: SourceIdentity,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InputEntry {
    bundle: Value,
    dependencies: Value,
    catalog: CatalogRefs,
    manifest: Value,
    source_identities: Vec<SourceIdentity>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Artifact {
    schema: String,
    revision: String,
    bundles: Vec<InputEntry>,
    removed: Vec<RemovedIdentity>,
}

#[derive(Debug, Clone)]
pub(crate) struct CompiledEntry {
    pub(crate) header: AuthoredSpell,
    pub(crate) dependencies: DependencyProfile,
    pub(crate) definition: Result<SpellDefinition, AuthoringError>,
    pub(crate) native: Option<CompiledNativeSpell>,
    pub(crate) party: Option<PartyParameters>,
    pub(crate) bundle: Value,
    pub(crate) dependency_payload: Value,
    pub(crate) manifest: Value,
    pub(crate) source_identities: Vec<SourceIdentity>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AliasGroup {
    pub(crate) words: String,
    pub(crate) identities: Vec<Identity>,
    /// True only when every complete header/body/dependency differs solely in
    /// the spell's display name and identity. Unequal familiar lifetimes fail.
    pub(crate) equivalent: bool,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceProof {
    repository: String,
    revision: String,
    path: String,
    sha256: String,
    url: String,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SelectionRow {
    words: String,
    selected: Identity,
    alternatives: Vec<Identity>,
    policy: String,
    source_proofs: Vec<SourceProof>,
    differences: Value,
    notes: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SelectionArtifact {
    schema: String,
    revision: String,
    catalog_sha256: String,
    selections: Vec<SelectionRow>,
}

/// Source policy is a separately pinned compiler artifact, not a runtime
/// admission override. It only selects a member of a captured ambiguous group.
pub(crate) fn compile_source_selection(
    bytes: &[u8],
    expected_digest: &str,
    catalog: &CompiledCatalog,
) -> Result<BTreeMap<String, Identity>, Error> {
    require(
        bytes.len() <= 256 * 1024 && is_digest(expected_digest) && digest(bytes) == expected_digest,
        "source-selection digest mismatch",
    )?;
    let artifact: SelectionArtifact = parsed(unique_document(bytes)?)?;
    require(
        artifact.schema == "OTERYN_SPELL_SOURCE_SELECTION/v1"
            && artifact.revision == REVISION
            && artifact.catalog_sha256 == catalog.artifact_digest
            && artifact.selections.len() == 4,
        "source-selection catalog binding mismatch",
    )?;
    let mut selections = BTreeMap::new();
    for row in artifact.selections {
        let group = catalog
            .aliases
            .iter()
            .find(|group| group.words == row.words && !group.equivalent)
            .ok_or_else(|| Error("selection is not an ambiguous captured group".into()))?;
        require(
            group.identities.contains(&row.selected)
                && row
                    .alternatives
                    .iter()
                    .all(|id| group.identities.contains(id) && id != &row.selected)
                && row.alternatives.len() + 1 == group.identities.len(),
            "substituted source selection",
        )?;
        let selected = catalog
            .entries
            .iter()
            .find(|entry| entry.header.identity == row.selected)
            .ok_or_else(|| Error("selected profile missing".into()))?;
        let parameters = &selected.bundle["spell"]["execution"]["native_behavior"]["parameters"];
        let vocation = parameters["vocation"]
            .as_str()
            .ok_or_else(|| Error("selection is not a familiar profile".into()))?;
        require(
            row.selected.key == format!("candidate:spell/{vocation}_familiar")
                && row.policy == "S21"
                && !row.notes.is_empty()
                && row.differences["login"]["selected"] == parameters["login"]
                && row.differences["party_protection_registration"]["selected"]
                    == parameters["party_protection_registration"],
            "wrong policy or selected familiar semantics",
        )?;
        let alternative = catalog
            .entries
            .iter()
            .find(|entry| row.alternatives.contains(&entry.header.identity))
            .ok_or_else(|| Error("alternative profile missing".into()))?;
        let other = &alternative.bundle["spell"]["execution"]["native_behavior"]["parameters"];
        require(
            row.differences["login"]["alternative"] == other["login"]
                && row.differences["party_protection_registration"]["alternative"]
                    == other["party_protection_registration"],
            "source variant evidence mismatch",
        )?;
        let mut proofs = BTreeSet::new();
        for proof in &row.source_proofs {
            let revision = match proof.repository.as_str() {
                "opentibiabr/canary" => "99902524e052f37574194466c2949c576e4ab269",
                "zimbadev/crystalserver" => "ff7ede593c69d4c658b382c97443e8155926924a",
                _ => return Err(Error("unqualified source repository".into())),
            };
            require(
                proof.revision == revision
                    && is_digest(&proof.sha256)
                    && proof.url
                        == format!(
                            "https://github.com/{}/blob/{}/{}",
                            proof.repository, revision, proof.path
                        )
                    && [
                        format!("data/scripts/spells/familiar/{vocation}_familiar.lua"),
                        "data/libs/systems/familiar.lua".into(),
                        "data/scripts/creaturescripts/familiar/on_login.lua".into(),
                    ]
                    .contains(&proof.path)
                    && proofs.insert((proof.repository.clone(), proof.path.clone())),
                "unqualified or duplicate source proof",
            )?;
        }
        require(
            proofs.len() == 6 && selections.insert(row.words, row.selected).is_none(),
            "incomplete source-policy coverage",
        )?;
    }
    Ok(selections)
}
#[derive(Debug, Clone)]
pub(crate) struct CompiledCatalog {
    artifact_digest: String,
    pub(crate) entries: Vec<CompiledEntry>,
    pub(crate) aliases: Vec<AliasGroup>,
    pub(crate) removed: Vec<RemovedIdentity>,
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn is_digest(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn require(ok: bool, message: impl Into<String>) -> Result<(), Error> {
    if ok {
        Ok(())
    } else {
        Err(Error(message.into()))
    }
}
fn parsed<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, Error> {
    serde_json::from_value(value).map_err(|e| Error(e.to_string()))
}

struct UniqueJson(Value);
impl<'de> Deserialize<'de> for UniqueJson {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = UniqueJson;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("JSON with unique object properties")
            }
            fn visit_bool<E: serde::de::Error>(self, value: bool) -> Result<Self::Value, E> {
                Ok(UniqueJson(Value::Bool(value)))
            }
            fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Ok(UniqueJson(Value::from(value)))
            }
            fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(UniqueJson(Value::from(value)))
            }
            fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Self::Value, E> {
                serde_json::Number::from_f64(value)
                    .map(|number| UniqueJson(Value::Number(number)))
                    .ok_or_else(|| E::custom("nonfinite JSON number"))
            }
            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(UniqueJson(Value::String(value.into())))
            }
            fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Self::Value, E> {
                Ok(UniqueJson(Value::String(value)))
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(UniqueJson(Value::Null))
            }
            fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(UniqueJson(Value::Null))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut access: A,
            ) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(value) = access.next_element::<UniqueJson>()? {
                    values.push(value.0);
                }
                Ok(UniqueJson(Value::Array(values)))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut access: A,
            ) -> Result<Self::Value, A::Error> {
                let mut object = serde_json::Map::new();
                while let Some(key) = access.next_key::<String>()? {
                    if object.contains_key(&key) {
                        return Err(serde::de::Error::custom(format!(
                            "duplicate JSON property {key}"
                        )));
                    }
                    object.insert(key, access.next_value::<UniqueJson>()?.0);
                }
                Ok(UniqueJson(Value::Object(object)))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}
fn unique_document(bytes: &[u8]) -> Result<Value, Error> {
    serde_json::from_slice::<UniqueJson>(bytes)
        .map(|value| value.0)
        .map_err(|error| Error(error.to_string()))
}

fn valid_revision(revision: &str) -> bool {
    !revision.is_empty()
        && revision.len() <= 128
        && revision.bytes().all(|byte| byte.is_ascii_graphic())
}
fn validate_identity(identity: &Identity) -> Result<(), Error> {
    require(
        !identity.key.is_empty() && valid_revision(&identity.revision),
        "invalid exact identity",
    )
}
fn validate_ref(reference: &DefinitionRef) -> Result<(), Error> {
    require(
        valid_revision(&reference.revision) && !reference.key.is_empty(),
        "invalid exact reference",
    )?;
    require(
        matches!(
            reference.family.as_str(),
            "Ability" | "Effect" | "Formula" | "Item" | "Creature"
        ),
        "unknown reference family",
    )?;
    if reference.family == "Item" {
        // Candidate references are not production identities. Their numeric
        // identity nevertheless must be explicit, canonical and positive.
        let suffix = reference
            .key
            .strip_prefix("candidate:item/")
            .or_else(|| reference.key.strip_prefix("oteryn:item.tibia.i"))
            .ok_or_else(|| Error("invalid numeric item reference".into()))?;
        let id = suffix
            .parse::<u32>()
            .map_err(|_| Error("invalid numeric item reference".into()))?;
        require(
            id > 0 && suffix == id.to_string(),
            "noncanonical or zero item id",
        )?;
    }
    Ok(())
}
fn references(value: &Value, output: &mut Vec<DefinitionRef>) -> Result<(), Error> {
    match value {
        Value::Object(object) => {
            if object.contains_key("family") {
                let reference: DefinitionRef = parsed(value.clone())?;
                validate_ref(&reference)?;
                output.push(reference);
            } else {
                for child in object.values() {
                    references(child, output)?;
                }
            }
        }
        Value::Array(values) => {
            for child in values {
                references(child, output)?;
            }
        }
        _ => {}
    }
    Ok(())
}
fn nonnullable_profile(value: &Value, native_parameters: bool) -> Result<(), Error> {
    match value {
        Value::Null if !native_parameters => {
            return Err(Error(
                "null is not an authored optional field; omit it explicitly".into(),
            ));
        }
        Value::Object(object) => {
            for (key, child) in object {
                // Native parameter nulls are explicitly part of some qualified
                // recipes; their complete exact profile comparison owns them.
                nonnullable_profile(
                    child,
                    native_parameters || (key == "parameters" && object.contains_key("key")),
                )?;
            }
        }
        Value::Array(children) => {
            for child in children {
                nonnullable_profile(child, native_parameters)?;
            }
        }
        _ => {}
    }
    Ok(())
}
fn expression(value: &Value, depth: usize) -> Result<(), Error> {
    require(
        depth <= super::formula::MAX_EXPRESSION_DEPTH,
        "formula depth exceeds limit",
    )?;
    let object = value
        .as_object()
        .ok_or_else(|| Error("expression is not an object".into()))?;
    if object.contains_key("const") {
        require(
            object.len() == 1
                && object["const"]
                    .as_str()
                    .is_some_and(|s| s.parse::<f64>().is_ok_and(f64::is_finite)),
            "invalid constant expression",
        )?;
    } else if object.contains_key("var") {
        require(
            object.len() == 1 && object["var"].is_string(),
            "invalid input expression",
        )?;
    } else {
        require(
            object.len() == 2
                && object.contains_key("args")
                && (object.contains_key("op") ^ object.contains_key("fn")),
            "invalid expression properties",
        )?;
        for child in object["args"]
            .as_array()
            .ok_or_else(|| Error("expression args missing".into()))?
        {
            expression(child, depth + 1)?;
        }
    }
    authoring::expression(value, depth).map_err(|e| Error(e.to_string()))?;
    Ok(())
}
fn matrix(rows: &[String]) -> Result<(), Error> {
    require(!rows.is_empty() && !rows[0].is_empty(), "empty area matrix")?;
    let mut centres = 0;
    for row in rows {
        require(row.len() == rows[0].len(), "nonrectangular area matrix")?;
        for cell in row.bytes() {
            require(
                matches!(cell, b'.' | b'x' | b'c' | b'C'),
                "unknown area cell",
            )?;
            if matches!(cell, b'c' | b'C') {
                centres += 1;
            }
        }
    }
    require(centres == 1, "area must contain exactly one centre")
}
fn validate_profile(header: &AuthoredSpell, deps: &DependencyProfile) -> Result<(), Error> {
    validate_identity(&header.identity)?;
    require(!header.name.is_empty(), "empty spell name")?;
    require(
        matches!(
            header.targeting.parameter.as_str(),
            "none" | "text" | "player_name"
        ),
        "unknown cast parameter",
    )?;
    require(
        header
            .targeting
            .allowed_targets
            .as_deref()
            .is_none_or(|key| super::target::AllowedTargets::from_key(key).is_some()),
        "unknown allowed target rule",
    )?;
    require(
        header
            .harmony_role
            .as_deref()
            .is_none_or(|role| matches!(role, "builder" | "spender")),
        "unknown harmony_role",
    )?;
    require(
        matches!((header.carrier.as_str(), &header.words, &header.rune),
        ("instant", Some(words), None) if !words.trim().is_empty())
            || matches!(
                (header.carrier.as_str(), &header.words, &header.rune),
                ("rune", None, Some(_))
            ),
        "invalid spell carrier",
    )?;
    require(
        !header.requirements.vocations.is_empty()
            && header
                .requirements
                .vocations
                .iter()
                .all(|v| Vocation::from_key(v).is_some()),
        "unknown or empty vocation set",
    )?;
    require(
        header
            .requirements
            .vocations
            .iter()
            .collect::<BTreeSet<_>>()
            .len()
            == header.requirements.vocations.len(),
        "duplicate vocation",
    )?;
    require(
        header.costs.mana.is_some() ^ header.costs.mana_percent.is_some(),
        "exactly one mana mode required",
    )?;
    require(
        header.costs.mana_percent.is_none_or(|v| v <= 100),
        "invalid mana percentage",
    )?;
    let execution = &header.execution;
    require(
        usize::from(execution.ability.is_some())
            + usize::from(execution.conjure.is_some())
            + usize::from(execution.native_behavior.is_some())
            == 1,
        "exactly one execution required",
    )?;
    if let Some(conjure) = &execution.conjure {
        require(
            conjure.count > 0
                && conjure.result.family == "Item"
                && conjure.reagent.as_ref().is_none_or(|r| r.family == "Item"),
            "invalid conjure",
        )?;
    }
    if let Some(rune) = &header.rune {
        require(
            rune.charges > 0 && rune.item.family == "Item",
            "invalid rune",
        )?;
    }
    if let Some(ability) = &execution.ability {
        require(
            ability.family == "Ability",
            "wrong execution reference family",
        )?;
    }
    for ability in &deps.abilities {
        validate_identity(&ability.identity)?;
        require(
            ability.kind == "spell"
                && ((!ability.effects.is_empty() && ability.variants.is_none())
                    || (ability.effects.is_empty()
                        && ability.variants.as_ref().is_some_and(|variants| {
                            variants.len() >= 2
                                && variants
                                    .iter()
                                    .all(|reference| reference.family == "Ability")
                        })
                        && ability.area.is_none()
                        && ability.chain.is_none()))
                && ability.effects.iter().all(|r| r.family == "Effect"),
            "invalid spell ability",
        )?;
        if let Some(area) = &ability.area {
            matrix(&area.matrix.north)?;
            if let Some(diagonal) = &area.matrix.diagonal {
                matrix(diagonal)?;
            }
        }
        if let Some(chain) = &ability.chain {
            require(
                chain.max_targets > 0
                    && chain.range_tiles > 0
                    && chain.initial_range_tiles.is_none_or(|n| n > 0)
                    && chain
                        .damage_step_percent
                        .is_none_or(|n| (-100..=100).contains(&n))
                    && chain
                        .shape
                        .as_deref()
                        .is_none_or(|s| matches!(s, "sequential" | "fork")),
                "invalid chain",
            )?;
        }
        require(
            ability
                .target_selection
                .as_deref()
                .is_none_or(|s| s == "caster_or_top_creature"),
            "unknown target selector",
        )?;
    }
    for effect in &deps.effects {
        validate_identity(&effect.identity)?;
        let valid = match effect.operation.as_str() {
            "damage" | "heal" => {
                effect
                    .formula
                    .as_ref()
                    .is_some_and(|r| r.family == "Formula")
                    && effect.damage_type.is_some()
            }
            "condition" => effect.condition.is_some(),
            "create_item" => effect
                .created_item
                .as_ref()
                .is_some_and(|r| r.family == "Item"),
            "remove_condition" => effect.removed_condition.is_some(),
            "presentation_only" => effect.presentation.is_some(),
            _ => false,
        };
        require(
            valid,
            format!("invalid effect operation {}", effect.operation),
        )?;
        require(
            effect.damage_type.as_deref().is_none_or(|kind| {
                matches!(
                    kind,
                    "physical" | "holy" | "earth" | "fire" | "ice" | "death" | "energy" | "healing"
                )
            }),
            "unknown damage type",
        )?;
        if let Some(condition) = &effect.condition {
            require(
                matches!(
                    condition.kind.as_str(),
                    "regeneration"
                        | "attributes"
                        | "fire"
                        | "poison"
                        | "bleeding"
                        | "light"
                        | "dazzled"
                        | "invisible"
                        | "cursed"
                        | "energy"
                        | "paralyze"
                ),
                "unknown condition type",
            )?;
            require(
                matches!(
                    condition.lifetime.as_str(),
                    "fixed_duration" | "damage_schedule"
                ),
                "unknown condition lifetime",
            )?;
            if let Some(dot) = &condition.damage_over_time {
                require(
                    condition.lifetime == "damage_schedule"
                        && dot.tick_profile == "fixed"
                        && dot.first_tick == "after_interval"
                        && !dot.fixed_ticks.is_empty()
                        && dot
                            .fixed_ticks
                            .iter()
                            .all(|tick| tick.count > 0 && tick.interval_ms > 0 && tick.amount > 0),
                    "invalid condition damage schedule",
                )?;
            }
            if let Some(attributes) = &condition.attribute_modifiers {
                require(
                    !attributes.is_empty()
                        && attributes.iter().all(|attribute| {
                            attribute.mode == "add"
                                && matches!(
                                    attribute.attribute.as_str(),
                                    "skill_shield"
                                        | "skill_fist"
                                        | "skill_distance"
                                        | "skill_melee"
                                        | "stat_magicpoints"
                                )
                        }),
                    "unknown attribute modifier",
                )?;
            }
            if let Some(regen) = &condition.regeneration {
                require(
                    regen.health_gain.is_some() == regen.health_interval_ms.is_some()
                        && regen.mana_gain.is_some() == regen.mana_interval_ms.is_some()
                        && (regen.health_gain.is_some() || regen.mana_gain.is_some())
                        && regen.health_interval_ms.is_none_or(|interval| interval > 0)
                        && regen.mana_interval_ms.is_none_or(|interval| interval > 0),
                    "invalid regeneration schedule",
                )?;
            }
            if let Some(light) = &condition.light {
                require(
                    light.level <= 255 && light.color <= 255,
                    "invalid light condition",
                )?;
            }
            if let Some(speed) = &condition.speed_formula {
                require(
                    speed.family == "Formula",
                    "wrong condition speed reference family",
                )?;
            }
        }
        if let Some(range) = &effect.duration_range_ms {
            require(
                range.minimum > 0
                    && range.minimum <= range.maximum
                    && range.minimum % 1000 == 0
                    && range.maximum % 1000 == 0,
                "invalid barrier duration range",
            )?;
        }
    }
    for formula in &deps.formulas {
        validate_identity(&formula.identity)?;
        match formula.kind.as_str() {
            "player_expression" => {
                require(
                    formula.speed.is_none()
                        && formula
                            .inputs
                            .as_deref()
                            .is_some_and(|s| matches!(s, "level_magic" | "skill")),
                    "invalid player formula",
                )?;
                expression(
                    formula
                        .minimum
                        .as_ref()
                        .ok_or_else(|| Error("missing minimum".into()))?,
                    1,
                )?;
                expression(
                    formula
                        .maximum
                        .as_ref()
                        .ok_or_else(|| Error("missing maximum".into()))?,
                    1,
                )?;
            }
            "speed_modifier" => {
                let speed = formula
                    .speed
                    .as_ref()
                    .ok_or_else(|| Error("missing speed formula".into()))?;
                require(
                    formula.minimum.is_none()
                        && formula.maximum.is_none()
                        && formula.inputs.is_none()
                        && speed.minimum_multiplier.denominator > 0
                        && speed.maximum_multiplier.denominator > 0,
                    "invalid speed formula",
                )?;
            }
            _ => return Err(Error("unknown formula kind".into())),
        }
    }
    Ok(())
}

#[allow(
    clippy::expect_used,
    reason = "post-validation commit invariant; a fallible exit here would leave a partial owner write"
)]
fn equivalent_body(entry: &CompiledEntry) -> (Value, Value) {
    let mut spell = entry.bundle["spell"].clone();
    let object = spell.as_object_mut().expect("typed spell object");
    object.remove("identity");
    object.remove("name");
    (spell, entry.dependency_payload.clone())
}

/// Compile all identities, including representability failures. The caller must
/// independently bind this digest into its admitted content generation.
pub(crate) fn compile(
    bytes: &[u8],
    expected_artifact_digest: &str,
) -> Result<CompiledCatalog, Error> {
    require(
        !bytes.is_empty() && bytes.len() <= MAX_ARTIFACT_BYTES,
        "invalid artifact byte length",
    )?;
    let actual = digest(bytes);
    require(
        is_digest(expected_artifact_digest) && actual == expected_artifact_digest,
        "artifact digest mismatch",
    )?;
    let artifact: Artifact = parsed(unique_document(bytes)?)?;
    require(
        artifact.schema == SCHEMA
            && artifact.revision == REVISION
            && artifact.bundles.len() == 246
            && artifact.removed.len() == 6,
        "unqualified catalog coverage or revision",
    )?;
    let expected_removed = [
        ("expose weakness", "S24"),
        ("sap strength", "S24"),
        ("light stone shower rune", "S25"),
        ("lightest missile rune", "S25"),
        ("practise fire wave", "S25"),
        ("practise healing", "S25"),
    ];
    let mut removed_names = BTreeSet::new();
    for removed in &artifact.removed {
        validate_identity(&removed.source_identity.identity)?;
        require(
            removed.source_identity.identity.revision == REVISION
                && removed.carrier == "instant"
                && expected_removed.contains(&(removed.name.as_str(), removed.policy.as_str()))
                && removed.evidence.contains(&removed.policy)
                && removed.evidence.contains("https://")
                && removed_names.insert(removed.name.clone()),
            "missing or substituted removal evidence",
        )?;
    }
    let mut entries = Vec::with_capacity(246);
    let mut identities = BTreeSet::new();
    let mut rune_items = BTreeSet::new();
    let mut native_count = 0;
    for input in artifact.bundles {
        nonnullable_profile(&input.bundle, false)?;
        nonnullable_profile(&input.dependencies, false)?;
        let bundle: Bundle = parsed(input.bundle.clone())?;
        let header = bundle.spell;
        let deps: DependencyProfile = parsed(input.dependencies.clone())?;
        validate_profile(&header, &deps)?;
        require(
            header.identity.revision == REVISION
                && deps
                    .abilities
                    .iter()
                    .all(|value| value.identity.revision == REVISION)
                && deps
                    .effects
                    .iter()
                    .all(|value| value.identity.revision == REVISION)
                && deps
                    .formulas
                    .iter()
                    .all(|value| value.identity.revision == REVISION),
            "unqualified catalog definition revision",
        )?;
        require(
            identities.insert(header.identity.clone())
                && !(header.carrier == "instant"
                    && removed_names.contains(&header.name.to_lowercase())),
            "duplicate or removed active identity",
        )?;
        if let Some(rune) = &header.rune {
            require(
                rune_items.insert(rune.item.key.clone()),
                "ambiguous rune item",
            )?;
        }
        require(
            !input.source_identities.is_empty(),
            "missing source identity",
        )?;
        for source in &input.source_identities {
            require(
                source.identity == header.identity
                    && !source.bundle_id.is_empty()
                    && is_digest(&source.manifest_sha256)
                    && !source.sources.is_empty()
                    && input.manifest["sources"] == Value::Array(source.sources.clone()),
                "inconsistent source provenance",
            )?;
        }
        let mut scope = BTreeSet::new();
        for (family, rows) in [
            ("Ability", &input.dependencies["abilities"]),
            ("Effect", &input.dependencies["effects"]),
            ("Formula", &input.dependencies["formulas"]),
        ] {
            for row in rows
                .as_array()
                .ok_or_else(|| Error("missing dependency family".into()))?
            {
                let id: Identity = parsed(row["identity"].clone())?;
                require(
                    scope.insert(DefinitionRef {
                        family: family.into(),
                        key: id.key,
                        revision: id.revision,
                    }),
                    "duplicate exact dependency",
                )?;
            }
        }
        for reference in input.catalog.definitions {
            validate_ref(&reference)?;
            require(
                matches!(reference.family.as_str(), "Item" | "Creature"),
                "catalog substitutes an executable local payload",
            )?;
            require(scope.insert(reference), "duplicate external reference")?;
        }
        let mut refs = Vec::new();
        references(&input.bundle, &mut refs)?;
        references(&input.dependencies, &mut refs)?;
        for reference in refs {
            require(
                scope.contains(&reference),
                format!(
                    "missing exact definition {}@{}",
                    reference.key, reference.revision
                ),
            )?;
        }
        // The 67 complete native profiles are closed by their qualified reader,
        // including the two barrier Abilities. A failed native qualification is
        // never reinterpreted as an ordinary Ability or skipped.
        let native = if header
            .execution
            .native_behavior
            .as_ref()
            .is_some_and(|n| n.key != "party_buff")
            || deps.effects.iter().any(|e| e.pvp_safe_item.is_some())
        {
            native_count += 1;
            Some(
                native::spell_from_bundle(&input.bundle, &input.dependencies)
                    .map_err(|e| Error(format!("native qualification: {:?}", e)))?,
            )
        } else {
            None
        };
        let party = if let Some(behavior) = header
            .execution
            .native_behavior
            .as_ref()
            .filter(|n| n.key == "party_buff")
        {
            let party: PartyParameters = parsed(behavior.parameters.clone())?;
            matrix(&party.area)?;
            require(
                party.same_floor
                    && party.requires_party
                    && party.min_affected > 0
                    && party.effect.family == "Effect"
                    && header.costs.mana == Some(0),
                "invalid party cast parameters",
            )?;
            match &party.mana {
                PartyManaProfile::Fixed { .. } => {}
                PartyManaProfile::Scaled {
                    falloff, rounding, ..
                } => require(
                    falloff.is_finite()
                        && *falloff > 0.0
                        && *falloff <= 1.0
                        && rounding == "up"
                        && (falloff * 100.0 - (falloff * 100.0).round()).abs() <= 1e-9,
                    "invalid party mana falloff",
                )?,
            }
            Some(party)
        } else {
            None
        };
        let definition = authoring::spell_from_bundle(&input.bundle, &input.dependencies);
        entries.push(CompiledEntry {
            header,
            dependencies: deps,
            definition,
            native,
            party,
            bundle: input.bundle,
            dependency_payload: input.dependencies,
            manifest: input.manifest,
            source_identities: input.source_identities,
        });
    }
    require(
        native_count == 67,
        "incomplete native qualification coverage",
    )?;
    entries.sort_by(|a, b| a.header.identity.cmp(&b.header.identity));
    let mut by_words: BTreeMap<String, Vec<&CompiledEntry>> = BTreeMap::new();
    for entry in &entries {
        if let Some(words) = &entry.header.words {
            by_words
                .entry(words.to_ascii_lowercase())
                .or_default()
                .push(entry);
        }
    }
    let aliases = by_words
        .into_iter()
        .filter(|(_, rows)| rows.len() > 1)
        .map(|(words, rows)| {
            let baseline = equivalent_body(rows[0]);
            AliasGroup {
                words,
                identities: rows.iter().map(|e| e.header.identity.clone()).collect(),
                equivalent: rows.iter().all(|e| equivalent_body(e) == baseline),
            }
        })
        .collect();
    Ok(CompiledCatalog {
        artifact_digest: actual,
        entries,
        aliases,
        removed: artifact.removed,
    })
}

impl CompiledCatalog {
    pub(crate) fn artifact_digest(&self) -> &str {
        &self.artifact_digest
    }

    pub(crate) fn into_spell_book(self) -> Result<SpellBook, Error> {
        self.into_spell_book_with_selections(&BTreeMap::new())
    }

    /// Selections resolve only an already reported ambiguous incantation. They
    /// do not invent equivalence, change either profile, or authorize activation.
    pub(crate) fn into_spell_book_with_selections(
        self,
        selections: &BTreeMap<String, Identity>,
    ) -> Result<SpellBook, Error> {
        let mut lookup_selections = BTreeMap::new();
        let mut used = BTreeSet::new();
        for group in &self.aliases {
            let selected = if group.equivalent {
                &group.identities[0]
            } else {
                let selected = selections.get(&group.words).ok_or_else(|| {
                    Error(format!(
                        "ambiguous incantation requires explicit selection: {}",
                        group.words
                    ))
                })?;
                require(
                    group.identities.contains(selected),
                    "selection does not belong to alias group",
                )?;
                used.insert(group.words.clone());
                selected
            };
            lookup_selections.insert(group.words.clone(), selected.key.clone());
        }
        require(
            used.len() == selections.len(),
            "unused or substituted alias selection",
        )?;
        let definitions = self
            .entries
            .into_iter()
            .map(|e| {
                e.definition.map_err(|error| {
                    Error(format!(
                        "{} cannot lower to the common cast owner: {}",
                        e.header.identity.key, error
                    ))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        SpellBook::new_with_alias_selections(definitions, &lookup_selections)
            .map_err(|e| Error(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
    use super::*;
    const SAMPLE: &[u8] = include_bytes!(
        "../../../../tools/content-schema/spell-authoring/samples/executable-spell-catalog.json"
    );
    const SELECTION: &[u8] = include_bytes!(
        "../../../../tools/content-schema/spell-authoring/samples/executable-spell-source-selection.json"
    );
    fn compile_sample() -> CompiledCatalog {
        compile(SAMPLE, &digest(SAMPLE)).unwrap()
    }
    fn changed(mutator: impl FnOnce(&mut Value)) -> Result<CompiledCatalog, Error> {
        let mut value: Value = serde_json::from_slice(SAMPLE).unwrap();
        mutator(&mut value);
        let bytes = serde_json::to_vec(&value).unwrap();
        compile(&bytes, &digest(&bytes))
    }
    #[test]
    fn preserves_every_identity_and_exact_native_profile() {
        let catalog = compile_sample();
        assert_eq!(catalog.entries.len(), 246);
        assert_eq!(catalog.removed.len(), 6);
        for entry in &catalog.entries {
            assert!(
                entry.definition.is_ok(),
                "{}: {:?}",
                entry.header.name,
                entry.definition
            );
        }
        assert_eq!(
            catalog
                .entries
                .iter()
                .filter(|entry| entry.party.is_some())
                .count(),
            5
        );
        assert_eq!(
            catalog
                .entries
                .iter()
                .filter(|e| e.native.is_some())
                .count(),
            67
        );
        assert_eq!(catalog.artifact_digest(), digest(SAMPLE));
        assert_eq!(catalog.aliases.iter().filter(|a| a.equivalent).count(), 1);
        assert_eq!(catalog.aliases.iter().filter(|a| !a.equivalent).count(), 4);
        assert!(catalog.entries.iter().any(|e| {
            e.dependencies
                .abilities
                .iter()
                .any(|a| a.zero_damage_health_path == Some(true))
        }));
        assert!(
            catalog
                .entries
                .iter()
                .any(|e| e.dependencies.effects.iter().any(|a| a
                    .presentation
                    .as_ref()
                    .is_some_and(|p| p.caster_effect_timing.is_some())))
        );
    }
    #[test]
    fn different_familiar_lifetime_requires_selection() {
        let error = compile_sample().into_spell_book().unwrap_err();
        assert!(error.0.contains("ambiguous incantation"));
    }
    #[test]
    fn policy_selection_preserves_full_indices_and_blocks_alternative_casts() {
        let catalog = compile_sample();
        let selected = compile_source_selection(SELECTION, &digest(SELECTION), &catalog).unwrap();
        let book = catalog.into_spell_book_with_selections(&selected).unwrap();
        let mut inactive = 0;
        for number in 1..=246 {
            let index = std::num::NonZeroU32::new(number).unwrap();
            let (_, active) = book
                .source_indexed(index)
                .expect("all source identities survive");
            assert_eq!(book.indexed(index).is_some(), active);
            if !active {
                inactive += 1;
            }
        }
        assert_eq!(inactive, 5);
        assert_eq!(
            book.spoken("utevo gran res dru").unwrap().spell.key,
            "candidate:spell/druid_familiar"
        );
    }
    #[test]
    fn selection_cannot_replace_its_catalog_or_source_policy() {
        let catalog = compile_sample();
        assert!(compile_source_selection(SELECTION, &"0".repeat(64), &catalog).is_err());
        let mut value: Value = serde_json::from_slice(SELECTION).unwrap();
        value["catalog_sha256"] = Value::String("0".repeat(64));
        let bytes = serde_json::to_vec(&value).unwrap();
        assert!(compile_source_selection(&bytes, &digest(&bytes), &catalog).is_err());
        let mut value: Value = serde_json::from_slice(SELECTION).unwrap();
        value["selections"][0]["policy"] = Value::String("implicit".into());
        let bytes = serde_json::to_vec(&value).unwrap();
        assert!(compile_source_selection(&bytes, &digest(&bytes), &catalog).is_err());
    }
    #[test]
    fn typed_effects_preserve_nonhealth_operations() {
        let catalog = compile_sample();
        assert!(catalog.entries.iter().any(|entry| {
            let Ok(definition) = &entry.definition else { return false; };
            matches!(&definition.execution, super::super::Execution::Effects(effects) if effects.iter().any(|effect| matches!(effect, super::super::SpellEffect::ResolvedOther { profile, .. } if profile.operation == "condition" && profile.condition.is_some())))
        }));
    }
    #[test]
    fn native_wheel_guard_uses_owned_facts_before_any_plan_or_draw() {
        let catalog = compile_sample();
        let definition = catalog
            .entries
            .iter()
            .find(|entry| {
                entry.native.is_some() && entry.header.requirements.wheel_unlock == Some(true)
            })
            .unwrap()
            .definition
            .as_ref()
            .unwrap();
        let caster = super::super::CasterState {
            vocation: *definition.vocations.iter().next().unwrap(),
            level: 999,
            magic_level: 999,
            premium: true,
            mana: 1_000_000,
            max_mana: 1_000_000,
            soul: 10_000,
            learned: BTreeSet::new(),
            attack_skill: 100,
            attack_value: 100,
            attack_factor: 1.0,
            shielding_skill: 100,
            melee_weapon: true,
            shield_defense: Some(100),
            harmony_multiplier: super::super::harmony::HarmonyMultiplier::ONE,
        };
        let position = super::super::chain::TilePosition {
            x: 0,
            y: 0,
            floor: 0,
        };
        let facts = super::super::OperationalCastFacts {
            caster_position: position,
            target_position: Some(position),
            target: None,
            line_of_sight_clear: Some(true),
            direction_available: true,
            wheel_unlocked: None,
            in_protection_zone: false,
            target_tile_solid: Some(false),
            target_tile_creature: Some(false),
        };
        let cooldowns = super::super::Cooldowns::default();
        let result = super::super::resolve_native_cast(
            definition,
            &caster,
            &cooldowns,
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
            &facts,
            native::Facts::Food,
            &mut |_, _| panic!("a rejected header cannot draw"),
        );
        assert_eq!(
            result.unwrap_err(),
            super::super::CastRejection::WheelUnlockRequired
        );
        assert!(cooldowns.canonical_deadlines().is_empty());
        assert_eq!(caster.mana, 1_000_000);
    }
    #[test]
    fn cooldown_preflight_snapshot_keeps_namespaces_and_microsecond_units() {
        let mut cooldowns = super::super::Cooldowns::default();
        cooldowns.spells.insert(
            "same".into(),
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(12_345),
        );
        cooldowns.groups.insert(
            "same".into(),
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(67_890),
        );
        assert_eq!(
            cooldowns.canonical_deadlines(),
            vec![("group:same".into(), 67_890), ("spell:same".into(), 12_345)]
        );
    }
    #[test]
    fn digest_and_coverage_fail_closed() {
        assert!(compile(SAMPLE, &"0".repeat(64)).is_err());
        assert!(
            changed(|v| {
                v["bundles"].as_array_mut().unwrap().pop();
            })
            .is_err()
        );
        assert!(
            changed(|v| {
                v["removed"].as_array_mut().unwrap().pop();
            })
            .is_err()
        );
    }
    #[test]
    fn duplicate_json_cannot_silently_replace_a_guard_or_proof() {
        assert!(
            unique_document(
                br#"{"targeting":{"allowed_targets":"not_self","allowed_targets":"any"}}"#
            )
            .is_err()
        );
        assert!(unique_document(br#"{"revision":"qualified","revision":"different"}"#).is_err());
    }
    #[test]
    fn exact_reference_revision_cannot_be_substituted() {
        assert!(
            changed(|v| {
                for entry in v["bundles"].as_array_mut().unwrap() {
                    if let Some(ability) = entry["bundle"]["spell"]["execution"].get_mut("ability")
                    {
                        ability["revision"] = Value::String("other-generation".into());
                        break;
                    }
                }
            })
            .is_err()
        );
    }
    #[test]
    fn unknown_operational_field_cannot_be_ignored() {
        assert!(
            changed(|v| {
                v["bundles"][0]["bundle"]["spell"]["targeting"]["unimplemented_guard"] =
                    Value::Bool(true);
            })
            .is_err()
        );
    }
    #[test]
    fn native_header_tampering_is_not_ordinary_fallback() {
        assert!(
            changed(|v| {
                for entry in v["bundles"].as_array_mut().unwrap() {
                    if entry["bundle"]["spell"]["execution"]["native_behavior"]["key"]
                        .as_str()
                        .is_some_and(|s| s != "party_buff")
                    {
                        entry["bundle"]["spell"]["costs"]["soul"] = Value::from(123u32);
                        break;
                    }
                }
            })
            .is_err()
        );
    }
    #[test]
    fn removed_identity_cannot_be_reactivated_by_status() {
        assert!(
            changed(|v| {
                v["removed"][0]["policy"] = Value::String("unknown".into());
            })
            .is_err()
        );
    }
}
