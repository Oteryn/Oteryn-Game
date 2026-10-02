//! CHARM-1 source decoding into CHARM-3/4's existing types. This immutable projection is
//! not an activated Content generation and supplies no capability or effect availability.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::value::RawValue;
use sha2::{Digest, Sha256};

use crate::combat::charm_effects::{
    CharmCatalogueRead, CharmCategory, CharmDefinition, CharmPercent, CharmStageValue,
};
use crate::domain::charm::{CharmCatalogue, CharmCategory as DomainCategory, CharmKey};

use super::charm_source_effect::lower_effect;
use super::charm_source_json::{CharmSourceMembers, ExactPercent};
use super::{ContentError, DefinitionRevisionRef, EvidenceLimits, ProductionKey};

const COUNT: usize = 25;
const SHARD_PATH: &str = "content/charms/charms-00000-00024.json";

fn invalid() -> ContentError {
    ContentError::InvalidArtifact("invalid canonical Charm catalogue")
}

/// Raw source bytes preserve provenance independently of the typed arithmetic projection.
pub(crate) struct CanonicalCharmCatalogue {
    catalogue: CharmCatalogue,
    effects: BTreeMap<String, CharmDefinition>,
    index_bytes: Vec<u8>,
    shard_bytes: Vec<u8>,
    source_digest: [u8; 32],
}

impl CanonicalCharmCatalogue {
    /// Import the committed static data once at node boot, like the spell book and
    /// Achievement catalogue. These are data, not an activated generation or capability.
    pub(crate) fn embedded() -> Result<Self, ContentError> {
        let limits = embedded_limits()?;
        decode_canonical_charms(
            include_bytes!("../../../../content/charms/index.json"),
            &[(
                SHARD_PATH,
                include_bytes!("../../../../content/charms/charms-00000-00024.json"),
            )],
            &limits,
        )
    }

    pub(crate) fn catalogue(&self) -> &CharmCatalogue {
        &self.catalogue
    }

    /// Evidence digest only; it is not a qualified generation identity or activation pin.
    pub(crate) fn source_digest(&self) -> [u8; 32] {
        self.source_digest
    }

    pub(crate) fn source_bytes(&self) -> (&[u8], &[u8]) {
        (&self.index_bytes, &self.shard_bytes)
    }
}

impl CharmCatalogueRead for CanonicalCharmCatalogue {
    fn charm(&self, key: &str) -> Option<&CharmDefinition> {
        self.effects.get(key)
    }
}

// Reuse existing Content resource ceilings for this read-only import. The evidence
// profile does not extend the admitted production format or qualify this catalogue.
fn embedded_limits() -> Result<EvidenceLimits, ContentError> {
    use super::{
        FIRST_PRODUCTION_MAX_ATOM_BYTES, FIRST_PRODUCTION_MAX_CELLS,
        FIRST_PRODUCTION_MAX_DEFINITIONS, FIRST_PRODUCTION_MAX_KEY_BYTES,
        FIRST_PRODUCTION_MAX_RECORD_BYTES, FIRST_PRODUCTION_MAX_REFERENCES,
        FIRST_PRODUCTION_MAX_SECTION_BYTES, FIRST_PRODUCTION_MAX_SERVER_ARTIFACT_BYTES,
        FIRST_PRODUCTION_MAX_SERVER_RECORDS,
    };
    EvidenceLimits::new(
        "evidence:embedded-charm-catalogue",
        FIRST_PRODUCTION_MAX_SERVER_ARTIFACT_BYTES,
        2, // The registered source has exactly an index and one shard.
        FIRST_PRODUCTION_MAX_SECTION_BYTES,
        FIRST_PRODUCTION_MAX_SERVER_RECORDS,
        FIRST_PRODUCTION_MAX_RECORD_BYTES,
        FIRST_PRODUCTION_MAX_KEY_BYTES,
        FIRST_PRODUCTION_MAX_ATOM_BYTES,
        FIRST_PRODUCTION_MAX_DEFINITIONS,
        FIRST_PRODUCTION_MAX_CELLS,
        FIRST_PRODUCTION_MAX_REFERENCES,
    )
}

/// Decode the actual registered, single-shard CHARM-1 source format. No filesystem or
/// mutable global catalogue is introduced. The Content owner supplies the named source bytes.
pub(crate) fn decode_canonical_charms(
    index_bytes: &[u8],
    shards: &[(&str, &[u8])],
    limits: &EvidenceLimits,
) -> Result<CanonicalCharmCatalogue, ContentError> {
    if shards.len() != 1 {
        return Err(invalid());
    }
    limits.check("Charm sections", 2, limits.max_sections())?;
    limits.check("Charm records", COUNT, limits.max_records())?;
    limits.check("Charm definitions", COUNT, limits.max_definitions())?;
    let shard_bytes = shards[0].1;
    let total = index_bytes
        .len()
        .checked_add(shard_bytes.len())
        .ok_or_else(invalid)?;
    limits.check("Charm artifact bytes", total, limits.max_artifact_bytes())?;
    for section in [index_bytes, shard_bytes] {
        limits.check(
            "Charm section bytes",
            section.len(),
            limits.max_section_bytes(),
        )?;
    }
    let index: Index = serde_json::from_slice(index_bytes).map_err(|_| invalid())?;
    check_text(index_bytes, limits)?;
    if index.schema != "OTERYN_FAMILY_INDEX/v1"
        || index.family != "Charm"
        || index.record_count != COUNT
        || index.shards != [SHARD_PATH]
        || shards[0].0 != SHARD_PATH
        || index.authoring_source.schema != "OTERYN_CHARM_AUTHORING_CATALOGUE/v1"
        || index.authoring_source.path
            != "tools/content-schema/charm-authoring/samples/charms-candidate.json"
        || !valid_digest(&index.authoring_source.sha256)
    {
        return Err(invalid());
    }
    let shard: Shard = serde_json::from_slice(shard_bytes).map_err(|_| invalid())?;
    if shard.schema != "OTERYN_CHARM_SHARD/v1"
        || shard.family != "Charm"
        || shard.shard.count != COUNT
        || shard.shard.start != 0
        || shard.shard.end != COUNT - 1
        || shard.shard.index != 0
    {
        return Err(invalid());
    }
    let mut effects = BTreeMap::new();
    let mut domain = Vec::with_capacity(COUNT);
    // Closed shard header keys/strings are shorter than the mandatory index's
    // authoring_source key and schema string, whose text budgets already passed.
    for record in shard.records {
        limits.check(
            "Charm record bytes",
            record.definition.get().len(),
            limits.max_record_bytes(),
        )?;
        check_text(record.definition.get().as_bytes(), limits)?;
        let row: Definition =
            serde_json::from_str(record.definition.get()).map_err(|_| invalid())?;
        limits.check(
            "Charm key bytes",
            row.identity.key.len(),
            limits.max_key_bytes(),
        )?;
        ProductionKey::new(&row.identity.key)?;
        DefinitionRevisionRef::new(&row.identity.revision)?;
        if row.name.is_empty()
            || !matches!(row.kind.as_str(), "passive" | "offensive" | "defensive")
            || row.implemented_version.is_empty()
            || !row.sources.get().starts_with('{')
        {
            return Err(invalid());
        }
        let (category, domain_category, currency) = match row.category.as_str() {
            "major" => (CharmCategory::Major, DomainCategory::Major, "charm_points"),
            "minor" => (
                CharmCategory::Minor,
                DomainCategory::Minor,
                "minor_charm_echoes",
            ),
            _ => return Err(invalid()),
        };
        if row.cost_currency != currency || row.stages.map(|stage| stage.stage) != [1, 2, 3] {
            return Err(invalid());
        }
        let costs = row.stages.map(|stage| stage.cost);
        if costs[0] == 0
            || costs.windows(2).any(|pair| pair[0] >= pair[1])
            || costs
                .iter()
                .any(|cost| *cost > oteryn_protocol_oteryn::charm::MAX_CHARM_STAGE_COST)
        {
            return Err(invalid());
        }
        let stage_value = match row.stage_value.as_str() {
            "effect_percent" => CharmStageValue::EffectPercent,
            "trigger_chance_percent" => CharmStageValue::TriggerChancePercent,
            _ => return Err(invalid()),
        };
        let values = row.stages.map(|stage| stage.value.0);
        let stages = [
            percent(values[0])?,
            percent(values[1])?,
            percent(values[2])?,
        ];
        let definition = CharmDefinition::new(
            &row.identity.key,
            category,
            stage_value,
            stages,
            lower_effect(&row.effect)?,
        )
        .map_err(|_| invalid())?;
        domain.push(crate::domain::charm::CharmDefinition {
            key: CharmKey::new(&row.identity.key).map_err(|_| invalid())?,
            category: domain_category,
            stage_costs: costs,
        });
        if effects.insert(row.identity.key, definition).is_some() {
            return Err(invalid());
        }
    }
    let catalogue = CharmCatalogue::new(domain).map_err(|_| invalid())?;
    let source_digest = Sha256::new()
        .chain_update(b"oteryn:canonical-charms-source:v1")
        .chain_update((index_bytes.len() as u64).to_be_bytes())
        .chain_update(index_bytes)
        .chain_update((shard_bytes.len() as u64).to_be_bytes())
        .chain_update(shard_bytes)
        .finalize()
        .into();
    Ok(CanonicalCharmCatalogue {
        catalogue,
        effects,
        index_bytes: index_bytes.to_vec(),
        shard_bytes: shard_bytes.to_vec(),
        source_digest,
    })
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn percent(value: u32) -> Result<CharmPercent, ContentError> {
    CharmPercent::from_hundredths(value).ok_or_else(invalid)
}

// Walk raw values so duplicate names cannot hide earlier oversized text. Numeric
// evidence remains uninterpreted, and runtime percentages still use ExactPercent.
fn check_text(bytes: &[u8], limits: &EvidenceLimits) -> Result<(), ContentError> {
    let raw: Box<RawValue> = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    let mut pending = vec![raw];
    while let Some(raw) = pending.pop() {
        match raw.get().as_bytes().first() {
            Some(b'{') => {
                let members: CharmSourceMembers =
                    serde_json::from_str(raw.get()).map_err(|_| invalid())?;
                for (key, value) in members.0 {
                    limits.check("Charm key bytes", key.len(), limits.max_key_bytes())?;
                    pending.push(value);
                }
            }
            Some(b'[') => {
                let values: Vec<Box<RawValue>> =
                    serde_json::from_str(raw.get()).map_err(|_| invalid())?;
                pending.extend(values);
            }
            Some(b'"') => {
                let text: String = serde_json::from_str(raw.get()).map_err(|_| invalid())?;
                limits.check("Charm string bytes", text.len(), limits.max_string_bytes())?;
            }
            _ => {}
        }
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Index {
    schema: String,
    family: String,
    record_count: usize,
    shards: [String; 1],
    authoring_source: AuthoringSource,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthoringSource {
    path: String,
    schema: String,
    sha256: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Shard {
    schema: String,
    family: String,
    shard: ShardIdentity,
    records: [Record; COUNT],
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ShardIdentity {
    count: usize,
    start: usize,
    end: usize,
    index: usize,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    definition: Box<RawValue>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Definition {
    identity: Identity,
    category: String,
    cost_currency: String,
    effect: Box<RawValue>,
    implemented_version: String,
    kind: String,
    name: String,
    sources: Box<RawValue>,
    stage_value: String,
    stages: [Stage; 3],
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Identity {
    key: String,
    revision: String,
}
#[derive(Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stage {
    stage: u8,
    cost: u32,
    value: ExactPercent,
}

#[cfg(test)]
#[path = "charm_source_tests.rs"]
mod tests;
