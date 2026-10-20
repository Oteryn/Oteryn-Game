//! Immutable source-qualified build training profile.
use crate::domain::character_build::{BuildFormula, MAX_BUILD_LEVEL, skill_tries_required};
use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrainingError {
    InvalidProfile,
    UnknownVocation,
    InvalidBuild,
    StaleTime,
    StaleReceipt,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceFile {
    path: String,
    sha256: String,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    repository: String,
    revision: String,
    files: Vec<SourceFile>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VocationProfile {
    pub key: String,
    pub mana_multiplier: String,
    pub skill_multipliers: [String; 7],
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    schema: String,
    source: Source,
    pub magic_base: u16,
    pub skill_bases: [u16; 7],
    multiplier_storage: String,
    pub vocations: Vec<VocationProfile>,
}
#[derive(Debug, Clone)]
pub struct CompiledTrainingFormula {
    pub profile: Profile,
    pub content_revision: String,
    pub digest: [u8; 32],
}
impl CompiledTrainingFormula {
    pub fn from_profile(bytes: &[u8], content_revision: &str) -> Result<Self, TrainingError> {
        if bytes.len() > 32768
            || content_revision.is_empty()
            || content_revision.len() > 128
            || !content_revision.bytes().all(|b| b.is_ascii_graphic())
        {
            return Err(TrainingError::InvalidProfile);
        }
        let profile: Profile =
            serde_json::from_slice(bytes).map_err(|_| TrainingError::InvalidProfile)?;
        // This independently qualified extraction is evidence, never the active
        // runtime artifact. The Content caller still owns bytes/pin/revision.
        let evidence: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../tools/content-schema/native-gameplay/build-training.json"
        ))
        .map_err(|_| TrainingError::InvalidProfile)?;
        let supplied: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|_| TrainingError::InvalidProfile)?;
        if supplied != evidence
            || profile.schema != "OTERYN_BUILD_TRAINING_PROFILE/v1"
            || profile.multiplier_storage != "ieee754_binary32"
            || profile.magic_base != 1600
            || profile.vocations.len() != 11
            || profile.source.files.len() != 3
            || profile.source.repository != "opentibiabr/canary"
            || profile.source.revision.len() != 40
            || profile
                .source
                .files
                .iter()
                .any(|f| f.path.is_empty() || f.sha256.len() != 64)
        {
            return Err(TrainingError::InvalidProfile);
        }
        for v in &profile.vocations {
            for value in std::iter::once(&v.mana_multiplier).chain(v.skill_multipliers.iter()) {
                let value = value
                    .parse::<f32>()
                    .map_err(|_| TrainingError::InvalidProfile)?;
                if !value.is_finite() || value <= 0.0 {
                    return Err(TrainingError::InvalidProfile);
                }
            }
        }
        let digest = Sha256::new()
            .chain_update(b"oteryn:build-training:v1\0")
            .chain_update(bytes)
            .chain_update(content_revision.as_bytes())
            .finalize()
            .into();
        Ok(Self {
            profile,
            content_revision: content_revision.to_owned(),
            digest,
        })
    }
    pub fn content_revision(&self) -> &str {
        &self.content_revision
    }
}

impl BuildFormula for CompiledTrainingFormula {
    fn required(&self, vocation: &str, family: usize, level: u16) -> Option<u64> {
        let v = self.profile.vocations.iter().find(|v| v.key == vocation)?;
        if family == 0 {
            if !(1..=MAX_BUILD_LEVEL).contains(&level) {
                return None;
            }
            // Source stores the XML literal in float, then promotes it to double.
            let multiplier = f64::from(v.mana_multiplier.parse::<f32>().ok()?);
            let value = f64::from(self.profile.magic_base) * multiplier.powf(f64::from(level - 1));
            (value.is_finite() && value < 9_223_372_036_854_775_808.0)
                .then_some(value.floor() as u64)
        } else {
            // SKILLS-0 §2: a promoted vocation trains skills with its base vocation's row (the
            // pinned elite_knight row swaps fist and distance, a Canary data error).
            let v = self
                .profile
                .vocations
                .iter()
                .find(|row| row.key == skill_vocation(vocation))?;
            let index = family.checked_sub(1)?;
            skill_tries_required(
                *self.profile.skill_bases.get(index)?,
                f64::from(v.skill_multipliers.get(index)?.parse::<f32>().ok()?),
                level,
            )
        }
    }
    fn digest(&self) -> [u8; 32] {
        self.digest
    }
    fn content_revision(&self) -> &str {
        &self.content_revision
    }
}

/// The base vocation whose skill multipliers a promoted vocation uses (SKILLS-0 §2).
fn skill_vocation(vocation: &str) -> &str {
    match vocation {
        "elder_druid" => "druid",
        "elite_knight" => "knight",
        "exalted_monk" => "monk",
        "master_sorcerer" => "sorcerer",
        "royal_paladin" => "paladin",
        other => other,
    }
}
