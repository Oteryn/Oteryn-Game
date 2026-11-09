use super::UpdateError;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub platform: String,
    pub architecture: String,
    pub target_path: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdatePolicy {
    pub schema_version: u32,
    pub policy_revision: u64,
    pub channel: String,
    pub current_release_id: String,
    pub current_release_sequence: u64,
    pub current_version_display: String,
    pub minimum_supported_release_sequence: u64,
    pub update_mode: String,
    pub artifacts: Vec<Artifact>,
    pub revoked_release_ids: Vec<String>,
    pub revoked_artifact_targets: Vec<String>,
    pub rollback_authorization: String,
}
fn identifier(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 96
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        && s != "."
        && s != ".."
}
impl UpdatePolicy {
    pub(super) fn validate(&self, channel: &str) -> Result<(), UpdateError> {
        use std::collections::BTreeSet;
        if self.schema_version != 1
            || self.channel != channel
            || self.policy_revision == 0
            || self.current_release_sequence == 0
            || self.minimum_supported_release_sequence == 0
            || self.minimum_supported_release_sequence > self.current_release_sequence
            || !identifier(&self.current_release_id)
            || self.current_version_display.is_empty()
            || self.current_version_display.len() > 96
            || self.current_version_display.chars().any(char::is_control)
            || !matches!(
                self.update_mode.as_str(),
                "optional" | "recommended" | "required"
            )
            || !matches!(self.rollback_authorization.as_str(), "none" | "explicit")
            || self.artifacts.is_empty()
            || self.artifacts.len() > 32
            || self.revoked_release_ids.len() > 1024
            || self.revoked_artifact_targets.len() > 1024
            || self.revoked_release_ids.contains(&self.current_release_id)
        {
            return Err(UpdateError::Verification);
        }
        let mut variants = BTreeSet::new();
        let mut paths = BTreeSet::new();
        for a in &self.artifacts {
            let prefix = format!(
                "channels/{channel}/releases/{}/{}/{}/",
                self.current_release_id, a.platform, a.architecture
            );
            if !matches!(a.platform.as_str(), "windows" | "linux" | "macos")
                || !matches!(a.architecture.as_str(), "x86_64" | "arm64" | "x86")
                || !a.target_path.starts_with(&prefix)
                || !a.target_path.split('/').all(identifier)
                || self.revoked_artifact_targets.contains(&a.target_path)
                || !variants.insert((&a.platform, &a.architecture))
                || !paths.insert(&a.target_path)
            {
                return Err(UpdateError::Verification);
            }
        }
        if self.revoked_release_ids.iter().any(|s| !identifier(s))
            || self.revoked_artifact_targets.iter().any(|s| {
                !s.starts_with(&format!("channels/{channel}/")) || !s.split('/').all(identifier)
            })
        {
            return Err(UpdateError::Verification);
        }
        Ok(())
    }
}
