//! Add the claimed vocation Items to the existing deterministic corpse loot path.
use super::*;
use crate::domain::loot_data::{LootDefinitionRef, LootTableDefinition, LootTableEntry};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use oteryn_gameplay_quests::tutorial::corruptor_items;
use std::collections::BTreeMap;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn tutorial_corruptor_table(
        &self,
        fence: CurrentCharacterGameplayFence,
        entry: &kill_reward::PendingKillSettlement,
    ) -> Result<LootTableDefinition, ()> {
        let mut table = entry.row.as_ref().ok_or(())?.loot_table.clone();
        if entry.creature != "oteryn:creature.the_corruptor" || !self.tutorial_enabled().await {
            return Ok(table);
        }
        let build = self
            .root
            .read_character_build_state(self.character, fence.character_id)
            .await
            .map_err(|_| ())?;
        let definitions = {
            let runtime = self.runtime.lock().await;
            let index =
                item_ref_admission::pinned_index(&runtime, self.active_generation).ok_or(())?;
            let mut definitions = BTreeMap::new();
            for id in [3031, 3723, 2853, 3338, 3359, 3285, 3375] {
                let key = format!("oteryn:item.tibia.i{id}");
                if let Some(revision) = index.revision(&key) {
                    definitions.insert(id, LootDefinitionRef::new("Item", key, revision));
                }
            }
            definitions
        };
        append_common(&mut table, &definitions)?;
        let desired = corruptor_items(build.vocation());
        let defined = if desired.iter().all(|id| definitions.contains_key(id)) {
            desired.to_vec()
        } else {
            Vec::new()
        };
        let claimed = self
            .root
            .claim_tutorial_corruptor(
                self.character,
                self.holder,
                fence,
                entry.facts.death,
                defined,
            )
            .await
            .map_err(|_| ())?;
        for id in claimed {
            table.entries.push(LootTableEntry {
                item: definitions.get(&id).ok_or(())?.clone(),
                min_count: 1,
                max_count: 1,
                probability_ppm: Some(1_000_000),
                details: Default::default(),
            });
        }
        Ok(table)
    }
}

/// Extend the admitted monster table; callback rows have their own deterministic rolls.
fn append_common(
    table: &mut LootTableDefinition,
    definitions: &BTreeMap<u32, LootDefinitionRef>,
) -> Result<(), ()> {
    // The donor callback adds these rows to the monster's ordinary loot.
    // Its 0..3 mushroom roll is P(positive)=3/4, uniform 1..3 thereafter.
    // Contradictory callback ID 7618 stays unavailable (Q10).
    for (id, min_count, max_count, probability_ppm, stackable) in [
        (3031, 1, 50, 1_000_000, true),
        (3723, 1, 3, 750_000, true),
        (2853, 1, 1, 500_000, false),
        (3338, 1, 1, 500_000, false),
    ] {
        table.entries.push(LootTableEntry {
            item: definitions.get(&id).ok_or(())?.clone(),
            min_count,
            max_count,
            probability_ppm: Some(probability_ppm),
            details: crate::domain::loot_data::LootEntryDetails {
                stackable,
                container_capacity: (id == 2853).then_some(8),
                ..Default::default()
            },
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::loot_data::LootSelectionAlgorithm;

    #[test]
    fn tutorial_corruptor_common_rows_preserve_native_loot_and_exclude_conflicts() {
        let definitions = [3031, 3723, 2853, 3338]
            .into_iter()
            .map(|id| {
                (
                    id,
                    LootDefinitionRef::new(
                        "Item",
                        format!("oteryn:item.tibia.i{id}"),
                        "definition-r1",
                    ),
                )
            })
            .collect();
        let mut table = LootTableDefinition {
            algorithm: LootSelectionAlgorithm::IndependentBernoulliPpm,
            entries: Vec::new(),
        };
        assert!(append_common(&mut table, &definitions).is_ok());
        assert_eq!(table.entries.len(), 4);
        for (entry, (id, min, max, chance)) in table.entries.iter().zip([
            (3031, 1, 50, 1_000_000),
            (3723, 1, 3, 750_000),
            (2853, 1, 1, 500_000),
            (3338, 1, 1, 500_000),
        ]) {
            assert_eq!(
                entry.item.production_key,
                format!("oteryn:item.tibia.i{id}")
            );
            assert_eq!(
                (entry.min_count, entry.max_count, entry.probability_ppm),
                (min, max, Some(chance))
            );
        }
        assert_eq!(table.entries[2].details.container_capacity, Some(8));
        let ordinary = table.entries.clone();
        assert!(append_common(&mut table, &definitions).is_ok());
        assert_eq!(&table.entries[..4], ordinary.as_slice());
    }
}
