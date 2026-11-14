//! The immutable cosmetics view of an admitted native artifact.
use super::{Arc, cosmetics_loader};
use crate::domain::appearance::{
    AppearanceContent, AppearanceSelection, EarnedCosmetic, Sex, UnlockFact,
};

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
    fn projection_fallback(&self, sex: Sex) -> Option<AppearanceSelection<String>> {
        Some(self.content.fallback(sex))
    }
    fn earned_grant_keys(&self) -> Vec<String> {
        let mut keys = self.content.earned_grant_keys();
        for selector in [17, 18] {
            if let Some(key) = self.drome_grant(selector) {
                keys.push(key.into());
            }
        }
        if let Some(offers) = crate::content::store_cosmetics::offers() {
            keys.extend(
                offers
                    .iter()
                    .filter(|offer| crate::content::store_cosmetics::compatible(self, offer))
                    .map(|offer| offer.key.clone()),
            );
        }
        keys.push(MARRIAGE_GRANT_KEY.into());
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
        if let Some(fact) =
            crate::content::store_cosmetics::unlock(self, grant_key, provenance, target)
        {
            return fact;
        }
        if grant_key == MARRIAGE_GRANT_KEY {
            let compatible = provenance == MARRIAGE_PROVENANCE
                && match target {
                    EarnedCosmetic::Outfit(key) => {
                        self.look_type(key, crate::domain::appearance::Sex::Male) == Some(328)
                            && self.look_type(key, crate::domain::appearance::Sex::Female)
                                == Some(329)
                    }
                    _ => false,
                };
            return if compatible {
                UnlockFact::Compatible
            } else {
                UnlockFact::Incompatible
            };
        }
        if provenance == DROME_PROVENANCE {
            let compatible = match (grant_key, target) {
                (DROME_PHANT, EarnedCosmetic::Mount(key)) => self
                    .mount_metadata(key)
                    .is_some_and(|(look, _, _)| look == 1417),
                (DROME_LION, EarnedCosmetic::Outfit(key)) => {
                    self.look_type(key, crate::domain::appearance::Sex::Female) == Some(1207)
                        && self.look_type(key, crate::domain::appearance::Sex::Male) == Some(1206)
                }
                _ => false,
            };
            return if compatible {
                UnlockFact::Compatible
            } else {
                UnlockFact::Incompatible
            };
        }
        self.content.earned_unlock(grant_key, provenance, target)
    }
    fn earned_unlock_for_sex(
        &self,
        grant_key: &str,
        provenance: &str,
        target: EarnedCosmetic<'_>,
        sex: crate::domain::appearance::Sex,
    ) -> UnlockFact {
        if provenance == DROME_PROVENANCE
            || provenance == crate::content::store_cosmetics::PROVENANCE
        {
            self.earned_unlock(grant_key, provenance, target)
        } else {
            self.content
                .earned_unlock_for_sex(grant_key, provenance, target, sex)
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

// Crystal 201189f data/XML/mounts.xml:211; data/XML/outfits.xml:112,244.
// These are explicit grant mappings against already admitted native definitions.
pub const DROME_PROVENANCE: &str = "crystal-201189f-tibiadrome-v1";
const DROME_PHANT: &str = "oteryn:appearance-grant/tibiadrome/phant";
const DROME_LION: &str = "oteryn:appearance-grant/tibiadrome/lion-of-war-base";
impl NativeCosmeticsView {
    pub fn drome_grant(&self, selector: u8) -> Option<&'static str> {
        match selector {
            17 if self.mount_keys().any(|key| {
                self.mount_metadata(key)
                    .is_some_and(|(look, _, _)| look == 1417)
            }) =>
            {
                Some(DROME_PHANT)
            }
            18 if self.outfit_keys().any(|key| {
                self.look_type(key, crate::domain::appearance::Sex::Female) == Some(1207)
                    && self.look_type(key, crate::domain::appearance::Sex::Male) == Some(1206)
            }) =>
            {
                Some(DROME_LION)
            }
            _ => None,
        }
    }
}

// Crystal201189f outfits.xml:52,184; Lynda grants both base looks at engagement.
pub const MARRIAGE_GRANT_KEY: &str = "oteryn:appearance-grant/npc/lynda/newly-wed-base";
pub const MARRIAGE_PROVENANCE: &str = "crystal-201189f-lynda-marriage-v1";
