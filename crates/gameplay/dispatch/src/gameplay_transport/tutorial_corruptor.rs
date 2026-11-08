//! Prepare guaranteed gear after ordinary loot transforms; MINT consumes the arm atomically.
use super::*;
use crate::domain::loot_data::{
    LootDefinitionRef, LootEntryDetails, LootTableDefinition, LootTableEntry,
};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::tutorial_corruptor::TutorialCorruptorClaim;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn tutorial_corruptor_table(
        &self,
        fence: CurrentCharacterGameplayFence,
        entry: &kill_reward::PendingKillSettlement,
        mut table: LootTableDefinition,
    ) -> Result<(LootTableDefinition, Option<TutorialCorruptorClaim>), ()> {
        if entry.creature != "oteryn:creature.the_corruptor"
            || entry.facts.top_damage_character.as_bytes() != fence.character_id.as_bytes()
            || !self.tutorial_enabled().await
        {
            return Ok((table, None));
        }
        let Some(claim) = self
            .root
            .prepare_tutorial_corruptor(self.character, self.holder, fence, entry.facts.death)
            .await
            .map_err(|_| ())?
        else {
            return Ok((table, None));
        };
        let definitions = {
            let runtime = self.runtime.lock().await;
            let index =
                item_ref_admission::pinned_index(&runtime, self.active_generation).ok_or(())?;
            claim
                .items()
                .iter()
                .map(|id| {
                    let key = format!("oteryn:item.tibia.i{id}");
                    let revision = index.revision(&key).ok_or(())?;
                    Ok(LootDefinitionRef::new("Item", key, revision))
                })
                .collect::<Result<Vec<_>, ()>>()?
        };
        // Requeues and historical recapture share this cell for the exact retained
        // corpse. Freeze ordinary stamina/event/duplex results before any MINT.
        table = entry.tutorial_loot.get_or_init(|| table).clone();
        let first = source_draw_count(&table.entries).ok_or(())?;
        let claim = claim.bind_draw_ordinals(first).ok_or(())?;
        // Append after all ordinary transforms. The DailyBoosted prefix remains
        // ordinary loot, and each guarantee has its own source ordinal.
        table
            .entries
            .extend(definitions.into_iter().map(guaranteed_entry));
        Ok((table, Some(claim)))
    }
}

fn source_draw_count(entries: &[LootTableEntry]) -> Option<u32> {
    entries.iter().try_fold(0_u32, |count, entry| {
        count
            .checked_add(1)?
            .checked_add(source_draw_count(&entry.details.children)?)
    })
}

fn guaranteed_entry(item: LootDefinitionRef) -> LootTableEntry {
    LootTableEntry {
        item,
        min_count: 1,
        max_count: 1,
        probability_ppm: Some(1_000_000),
        // Ordinary rolls apply a 95..=105% dynamic factor even at 100% chance.
        // Existing rated chances allow 2M: 2M * 95% exceeds every 0..1M draw.
        details: LootEntryDetails {
            rated_probability_ppm: Some(2_000_000),
            ..Default::default()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_gameplay_items::loot_plan::{LootPlanDeathKey, plan_creature_loot};

    #[test]
    fn newhaven_corruptor_guarantee_survives_low_factor_and_keeps_independent_ordinary_drop()
    -> Result<(), String> {
        let item = LootDefinitionRef::new("Item", "oteryn:item.tibia.i3359", "definition-r1");
        let reference = LootDefinitionRef::new(
            "Loot",
            "oteryn:loot.creature.the_corruptor",
            "definition-r1",
        );
        let mut ordinary = guaranteed_entry(item.clone());
        ordinary.details.rated_probability_ppm = None;
        let fixed = LootTableDefinition {
            algorithm: crate::combat::LootSelectionAlgorithm::IndependentBernoulliPpm,
            entries: vec![ordinary.clone(), guaranteed_entry(item)],
        };
        let legacy = LootTableDefinition {
            algorithm: fixed.algorithm,
            entries: vec![ordinary.clone(), ordinary],
        };
        let mut low_factor_witness = None;
        let mut ordinary_witness = false;
        for generation in 1..=4096 {
            let death = LootPlanDeathKey::new([1; 16], [2; 16], 1, 2, generation);
            let old =
                plan_creature_loot(death, &reference, &legacy).map_err(|e| format!("{e:?}"))?;
            let new =
                plan_creature_loot(death, &reference, &fixed).map_err(|e| format!("{e:?}"))?;
            assert_eq!(
                new.entries.iter().filter(|e| e.draw_ordinal == 1).count(),
                1
            );
            if !old.entries.iter().any(|e| e.draw_ordinal == 1) {
                low_factor_witness = Some(generation);
            }
            if old.entries.iter().any(|e| e.draw_ordinal == 0) {
                ordinary_witness = true;
                assert_eq!(
                    new.entries.len(),
                    2,
                    "ordinary and special drops have distinct causes"
                );
            }
        }
        assert!(
            low_factor_witness.is_some(),
            "exercise a nominal100% entry rejected by low factor/high draw"
        );
        assert!(ordinary_witness);
        Ok(())
    }
}
