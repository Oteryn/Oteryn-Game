//! WCHANGE-CONTENT-1 source-selection candidates (D1054/D1057), not compiled W1 definitions.
//!
//! These retain the accepted names, chances, weights and reference provenance while native
//! layers/value-source multiplicity/spawn/NPC/raid/announcement bindings remain unavailable.
//! Validation below checks those source facts only. It cannot admit a bundle or activate a
//! change. There is no runtime/default preset, RNG draw, geometry or World/Channel authority.

use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

// Accepted W14 candidate ceilings; registration/compiler integration remains separate.
const MAX_DEFINITIONS: usize = 256;
const MAX_VARIANTS: usize = 16;
const SCHEMA: &str = "OTERYN_WORLD_CHANGE_SELECTION_SOURCE/candidate-1";
const SOURCED_FIELDS: [&str; 6] = [
    "key",
    "kind",
    "trigger",
    "variants.variant_key",
    "variants.weight",
    "variants.reference_map",
];

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorldChangeCandidate {
    schema: String,
    key: String,
    kind: ChangeKind,
    trigger: Trigger,
    variants: Vec<Variant>,
    sources: BTreeMap<String, Source>,
    /// Each selection field names a source and a nonempty locator, e.g. `decision:W10`.
    provenance: BTreeMap<String, String>,
    /// Explicitly incomplete. No empty gameplay arrays stand in for missing source bindings.
    unresolved: Vec<BindingGap>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum ChangeKind {
    WorldChange,
    MiniWorldChange,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Trigger {
    AtReset { chance_permille: u16 },
    Calendar { month: u8, day: u8, epochs: u64 },
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Variant {
    variant_key: String,
    weight: u64,
    /// Legacy reference locator only; never a native map-layer identity or runtime file path.
    reference_map: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Source {
    url: String,
    sha256: String,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum BindingGap {
    NativeLayerGeometry,
    TargetHouseMembership,
    ValueSourceMultiplicity,
    SpawnNpcRaidBindings,
    AnnouncementText,
    CompilerBundleBinding,
}

const ALL_BINDING_GAPS: [BindingGap; 6] = [
    BindingGap::NativeLayerGeometry,
    BindingGap::TargetHouseMembership,
    BindingGap::ValueSourceMultiplicity,
    BindingGap::SpawnNpcRaidBindings,
    BindingGap::AnnouncementText,
    BindingGap::CompilerBundleBinding,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CandidateError {
    DefinitionCount,
    Schema,
    Key,
    DuplicateDefinition,
    Trigger,
    VariantCount,
    DuplicateVariant,
    Weight,
    ReferenceMap,
    Provenance,
    MissingBindingGaps,
}

/// A validated selection-source entry, not an executable W1 definition (D1206).
/// Its private field prevents a consumer from treating unchecked JSON as compiled selection.
#[derive(Debug)]
pub(crate) struct WorldChangeSelection<'a> {
    source: &'a WorldChangeCandidate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WorldChangeBindingsUnavailable;

impl WorldChangeSelection<'_> {
    /// Compiler boundary for an entry that needs native W1 bindings. Selection-source
    /// compilation alone cannot supply geometry, House, value-source or spawn authority.
    pub(crate) fn require_w1_bindings(&self) -> Result<(), WorldChangeBindingsUnavailable> {
        // Ok is unreachable for validated input: validate_candidates rejects an
        // empty unresolved list, and compile_selection_source alone constructs
        // this selection's private source field. No complete bindings are invented.
        if self.source.unresolved.is_empty() {
            Ok(())
        } else {
            Err(WorldChangeBindingsUnavailable)
        }
    }
}

/// Compile only the owned selection-source. No W1 schema or gameplay bindings are emitted.
pub(crate) fn compile_selection_source(
    candidates: &[WorldChangeCandidate],
) -> Result<Vec<WorldChangeSelection<'_>>, CandidateError> {
    validate_candidates(candidates)?;
    Ok(candidates
        .iter()
        .map(|source| WorldChangeSelection { source })
        .collect())
}

fn key(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        })
}

/// Validate bounded source candidates only. Every successful input still explicitly lacks
/// native W1 bindings; callers must not interpret `Ok` as compiler or gameplay admission.
/// Source hashes identify retained evidence; this function does not fetch or attest that evidence.
pub(crate) fn validate_candidates(
    candidates: &[WorldChangeCandidate],
) -> Result<(), CandidateError> {
    if candidates.len() > MAX_DEFINITIONS {
        return Err(CandidateError::DefinitionCount);
    }
    let mut keys = BTreeSet::new();
    for candidate in candidates {
        if candidate.schema != SCHEMA {
            return Err(CandidateError::Schema);
        }
        if !key(&candidate.key) {
            return Err(CandidateError::Key);
        }
        if !keys.insert(&candidate.key) {
            return Err(CandidateError::DuplicateDefinition);
        }
        match candidate.trigger {
            Trigger::AtReset { chance_permille } if chance_permille <= 1000 => {}
            Trigger::Calendar { month, day, epochs }
                if epochs > 0
                    && (1..=12).contains(&month)
                    && day > 0
                    && day
                        <= match month {
                            2 => 29,
                            4 | 6 | 9 | 11 => 30,
                            _ => 31,
                        } => {}
            _ => return Err(CandidateError::Trigger),
        }
        if candidate.variants.is_empty() || candidate.variants.len() > MAX_VARIANTS {
            return Err(CandidateError::VariantCount);
        }
        let mut variants = BTreeSet::new();
        let mut total_weight = 0_u64;
        for variant in &candidate.variants {
            if !key(&variant.variant_key) {
                return Err(CandidateError::Key);
            }
            if !variants.insert(&variant.variant_key) {
                return Err(CandidateError::DuplicateVariant);
            }
            if variant.weight == 0 {
                return Err(CandidateError::Weight);
            }
            total_weight = total_weight
                .checked_add(variant.weight)
                .ok_or(CandidateError::Weight)?;
            if variant.reference_map.is_empty() {
                return Err(CandidateError::ReferenceMap);
            }
        }
        if candidate.sources.is_empty()
            || candidate.sources.iter().any(|(name, source)| {
                !key(name)
                    || !source.url.starts_with("https://")
                    || source.sha256.len() != 64
                    || !source
                        .sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            })
            || candidate.provenance.len() != SOURCED_FIELDS.len()
        {
            return Err(CandidateError::Provenance);
        }
        for field in SOURCED_FIELDS {
            let Some((source, locator)) = candidate
                .provenance
                .get(field)
                .and_then(|value| value.split_once(':'))
            else {
                return Err(CandidateError::Provenance);
            };
            if !candidate.sources.contains_key(source) || locator.is_empty() {
                return Err(CandidateError::Provenance);
            }
        }
        let gaps: BTreeSet<_> = candidate.unresolved.iter().collect();
        // Resolved bindings are not representable yet, so every gap must remain.
        if gaps.len() != candidate.unresolved.len() || gaps.len() != ALL_BINDING_GAPS.len() {
            return Err(CandidateError::MissingBindingGaps);
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "content_tests.rs"]
mod tests;
