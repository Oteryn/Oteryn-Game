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
        if before.vocation() != "none" {
            return None;
        }
        self.after_vocation_change(before, after, facts, formula, now, true)
    }

    /// S16c: a Dawnport trial tile switched the committed vocation below level 20. Crystal
    /// 201189f8 data-global/lib/others/dawnport.lua:97-118 refills both pools only above level
    /// 8; up to level 8 the current pools are kept.
    pub fn after_vocation_trial(
        &self,
        before: &DurableBuildState,
        after: &DurableBuildState,
        facts: CharacterCastFacts,
        formula: &CompiledTrainingFormula,
        now: SemanticTimeMicros,
    ) -> Option<Self> {
        if before.vocation() == after.vocation() || facts.level >= 20 {
            return None;
        }
        self.after_vocation_change(before, after, facts, formula, now, facts.level > 8)
    }

    fn after_vocation_change(
        &self,
        before: &DurableBuildState,
        after: &DurableBuildState,
        facts: CharacterCastFacts,
        formula: &CompiledTrainingFormula,
        now: SemanticTimeMicros,
        refill: bool,
    ) -> Option<Self> {
        if facts.vocation == Vocation::None
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
        if Some(self.facts.vocation) != Vocation::from_key(before.vocation())
            || self.facts.level != facts.level
            || self.facts.magic_level != u32::from(before.magic().0)
            || self.owned_skills != Some(before.skills().map(|(level, _)| level))
            || self
                .training
                .as_ref()
                .is_some_and(|live| live.durable() != before)
            || self.training_checkpoint.is_some()
            || self.pending_stance.is_some()
            || self.health == 0
            || self.soul > facts.max_soul
        {
            return None;
        }
        // Use the same constructor to validate maxima, level and monk initialization bounds.
        let validated = Self::new(facts, 0, 0)?;
        let mut next = self.clone();
        // A vocationless actor, or one below level 20 (Dawnport trial), has no Wheel
        // contribution, so the new maxima are the base.
        next.base_max_health = facts.max_health;
        next.base_max_mana = facts.max_mana;
        next.facts = facts;
        // Dawnport changeVocation: recompute as if levelled from 8, then fill both pools.
        // The existing stance key/conditions survive the donor's setVocation call.
        next.health = if refill {
            facts.max_health
        } else {
            self.health.min(facts.max_health)
        };
        next.mana = if refill {
            facts.max_mana
        } else {
            self.mana.min(facts.max_mana)
        };
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
