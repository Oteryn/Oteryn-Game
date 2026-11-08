//! Read-only equipment admission composed with the existing source storage receipt.
use super::*;

impl NpcRuleCatalog {
    /// Closed equipment-and-storage branches; item mutation remains on its owner.
    pub fn equipped_storage_requirements(&self, npc: &str) -> Option<BTreeMap<String, String>> {
        let mut required = BTreeMap::new();
        for rule in self
            .programs
            .iter()
            .flat_map(|p| &p.document.rules)
            .filter(|r| r.npc == npc)
        {
            for guard in &rule.guards {
                match guard {
                    NpcRuleGuard::Storage { .. }
                    | NpcRuleGuard::StorageTime { .. }
                    | NpcRuleGuard::CalendarMonth { .. }
                    | NpcRuleGuard::Level { .. }
                    | NpcRuleGuard::Vocation { .. }
                    | NpcRuleGuard::Premium { .. } => {}
                    NpcRuleGuard::Equipped { slot, item }
                        if matches!(slot.as_str(), "head" | "necklace") =>
                    {
                        if required
                            .insert(slot.clone(), item.clone())
                            .is_some_and(|old| old != *item)
                        {
                            return None;
                        }
                    }
                    _ => return None,
                }
            }
            if rule.effects.iter().any(|effect| {
                !matches!(
                    effect,
                    NpcRuleEffect::Set { .. }
                        | NpcRuleEffect::Add { .. }
                        | NpcRuleEffect::SetNowPlus { .. }
                        | NpcRuleEffect::SetTopic { .. }
                        | NpcRuleEffect::EndConversation { .. }
                        | NpcRuleEffect::Experience { .. }
                )
            }) {
                return None;
            }
        }
        (!required.is_empty()).then_some(required)
    }

    pub fn optimistic_equipped_inventory(&self, npc: &str) -> Option<NpcInventorySnapshot> {
        Some(NpcInventorySnapshot {
            equipped: self.equipped_storage_requirements(npc)?,
            ..NpcInventorySnapshot::default()
        })
    }

    /// Evaluate only with an equipment-owner snapshot; this method never applies effects.
    pub fn plan_equipped_storage(
        &self,
        input: &NpcRuleInput<'_>,
    ) -> Result<Option<NpcRulePlan>, NpcRuleError> {
        self.equipped_storage_requirements(input.npc)
            .ok_or_else(|| NpcRuleError::MissingNativeResolution("equipped storage".into()))?;
        evaluate::plan(self, input, &BTreeMap::new(), false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equipment_admission_requires_the_live_head_slot() -> Result<(), NpcRuleError> {
        let source = format!("{SOURCE_PIN}data/npclib/npc.lua:13");
        let data = serde_json::json!({
            "schema":1,"quest":"oteryn:quest.equipment_fixture",
            "tracks":[{"key":"oteryn:quest-progress/equipment_fixture/stage",
                "quest":"oteryn:quest.equipment_fixture","initial":0,"min":0,"max":1,"sources":[source]}],
            "rules":[{"key":"oteryn:quest-npc/equipment_fixture/accept",
                "quest":"oteryn:quest.equipment_fixture","npc":"oteryn:npc.fixture","trigger":"say",
                "keywords":["mission"],"guards":[{"kind":"equipped","slot":"head","item":"oteryn:item.tibia.i5461"}],
                "effects":[{"kind":"set","track":"oteryn:quest-progress/equipment_fixture/stage","value":1}],
                "reply":["Accepted."],"sources":[source]}]
        }).to_string();
        let catalog = NpcRuleCatalog::from_json_documents(&[&data])?;
        let tracks = BTreeMap::new();
        let outfits = BTreeMap::new();
        for (slot, item, expected) in [
            (None, "oteryn:item.tibia.i5461", false),
            (Some("armor"), "oteryn:item.tibia.i5461", false),
            (Some("head"), "oteryn:item.tibia.i140", false),
            (Some("head"), "oteryn:item.tibia.i5461", true),
        ] {
            let mut inventory = NpcInventorySnapshot::default();
            inventory.items.insert(
                NpcItemKey {
                    item: "oteryn:item.tibia.i5461".into(),
                    subtype: None,
                },
                1,
            );
            if let Some(slot) = slot {
                inventory.equipped.insert(slot.into(), item.into());
            }
            let input = NpcRuleInput {
                npc: "oteryn:npc.fixture",
                trigger: NpcRuleTrigger::Say,
                message: "mission",
                player_name: None,
                topic: 0,
                tracks: &tracks,
                inventory: &inventory,
                now_epoch_seconds: 0,
                calendar_month: 1,
                premium_current: false,
                level: 0,
                vocation: "",
                sex: 0,
                outfits: &outfits,
            };
            assert_eq!(catalog.plan_equipped_storage(&input)?.is_some(), expected);
            assert!(catalog.plan_storage(&input).is_err());
        }
        Ok(())
    }
}
