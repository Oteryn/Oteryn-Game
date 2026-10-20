//! Immutable source-qualified build training profile.
use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrainingError {
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
pub(crate) struct VocationProfile {
    pub(crate) key: String,
    pub(crate) mana_multiplier: String,
    pub(crate) skill_multipliers: [String; 7],
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Profile {
    schema: String,
    source: Source,
    pub(crate) magic_base: u16,
    pub(crate) skill_bases: [u16; 7],
    multiplier_storage: String,
    pub(crate) vocations: Vec<VocationProfile>,
}
#[derive(Debug, Clone)]
pub(crate) struct CompiledTrainingFormula {
    pub(crate) profile: Profile,
    pub(crate) content_revision: String,
    pub(crate) digest: [u8; 32],
}
impl CompiledTrainingFormula {
    pub(crate) fn from_profile(
        bytes: &[u8],
        content_revision: &str,
    ) -> Result<Self, TrainingError> {
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
            "../../../../tools/content-schema/native-gameplay/build-training.json"
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
    pub(crate) fn content_revision(&self) -> &str {
        &self.content_revision
    }
}
