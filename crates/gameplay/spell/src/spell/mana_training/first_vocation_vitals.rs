//! Install a committed first vocation without replacing the vitals/conditions owner.
use super::*;
use crate::spell::Vocation;
use crate::spell::cast::{CharacterCastFacts, PlayerSpellState};
use oteryn_simulation_determinism::SemanticTimeMicros;

impl PlayerSpellState {
    /// Stage the already-committed build conversion. Exact installation replay preserves even
    /// subsequently accrued training. A different class or retained checkpoint is refused.
    pub fn after_first_vocation(
        &self,
        before: &DurableBuildState,
        after: &DurableBuildState,
        facts: CharacterCastFacts,
        formula: &CompiledTrainingFormula,
        now: SemanticTimeMicros,
    ) -> Option<Self> {
        if before.vocation() != "none"
            || facts.vocation == Vocation::None
            || Vocation::from_key(after.vocation()) != Some(facts.vocation)
            || facts.magic_level != u32::from(after.magic().0)
        {
            return None;
        }
        if self.facts == facts {
            return (self
                .training
                .as_ref()
                .is_none_or(|live| live.durable() == after)
                && self.owned_skills == Some(after.skills().map(|(level, _)| level)))
            .then(|| self.clone());
        }
        if self.facts.vocation != Vocation::None
            || self.facts.level != facts.level
            || self.facts.magic_level != u32::from(before.magic().0)
            || self.owned_skills != Some(before.skills().map(|(level, _)| level))
            || self
                .training
                .as_ref()
                .is_some_and(|live| live.durable() != before)
            || self.training_checkpoint.is_some()
            || self.pending_stance.is_some()
            || self.stance_key.is_some()
            || self.health > facts.max_health
            || self.mana > facts.max_mana
            || self.soul > facts.max_soul
        {
            return None;
        }
        // Use the same constructor to validate maxima, level and monk initialization bounds.
        let validated = Self::new(facts, 0, 0)?;
        let mut next = self.clone();
        next.facts = facts;
        next.base_speed = validated.base_speed;
        next.monk = validated.monk;
        next.make_playable(now).ok()?;
        next.owned_skills = Some(after.skills().map(|(level, _)| level));
        if let Some(live) = &mut next.training {
            // Keep hit streaks, timestamps and any tries accrued while the commit was in flight.
            live.durable = after.clone();
        }
        next.enable_owned_training(after, formula).ok()?;
        next.revision = next.revision.checked_add(1)?;
        Some(next)
    }
}
