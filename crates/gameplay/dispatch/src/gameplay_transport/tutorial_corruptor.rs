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
        if table.entries.len() > 13 {
            return Err(());
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
            let native = self
                .active_generation
                .and_then(|g| g.native_gameplay())
                .ok_or(())?;
            let mut definitions = BTreeMap::new();
            for vocation in ["knight", "paladin", "sorcerer", "druid", "monk"] {
                for id in corruptor_items(vocation) {
                    let key = format!("oteryn:item.tibia.i{id}");
                    if let Some(revision) = index.revision(&key) {
                        let Some(policy) = native.item_policy(&key, revision) else {
                            continue;
                        };
                        let admission = &policy.record().admission;
                        if !admission.materializable
                            || !admission.legal_destinations.contains(
                                &crate::content::native_gameplay::NativeItemDestination::CharacterInventory,
                            )
                        {
                            continue;
                        }
                        definitions.insert(*id, LootDefinitionRef::new("Item", key, revision));
                    }
                }
            }
            definitions
        };
        let desired = corruptor_items(build.vocation());
        // A missing native admission must not consume the armed one-time reward.
        // The durable claim accepts an empty selection, so refuse before claiming.
        if !desired.iter().all(|id| definitions.contains_key(id)) {
            return Err(());
        }
        let defined = desired.to_vec();
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
