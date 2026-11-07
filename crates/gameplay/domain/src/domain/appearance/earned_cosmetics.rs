//! Combine earned facts with content's explicit grant compatibility declarations.
use super::{AccountUnlocks, AppearanceContent, EarnedCosmetic, UnlockFact};

pub struct EarnedCosmetics<'a, C> {
    pub content: &'a C,
    pub facts: &'a [(String, String)],
}
impl<C: AppearanceContent<String>> EarnedCosmetics<'_, C> {
    fn resolve(&self, target: EarnedCosmetic<'_>) -> UnlockFact {
        let mut result = UnlockFact::Absent;
        for (key, provenance) in self.facts {
            match self.content.earned_unlock(key, provenance, target) {
                UnlockFact::Compatible => return UnlockFact::Compatible,
                UnlockFact::Incompatible => result = UnlockFact::Incompatible,
                UnlockFact::Absent => {}
            }
        }
        result
    }
}
impl<C: AppearanceContent<String>> AccountUnlocks<String> for EarnedCosmetics<'_, C> {
    fn outfit(&self, outfit: &String) -> UnlockFact {
        self.resolve(EarnedCosmetic::Outfit(outfit))
    }
    fn addon(&self, outfit: &String, addon: u8) -> UnlockFact {
        self.resolve(EarnedCosmetic::Addon(outfit, addon))
    }
    fn mount(&self, mount: &String) -> UnlockFact {
        self.resolve(EarnedCosmetic::Mount(mount))
    }
}

/// A view for an authoritative consumer after it has read the real Character's sex.
pub struct SexBoundEarnedCosmetics<'a, C> {
    content: &'a C,
    facts: &'a [(String, String)],
    sex: super::Sex,
}
impl<'a, C: AppearanceContent<String>> EarnedCosmetics<'a, C> {
    pub fn with_sex(&self, sex: super::Sex) -> SexBoundEarnedCosmetics<'a, C> {
        SexBoundEarnedCosmetics {
            content: self.content,
            facts: self.facts,
            sex,
        }
    }
    /// NPC source predicates may ask about a concrete earned look of either sex.
    /// This never implements AccountUnlocks or authorizes an appearance selection.
    pub fn exact_source_look_addon(&self, look: u32, addon: u8) -> UnlockFact {
        let mut result = UnlockFact::Absent;
        for (key, provenance) in self.facts {
            match self
                .content
                .earned_source_look_addon(key, provenance, look, addon)
            {
                UnlockFact::Compatible => return UnlockFact::Compatible,
                UnlockFact::Incompatible => result = UnlockFact::Incompatible,
                UnlockFact::Absent => {}
            }
        }
        result
    }
}
impl<C: AppearanceContent<String>> SexBoundEarnedCosmetics<'_, C> {
    pub fn exact_source_look_addon(&self, look: u32, addon: u8) -> UnlockFact {
        EarnedCosmetics {
            content: self.content,
            facts: self.facts,
        }
        .exact_source_look_addon(look, addon)
    }
    fn resolve(&self, target: EarnedCosmetic<'_>) -> UnlockFact {
        let mut result = UnlockFact::Absent;
        for (key, provenance) in self.facts {
            match self
                .content
                .earned_unlock_for_sex(key, provenance, target, self.sex)
            {
                UnlockFact::Compatible => return UnlockFact::Compatible,
                UnlockFact::Incompatible => result = UnlockFact::Incompatible,
                UnlockFact::Absent => {}
            }
        }
        result
    }
}
impl<C: AppearanceContent<String>> AccountUnlocks<String> for SexBoundEarnedCosmetics<'_, C> {
    fn outfit(&self, key: &String) -> UnlockFact {
        self.resolve(EarnedCosmetic::Outfit(key))
    }
    fn addon(&self, key: &String, bit: u8) -> UnlockFact {
        self.resolve(EarnedCosmetic::Addon(key, bit))
    }
    fn mount(&self, key: &String) -> UnlockFact {
        self.resolve(EarnedCosmetic::Mount(key))
    }
}
