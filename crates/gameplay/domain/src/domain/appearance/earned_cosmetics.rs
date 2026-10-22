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
