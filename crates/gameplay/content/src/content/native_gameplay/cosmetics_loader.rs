//! Private permanent-cosmetics CANDIDATE compiler (CHARACTER_APPEARANCE §4.3-§4.6).
//! Reuses existing staged Outfit reader output; no DAT parser or selection validator is added.
//! Proposed digests and this read-only view are DATA, never current admitted-world authority.
//! Active artifact/World/profile/sex/unlock admission and publication remain separate CP seams.
#![allow(
    dead_code,
    reason = "Private candidate compiler; active artifact integration is a CP slot"
)]

use crate::content::{DefinitionRevisionRef, ProductionKey};
use crate::domain::appearance::{
    AccountUnlocks, AppearanceContent, AppearanceSelection, Sex, UnlockFact, validate_selection,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

mod grant_mappings;

const JSON_BYTES_MAX: usize = 8 * 1024 * 1024;
const SOURCE_BYTES_MAX: usize = 32 * 1024 * 1024;

/// Independent proposed expectations for byte binding, NOT a trusted/current source capability.
#[derive(Debug, Clone, Copy)]
pub struct ProposedCosmeticsBinding<'a> {
    pub document_sha256: [u8; 32],
    pub content_revision: &'a str,
    pub manifest_sha256: [u8; 32],
    pub appearances_sha256: [u8; 32],
    pub client_version: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CosmeticsDiagnostic {
    InputTooLarge,
    MalformedDocument,
    DocumentBinding,
    SourceBinding,
    MalformedSource,
    MissingOrDuplicateShard,
    ShardDigest { shard: usize },
    SourceRecord { record: usize },
    Palette,
    Definition { family: &'static str, index: usize },
    DuplicateKey { family: &'static str, index: usize },
    Look { family: &'static str, index: usize },
    Fallback { sex: Sex },
}
type Result<T> = std::result::Result<T, CosmeticsDiagnostic>;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceCitation {
    reference: String,
    revision: String,
    sha256: String,
}
impl SourceCitation {
    fn valid(&self) -> bool {
        text(&self.reference, 1024) && text(&self.revision, 128) && hash(&self.sha256).is_some()
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LookRef {
    id: u32,
    source_record_sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Palette {
    key: String,
    revision: String,
    colours: Vec<[u8; 3]>,
    source: SourceCitation,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Outfit {
    key: String,
    revision: String,
    female: LookRef,
    male: LookRef,
    addon_count: u8,
    starter_female: bool,
    starter_male: bool,
    starter_addons_female: u8,
    starter_addons_male: u8,
    premium: bool,
    unlock_sources: Vec<SourceCitation>,
    source: SourceCitation,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Mount {
    key: String,
    revision: String,
    look: LookRef,
    speed_bonus: u16,
    premium: bool,
    source: SourceCitation,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fallback {
    outfit_key: String,
    colours: [u16; 4],
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fallbacks {
    female: Fallback,
    male: Fallback,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: String,
    content_revision: String,
    source_manifest_sha256: String,
    source_appearances_sha256: String,
    palette: Palette,
    outfits: Vec<Outfit>,
    mounts: Vec<Mount>,
    fallbacks: Fallbacks,
    #[serde(default)]
    grant_mappings: Vec<grant_mappings::GrantMapping>,
}

// Exact existing stage_appearance_tables.py formats. Large rendering observations are ignored,
// not reinterpreted; their complete bytes are still covered by the expected shard digest.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema: String,
    authority: String,
    client_version: String,
    family: String,
    identity_namespace: String,
    source: String,
    top_level_field: u8,
    record_count: usize,
    excluded_top_level: serde::de::IgnoredAny,
    inputs: Inputs,
    files: Vec<ShardRef>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Inputs {
    appearances: AppearanceInput,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AppearanceInput {
    file: String,
    sha256: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ShardRef {
    path: String,
    records: usize,
    sha256: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Observations {
    schema: String,
    family: String,
    records: Vec<Observation>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Observation {
    source_id: u32,
    source_index: usize,
    source_record_sha256: String,
    flags: serde::de::IgnoredAny,
    frame_groups: serde::de::IgnoredAny,
}

/// Validated proposed document only. Implements the native PURE content interface, but is not
/// an independently issued active-content capability and must not be installed as one.
#[derive(Debug)]
pub struct CosmeticsCandidate {
    document: Document,
    outfits: BTreeMap<String, usize>,
    mounts: BTreeMap<String, usize>,
    grants: BTreeMap<String, usize>,
    palette_size: u16,
    document_sha256: [u8; 32],
}

impl CosmeticsCandidate {
    pub fn content_revision(&self) -> &str {
        &self.document.content_revision
    }
    pub const fn document_sha256(&self) -> [u8; 32] {
        self.document_sha256
    }
    pub fn look_type(&self, key: &str, sex: Sex) -> Option<u32> {
        let outfit = &self.document.outfits[*self.outfits.get(key)?];
        Some(match sex {
            Sex::Female => outfit.female.id,
            Sex::Male => outfit.male.id,
        })
    }
    pub fn mount_metadata(&self, key: &str) -> Option<(u32, u16, bool)> {
        let mount = &self.document.mounts[*self.mounts.get(key)?];
        Some((mount.look.id, mount.speed_bonus, mount.premium))
    }
    pub fn palette(&self) -> &[[u8; 3]] {
        &self.document.palette.colours
    }
    pub fn fallback(&self, sex: Sex) -> AppearanceSelection<String> {
        let fallback = match sex {
            Sex::Female => &self.document.fallbacks.female,
            Sex::Male => &self.document.fallbacks.male,
        };
        AppearanceSelection {
            outfit_key: fallback.outfit_key.clone(),
            colours: fallback.colours,
            addons: 0,
            mount_key: None,
        }
    }
}

impl AppearanceContent<String> for CosmeticsCandidate {
    fn earned_grant_keys(&self) -> Vec<String> {
        self.document
            .grant_mappings
            .iter()
            .map(|m| m.grant_key.clone())
            .collect()
    }
    fn earned_unlock(
        &self,
        grant_key: &str,
        provenance: &str,
        target: crate::domain::appearance::EarnedCosmetic<'_>,
    ) -> UnlockFact {
        grant_mappings::resolve(
            self.grants
                .get(grant_key)
                .map(|index| &self.document.grant_mappings[*index]),
            provenance,
            target,
        )
    }
    fn palette_size(&self) -> u16 {
        self.palette_size
    }
    fn outfit_addon_count(&self, key: &String) -> Option<u8> {
        self.outfits
            .get(key)
            .map(|i| self.document.outfits[*i].addon_count)
    }
    fn outfit_is_starter(&self, key: &String, sex: Sex) -> bool {
        self.outfits.get(key).is_some_and(|i| match sex {
            Sex::Female => self.document.outfits[*i].starter_female,
            Sex::Male => self.document.outfits[*i].starter_male,
        })
    }
    fn addon_is_starter(&self, key: &String, sex: Sex, addon: u8) -> bool {
        let Some(index) = self.outfits.get(key) else {
            return false;
        };
        let outfit = &self.document.outfits[*index];
        if addon == 0 || addon > outfit.addon_count {
            return false;
        }
        let mask = match sex {
            Sex::Female => outfit.starter_addons_female,
            Sex::Male => outfit.starter_addons_male,
        };
        mask & (1 << (addon - 1)) != 0
    }
    fn mount_exists(&self, key: &String) -> bool {
        self.mounts.contains_key(key)
    }
}

/// Binds proposed authored bytes to the independently supplied proposed source bytes, checks
/// completeness and native selection semantics, and returns a NON-AUTHORITATIVE candidate.
pub fn compile_candidate(
    bytes: &[u8],
    expected: ProposedCosmeticsBinding<'_>,
    manifest: &[u8],
    shards: &[(&str, &[u8])],
) -> Result<CosmeticsCandidate> {
    if bytes.len() > JSON_BYTES_MAX {
        return Err(CosmeticsDiagnostic::InputTooLarge);
    }
    if expected.document_sha256 == [0; 32] || sha(bytes) != expected.document_sha256 {
        return Err(CosmeticsDiagnostic::DocumentBinding);
    }
    let document: Document =
        serde_json::from_slice(bytes).map_err(|_| CosmeticsDiagnostic::MalformedDocument)?;
    if document.schema != "OTERYN_CHARACTER_COSMETICS_CANDIDATE/v1"
        || !text(&document.content_revision, 128)
        || document.content_revision != expected.content_revision
        || hash(&document.source_manifest_sha256) != Some(expected.manifest_sha256)
        || hash(&document.source_appearances_sha256) != Some(expected.appearances_sha256)
    {
        return Err(CosmeticsDiagnostic::DocumentBinding);
    }
    let looks = read_staged_outfits(expected, manifest, shards)?;
    let palette = &document.palette;
    let palette_size =
        u16::try_from(palette.colours.len()).map_err(|_| CosmeticsDiagnostic::Palette)?;
    if palette_size == 0 || !identity(&palette.key, &palette.revision) || !palette.source.valid() {
        return Err(CosmeticsDiagnostic::Palette);
    }
    let mut outfits = BTreeMap::new();
    let mut mounts = BTreeMap::new();
    if document.outfits.is_empty() {
        return Err(CosmeticsDiagnostic::Definition {
            family: "Outfit",
            index: 0,
        });
    }
    for (index, outfit) in document.outfits.iter().enumerate() {
        let error = CosmeticsDiagnostic::Definition {
            family: "Outfit",
            index,
        };
        if !identity(&outfit.key, &outfit.revision)
            || outfit.addon_count > 8
            || !outfit.source.valid()
            || outfit.unlock_sources.iter().any(|s| !s.valid())
        {
            return Err(error);
        }
        let allowed = u8::try_from((1_u16 << outfit.addon_count) - 1).map_err(|_| error.clone())?;
        if (outfit.starter_addons_female | outfit.starter_addons_male) & !allowed != 0 {
            return Err(error);
        }
        if !look_matches(&outfit.female, &looks) || !look_matches(&outfit.male, &looks) {
            return Err(CosmeticsDiagnostic::Look {
                family: "Outfit",
                index,
            });
        }
        if outfits.insert(outfit.key.clone(), index).is_some() {
            return Err(CosmeticsDiagnostic::DuplicateKey {
                family: "Outfit",
                index,
            });
        }
    }
    for (index, mount) in document.mounts.iter().enumerate() {
        if !identity(&mount.key, &mount.revision) || !mount.source.valid() {
            return Err(CosmeticsDiagnostic::Definition {
                family: "Mount",
                index,
            });
        }
        if !look_matches(&mount.look, &looks) {
            return Err(CosmeticsDiagnostic::Look {
                family: "Mount",
                index,
            });
        }
        if outfits.contains_key(&mount.key) || mounts.insert(mount.key.clone(), index).is_some() {
            return Err(CosmeticsDiagnostic::DuplicateKey {
                family: "Mount",
                index,
            });
        }
    }
    grant_mappings::validate(
        &document.grant_mappings,
        &document.outfits,
        &document.mounts,
    )?;
    let grants = document
        .grant_mappings
        .iter()
        .enumerate()
        .map(|(index, mapping)| (mapping.grant_key.clone(), index))
        .collect();
    let candidate = CosmeticsCandidate {
        document,
        outfits,
        mounts,
        grants,
        palette_size,
        document_sha256: expected.document_sha256,
    };
    for sex in [Sex::Female, Sex::Male] {
        validate_selection(&candidate.fallback(sex), sex, &candidate, &NoEarnedUnlocks)
            .map_err(|_| CosmeticsDiagnostic::Fallback { sex })?;
    }
    Ok(candidate)
}

fn read_staged_outfits(
    expected: ProposedCosmeticsBinding<'_>,
    bytes: &[u8],
    shards: &[(&str, &[u8])],
) -> Result<BTreeMap<u32, [u8; 32]>> {
    let total = shards
        .iter()
        .try_fold(bytes.len(), |sum, (_, b)| sum.checked_add(b.len()))
        .ok_or(CosmeticsDiagnostic::InputTooLarge)?;
    if bytes.len() > JSON_BYTES_MAX
        || total > SOURCE_BYTES_MAX
        || shards.iter().any(|(_, b)| b.len() > JSON_BYTES_MAX)
    {
        return Err(CosmeticsDiagnostic::InputTooLarge);
    }
    if expected.manifest_sha256 == [0; 32]
        || expected.appearances_sha256 == [0; 32]
        || sha(bytes) != expected.manifest_sha256
    {
        return Err(CosmeticsDiagnostic::SourceBinding);
    }
    let manifest: Manifest =
        serde_json::from_slice(bytes).map_err(|_| CosmeticsDiagnostic::MalformedSource)?;
    if manifest.schema != "OTERYN_CIPSOFT_APPEARANCE_OUTFIT_SOURCE_MANIFEST/v1"
        || manifest.family != "Outfit"
        || manifest.top_level_field != 2
        || manifest.authority
            != "SOURCE_OBSERVATIONS_ONLY; no Oteryn ProductionKeys or gameplay promotion"
        || manifest.identity_namespace != "cipsoft/appearance-outfits"
        || manifest.source != "CIPSOFT_CLIENT_ASSETS"
        || manifest.client_version != expected.client_version
        || !text(expected.client_version, 128)
        || hash(&manifest.inputs.appearances.sha256) != Some(expected.appearances_sha256)
        || !text(&manifest.inputs.appearances.file, 1024)
        || manifest.record_count == 0
        || manifest.record_count > usize::from(u16::MAX)
    {
        return Err(CosmeticsDiagnostic::SourceBinding);
    }
    let supplied: BTreeMap<_, _> = shards.iter().copied().collect();
    if supplied.len() != shards.len()
        || manifest.files.is_empty()
        || manifest.files.len() != supplied.len()
    {
        return Err(CosmeticsDiagnostic::MissingOrDuplicateShard);
    }
    let mut seen = BTreeSet::new();
    let mut looks = BTreeMap::new();
    let mut ordinal = 0;
    for (index, file) in manifest.files.iter().enumerate() {
        if !plain_name(&file.path) || !seen.insert(&file.path) {
            return Err(CosmeticsDiagnostic::MissingOrDuplicateShard);
        }
        let data = supplied
            .get(file.path.as_str())
            .ok_or(CosmeticsDiagnostic::MissingOrDuplicateShard)?;
        if hash(&file.sha256) != Some(sha(data)) {
            return Err(CosmeticsDiagnostic::ShardDigest { shard: index });
        }
        let records: Observations =
            serde_json::from_slice(data).map_err(|_| CosmeticsDiagnostic::MalformedSource)?;
        if records.schema != "OTERYN_CIPSOFT_APPEARANCE_OUTFIT_SOURCE_OBSERVATIONS/v1"
            || records.family != "Outfit"
            || records.records.len() != file.records
        {
            return Err(CosmeticsDiagnostic::MalformedSource);
        }
        for record in records.records {
            let digest = hash(&record.source_record_sha256)
                .ok_or(CosmeticsDiagnostic::SourceRecord { record: ordinal })?;
            if record.source_id == 0
                || record.source_id > u32::from(u16::MAX)
                || record.source_index != ordinal
                || looks.insert(record.source_id, digest).is_some()
            {
                return Err(CosmeticsDiagnostic::SourceRecord { record: ordinal });
            }
            ordinal += 1;
        }
    }
    if ordinal != manifest.record_count {
        return Err(CosmeticsDiagnostic::MalformedSource);
    }
    Ok(looks)
}

fn look_matches(look: &LookRef, source: &BTreeMap<u32, [u8; 32]>) -> bool {
    hash(&look.source_record_sha256)
        .as_ref()
        .is_some_and(|digest| source.get(&look.id) == Some(digest))
}
fn identity(key: &str, revision: &str) -> bool {
    text(revision, 128)
        && ProductionKey::new(key).is_ok()
        && DefinitionRevisionRef::new(revision).is_ok()
}
fn text(value: &str, max: usize) -> bool {
    !value.is_empty() && value.len() <= max && !value.chars().any(char::is_control)
}
fn plain_name(value: &str) -> bool {
    text(value, 255) && value.ends_with(".json") && !value.contains(['/', '\\']) && value != ".."
}
fn sha(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
fn hash(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return None;
    }
    let mut out = [0; 32];
    for (slot, pair) in out.iter_mut().zip(value.as_bytes().chunks_exact(2)) {
        *slot = u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()?;
    }
    (out != [0; 32]).then_some(out)
}
struct NoEarnedUnlocks;
impl AccountUnlocks<String> for NoEarnedUnlocks {
    fn outfit(&self, _: &String) -> UnlockFact {
        UnlockFact::Absent
    }
    fn addon(&self, _: &String, _: u8) -> UnlockFact {
        UnlockFact::Absent
    }
    fn mount(&self, _: &String) -> UnlockFact {
        UnlockFact::Absent
    }
}

#[cfg(test)]
mod tests;
