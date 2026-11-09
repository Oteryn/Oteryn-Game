//! Closed, reusable storage-and-plain-item source plans. Durable owners supply custody.
use super::*;
use std::collections::BTreeSet;

impl NpcRuleCatalog {
    /// One source NPC may require up to eight plain item types and grant one type
    /// per branch. Money, outfits, optional removals and native effects stay closed.
    pub fn item_exchange_keys(&self, npc: &str) -> Option<Vec<String>> {
        let mut keys = BTreeSet::new();
        let mut items = false;
        for rule in self
            .programs
            .iter()
            .flat_map(|program| &program.document.rules)
            .filter(|rule| rule.npc == npc)
        {
            for guard in &rule.guards {
                match guard {
                    NpcRuleGuard::Storage { .. }
                    | NpcRuleGuard::CalendarMonth { .. }
                    | NpcRuleGuard::Level { .. } => {}
                    NpcRuleGuard::Item {
                        item,
                        count: 1..=100,
                        subtype: None,
                        op: NpcComparison::Ge | NpcComparison::Lt,
                    } => {
                        keys.insert(item.clone());
                    }
                    _ => return None,
                }
            }
            let mut grants = 0;
            for effect in &rule.effects {
                match effect {
                    NpcRuleEffect::Set { .. }
                    | NpcRuleEffect::Add { .. }
                    | NpcRuleEffect::SetTopic { .. }
                    | NpcRuleEffect::EndConversation { .. }
                    | NpcRuleEffect::Experience { .. } => {}
                    NpcRuleEffect::TakeItem {
                        item,
                        count: 1..=100,
                        subtype: None,
                        optional: false,
                    } => {
                        keys.insert(item.clone());
                        items = true;
                    }
                    NpcRuleEffect::GiveItem {
                        count: 1..=100,
                        subtype: None,
                        ..
                    } => {
                        grants += 1;
                        items = true;
                    }
                    _ => return None,
                }
            }
            if grants > 1 || keys.len() > 8 {
                return None;
            }
        }
        items.then(|| keys.into_iter().collect())
    }

    pub fn optimistic_item_inventory(&self, npc: &str) -> Option<NpcInventorySnapshot> {
        Some(NpcInventorySnapshot {
            items: self
                .item_exchange_keys(npc)?
                .into_iter()
                .map(|item| {
                    (
                        NpcItemKey {
                            item,
                            subtype: None,
                        },
                        100,
                    )
                })
                .collect(),
            ..NpcInventorySnapshot::default()
        })
    }

    /// This is a preview or an evaluation over the locked inventory snapshot;
    /// only the fenced durable item participant may apply the resulting effects.
    pub fn plan_item_exchange(
        &self,
        input: &NpcRuleInput<'_>,
    ) -> Result<Option<NpcRulePlan>, NpcRuleError> {
        self.item_exchange_keys(input.npc)
            .ok_or_else(|| NpcRuleError::MissingNativeResolution("plain item exchange".into()))?;
        // Keep storage-only branches on their existing owner, including the
        // exact eleven-track Father acceptance. Only inventory-owned guards
        // and effects select the plain item planner with its ordinary cap.
        match self.plan_storage(input) {
            Err(NpcRuleError::MissingNativeResolution(_)) => {
                evaluate::plan(self, input, &BTreeMap::new(), false)
            }
            result => result,
        }
    }
}
