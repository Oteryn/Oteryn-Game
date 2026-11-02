//! Explicit opt-in to the native shared defence/summon clock.
use super::*;
use crate::ai_think::profile_schedule::{MAX_MONSTER_SUMMONS, MAX_SUMMON_ENTRIES};

impl CreatureBehaviourProfile {
    pub fn project_with_summons(
        behavior: Option<&ProjectV2BehaviorAuthoring>,
        abilities: &BTreeMap<ProjectV2DefinitionRef, ProjectV2AbilityAuthoring>,
    ) -> Result<Self, ProfileRefusal> {
        Self::project(behavior, abilities)?.with_summons(behavior)
    }

    pub(super) fn with_summons(
        mut self,
        behavior: Option<&ProjectV2BehaviorAuthoring>,
    ) -> Result<Self, ProfileRefusal> {
        if let Some(summons) = behavior.and_then(|b| b.summons.as_ref()) {
            let mut entries = std::collections::BTreeSet::new();
            if summons.entries.is_empty()
                || summons.entries.len() > MAX_SUMMON_ENTRIES
                || !(1..=MAX_MONSTER_SUMMONS).contains(&summons.max_summons)
                || summons.entries.iter().any(|entry| {
                    entry.interval_ms == 0
                        || entry.chance_ppm > 1_000_000
                        || !(1..=MAX_MONSTER_SUMMONS).contains(&entry.count)
                        || entry.creature.family != crate::content::ProjectV2Family::Creature
                        // Crystal Eshtaba authors repeated creature names with distinct
                        // chances. Native entry order is execution identity, not a set.
                        || (!self.native && !entries.insert(&entry.creature))
                })
            {
                return Err(ProfileRefusal::Invalid);
            }
            self.behavior.summons = Some(summons.clone());
        }
        Ok(self)
    }
}
