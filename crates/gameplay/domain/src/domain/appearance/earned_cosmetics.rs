//! Combine earned facts with content's explicit grant compatibility declarations.
use super::{AccountUnlocks, AppearanceContent, EarnedCosmetic, Sex, UnlockFact};

pub struct EarnedCosmetics<'a, C> {
    pub content: &'a C,
    pub facts: &'a [(String, String)],
}
impl<'a, C: AppearanceContent<String>> EarnedCosmetics<'a, C> {
    fn resolve(&self, target: EarnedCosmetic<'_>) -> UnlockFact {
        self.fact_for(target, None)
    }
    pub fn with_sex(self, sex: Sex) -> SexQualifiedEarnedCosmetics<'a, C> {
        SexQualifiedEarnedCosmetics { inner: self, sex }
    }
    pub fn fact_for(&self, target: EarnedCosmetic<'_>, sex: Option<Sex>) -> UnlockFact {
        let mut result = UnlockFact::Absent;
        for (key, provenance) in self.facts {
            let fact = match sex {
                Some(sex) => self
                    .content
                    .earned_unlock_for_sex(key, provenance, target, sex),
                None => self.content.earned_unlock(key, provenance, target),
            };
            match fact {
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

/// Projects the same immutable account facts for the selecting character's sex.
pub struct SexQualifiedEarnedCosmetics<'a, C> {
    inner: EarnedCosmetics<'a, C>,
    sex: Sex,
}
impl<C: AppearanceContent<String>> AccountUnlocks<String> for SexQualifiedEarnedCosmetics<'_, C> {
    fn outfit(&self, key: &String) -> UnlockFact {
        self.inner
            .fact_for(EarnedCosmetic::Outfit(key), Some(self.sex))
    }
    fn addon(&self, key: &String, addon: u8) -> UnlockFact {
        self.inner
            .fact_for(EarnedCosmetic::Addon(key, addon), Some(self.sex))
    }
    fn mount(&self, key: &String) -> UnlockFact {
        self.inner
            .fact_for(EarnedCosmetic::Mount(key), Some(self.sex))
    }
}
