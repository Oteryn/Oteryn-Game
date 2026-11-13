//! Closed, reusable storage-and-plain-item source plans. Durable owners supply custody.
use super::*;
use std::collections::BTreeSet;

impl NpcRuleCatalog {
    /// A source NPC may require eight plain item types and grant twenty types.
    /// Outfit effects use the shared appearance participant; money and optional removals stay closed.
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
                    | NpcRuleGuard::StorageTime { .. }
                    | NpcRuleGuard::CalendarMonth { .. }
                    | NpcRuleGuard::CalendarWindow { .. }
                    | NpcRuleGuard::Level { .. }
                    | NpcRuleGuard::Vocation { .. }
                    | NpcRuleGuard::Premium { .. }
                    | NpcRuleGuard::Outfit { .. }
                    | NpcRuleGuard::Sex { .. } => {}
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
                    | NpcRuleEffect::SetNowPlus { .. }
                    | NpcRuleEffect::Add { .. }
                    | NpcRuleEffect::SetTopic { .. }
                    | NpcRuleEffect::EndConversation { .. }
                    | NpcRuleEffect::Experience { .. }
                    | NpcRuleEffect::Achievement { .. } => {}
                    NpcRuleEffect::Outfit { .. } | NpcRuleEffect::OutfitAddon { .. } => {
                        items = true;
                    }
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
            if grants > 20 || keys.len() > 8 {
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
        // Keep storage-only branches on their existing owner. Only inventory-owned
        // guards and effects select the plain item planner with the same atomic cap.
        match self.plan_storage(input) {
            Err(NpcRuleError::MissingNativeResolution(_)) => {
                evaluate::plan(self, input, &BTreeMap::new(), false)
            }
            result => result,
        }
    }
}

impl NpcRuleCatalog {
    /// Bounded custody discovery for outfit sources. Unsupported effects in a
    /// different branch do not admit them: the selected plan is checked below.
    pub fn outfit_exchange_keys(&self, npc: &str) -> Option<Vec<String>> {
        let rules = self
            .programs
            .iter()
            .flat_map(|p| &p.document.rules)
            .filter(|r| r.npc == npc)
            .collect::<Vec<_>>();
        if !rules.iter().any(|r| {
            r.effects.iter().any(|e| {
                matches!(
                    e,
                    NpcRuleEffect::Outfit { .. } | NpcRuleEffect::OutfitAddon { .. }
                )
            })
        }) {
            return None;
        }
        let mut keys = BTreeSet::new();
        for rule in rules {
            for guard in &rule.guards {
                match guard {
                    NpcRuleGuard::Storage { .. }
                    | NpcRuleGuard::StorageTime { .. }
                    | NpcRuleGuard::CalendarMonth { .. }
                    | NpcRuleGuard::CalendarWindow { .. }
                    | NpcRuleGuard::Level { .. }
                    | NpcRuleGuard::Vocation { .. }
                    | NpcRuleGuard::Premium { .. }
                    | NpcRuleGuard::Outfit { .. }
                    | NpcRuleGuard::Sex { .. } => {}
                    NpcRuleGuard::Item {
                        item,
                        subtype: None,
                        ..
                    } => {
                        keys.insert(item.clone());
                    }
                    _ => return None,
                }
            }
            for effect in &rule.effects {
                if let NpcRuleEffect::TakeItem { item, .. } = effect {
                    keys.insert(item.clone());
                }
            }
        }
        (keys.len() <= 8).then(|| keys.into_iter().collect())
    }

    pub fn plan_outfit_exchange(
        &self,
        input: &NpcRuleInput<'_>,
    ) -> Result<Option<NpcRulePlan>, NpcRuleError> {
        self.outfit_exchange_keys(input.npc)
            .ok_or_else(|| NpcRuleError::MissingNativeResolution("outfit source custody".into()))?;
        let plan = evaluate::plan(self, input, &BTreeMap::new(), false)?;
        if let Some(plan) = &plan {
            let gifts = plan
                .effects
                .iter()
                .filter(|effect| matches!(effect, NpcRuleEffect::GiveItem { .. }))
                .count();
            if gifts > 1
                || plan
                    .effects
                    .iter()
                    .any(|effect| !outfit_effect_owned(effect))
            {
                return Err(NpcRuleError::MissingNativeResolution(
                    "unowned outfit source effect".into(),
                ));
            }
        }
        Ok(plan)
    }
}

fn outfit_effect_owned(effect: &NpcRuleEffect) -> bool {
    match effect {
        NpcRuleEffect::Set { .. }
        | NpcRuleEffect::Add { .. }
        | NpcRuleEffect::SetNowPlus { .. }
        | NpcRuleEffect::SetTopic { .. }
        | NpcRuleEffect::EndConversation { .. }
        | NpcRuleEffect::Experience { .. }
        | NpcRuleEffect::Outfit { .. }
        | NpcRuleEffect::OutfitAddon { .. }
        | NpcRuleEffect::TakeItem {
            count: 1..=100,
            subtype: None,
            optional: false,
            ..
        }
        | NpcRuleEffect::GiveItem {
            count: 1..=100,
            subtype: None,
            ..
        } => true,
        NpcRuleEffect::MagicEffect { effect } => effect == "CONST_ME_MAGIC_BLUE",
        _ => false,
    }
}
