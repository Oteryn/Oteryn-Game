//! Presentation-only effect categories authenticated by the asset manifest pin.
use std::collections::BTreeSet;
#[derive(Debug, Clone, Default)]
pub struct EffectCatalog {
    boss_area: BTreeSet<u32>,
}
impl EffectCatalog {
    pub(crate) fn from_ids(ids: Vec<u32>) -> Result<Self, &'static str> {
        if ids.len() > usize::from(u16::MAX)
            || ids.iter().any(|id| !(1..=u32::from(u16::MAX)).contains(id))
        {
            return Err("boss-area effect ids must be in 1..=65535");
        }
        let count = ids.len();
        let boss_area: BTreeSet<_> = ids.into_iter().collect();
        if boss_area.len() != count {
            return Err("duplicate boss-area effect id");
        }
        Ok(Self { boss_area })
    }
    #[must_use]
    pub fn is_boss_area(&self, id: u32) -> bool {
        self.boss_area.contains(&id)
    }
}
