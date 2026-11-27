//! Closed, reusable storage-and-plain-item source plans. Durable owners supply custody.
use super::*;
use std::collections::BTreeSet;

const BIGFOOT: &str = "oteryn:quest.bigfoot_s_burden_quest";
const GNOMERAL: &str = "oteryn:npc.gnomeral";
const MATCHMAKER_ACCEPT: &str = "oteryn:quest-npc/bigfoot_s_burden_quest/gnomeral/branch-77-4-1-1";

fn bigfoot_daily_contact(npc: &str) -> bool {
    matches!(npc, GNOMERAL | "oteryn:npc.commander_stone")
}

/// Crystal gnomeral.lua:85 chooses one of the seven crystal appearances.
pub(in crate::quest) fn matchmaker_target(draw: u8) -> Option<u16> {
    (draw < 7).then(|| 15809 + u16::from(draw))
}

impl NpcRuleCatalog {
    /// A source NPC may require eight plain item types and grant twenty types.
    /// Outfit effects and the fixed Olrik wager use their dedicated durable participants.
    /// Bigfoot daily tools and promotion effects use the same fenced participants.
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
                        // An item can qualify a topic without being consumed.
                        items |= npc == "oteryn:npc.noodles";
                    }
                    NpcRuleGuard::Money {
                        amount: 5,
                        include_bank: true,
                    } if rule.key == "oteryn:quest-npc/postman/olrik/roll" => {}
                    NpcRuleGuard::ItemTotal {
                        items: ingredients, ..
                    } => {
                        keys.extend(ingredients.iter().cloned());
                        items = true;
                    }
                    _ => return None,
                }
            }
            let bigfoot_daily = rule.quest == BIGFOOT && bigfoot_daily_contact(npc);
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
                        count: count @ 1..=100,
                        subtype: None,
                        optional,
                    } if !optional || (bigfoot_daily && *count == 1) => {
                        keys.insert(item.clone());
                        items = true;
                    }
                    NpcRuleEffect::TakeMoney {
                        amount: 5,
                        include_bank: true,
                    } if rule.key == "oteryn:quest-npc/postman/olrik/roll" => {
                        items = true;
                    }
                    NpcRuleEffect::TakeAllItems { items: ingredients } => {
                        keys.extend(ingredients.iter().cloned());
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
                    NpcRuleEffect::MagicEffect { effect }
                        if bigfoot_daily && effect == "CONST_ME_MAGIC_BLUE" => {}
                    NpcRuleEffect::BigfootMatchmakerTarget
                        if bigfoot_daily && npc == GNOMERAL && rule.key == MATCHMAKER_ACCEPT => {}
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
            money: if npc == "oteryn:npc.olrik" { 5 } else { 0 },
            ..NpcInventorySnapshot::default()
        })
    }

    /// This is a preview or an evaluation over the locked inventory snapshot;
    /// only the fenced durable item participant may apply the resulting effects.
    pub fn plan_item_exchange(
        &self,
        input: &NpcRuleInput<'_>,
    ) -> Result<Option<NpcRulePlan>, NpcRuleError> {
        self.plan_item_exchange_with_draw(input, || None)
    }

    /// The caller supplies a command-bound draw for Gnomeral's native target selection.
    pub fn plan_item_exchange_with_draw(
        &self,
        input: &NpcRuleInput<'_>,
        matchmaker_draw: impl FnOnce() -> Option<u8>,
    ) -> Result<Option<NpcRulePlan>, NpcRuleError> {
        self.item_exchange_keys(input.npc)
            .ok_or_else(|| NpcRuleError::MissingNativeResolution("plain item exchange".into()))?;
        // Keep storage-only branches on their existing owner. Only inventory-owned
        // guards and effects select the plain item planner with the same atomic cap.
        match self.plan_storage(input) {
            Err(NpcRuleError::MissingNativeResolution(_)) => {
                let mut resolutions = BTreeMap::new();
                if input.npc == GNOMERAL
                    && let Some(target) = matchmaker_draw().and_then(matchmaker_target)
                {
                    resolutions.insert(
                        MATCHMAKER_ACCEPT.into(),
                        vec![NpcRuleEffect::Set {
                            track: "oteryn:quest-progress/quest/u9_60/bigfoots_burden/matchmaker_id_needed".into(),
                            value: i64::from(target),
                        }],
                    );
                }
                let mut plan = evaluate::plan(self, input, &resolutions, false)?;
                if let Some(plan) = &mut plan
                    && plan.quest == BIGFOOT
                    && bigfoot_daily_contact(input.npc)
                {
                    // The evaluator omits absent tools. Each remaining removal must
                    // consume the same quantity from the locked snapshot at commit.
                    for effect in &mut plan.effects {
                        if let NpcRuleEffect::TakeItem { optional, .. } = effect {
                            *optional = false;
                        }
                    }
                }
                Ok(plan)
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
