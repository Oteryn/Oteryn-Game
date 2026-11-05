//! The immutable cosmetics view of an admitted native artifact.
use super::{Arc, cosmetics_loader};
use crate::domain::appearance::{AppearanceContent, EarnedCosmetic, UnlockFact};

/// Immutable decoded definitions and sorted identities from this exact native artifact.
/// This is borrowed data; ActiveGeneration remains the current admission authority.
#[derive(Debug, Clone)]
pub struct NativeCosmeticsView {
    pub(super) source_digest: [u8; 32],
    pub(super) content: Arc<cosmetics_loader::CosmeticsCandidate>,
    pub(super) outfit_keys: Arc<[String]>,
    pub(super) mount_keys: Arc<[String]>,
}
impl NativeCosmeticsView {
    pub fn source_digest(&self) -> [u8; 32] {
        self.source_digest
    }
    pub fn outfit_keys(&self) -> impl Iterator<Item = &str> {
        self.outfit_keys.iter().map(String::as_str)
    }
    pub fn mount_keys(&self) -> impl Iterator<Item = &str> {
        self.mount_keys.iter().map(String::as_str)
    }
}
impl std::ops::Deref for NativeCosmeticsView {
    type Target = cosmetics_loader::CosmeticsCandidate;
    fn deref(&self) -> &Self::Target {
        &self.content
    }
}
impl AppearanceContent<String> for NativeCosmeticsView {
    fn earned_grant_keys(&self) -> Vec<String> {
        let mut keys = self.content.earned_grant_keys();
        keys.extend(super::item_action_grants::keys());
        keys.sort();
        keys.dedup();
        keys
    }
    fn earned_unlock(
        &self,
        grant_key: &str,
        provenance: &str,
        target: EarnedCosmetic<'_>,
    ) -> UnlockFact {
        match self.content.earned_unlock(grant_key, provenance, target) {
            UnlockFact::Absent => {
                super::item_action_grants::resolve(self, grant_key, provenance, target)
            }
            result => result,
        }
    }
    fn palette_size(&self) -> u16 {
        self.content.palette_size()
    }
    fn outfit_addon_count(&self, key: &String) -> Option<u8> {
        self.content.outfit_addon_count(key)
    }
    fn outfit_is_starter(&self, key: &String, sex: crate::domain::appearance::Sex) -> bool {
        self.content.outfit_is_starter(key, sex)
    }
    fn addon_is_starter(
        &self,
        key: &String,
        sex: crate::domain::appearance::Sex,
        addon: u8,
    ) -> bool {
        self.content.addon_is_starter(key, sex, addon)
    }
    fn mount_exists(&self, key: &String) -> bool {
        self.content.mount_exists(key)
    }
}
