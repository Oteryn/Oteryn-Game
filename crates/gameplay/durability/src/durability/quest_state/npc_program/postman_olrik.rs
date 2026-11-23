//! Resolve the paid Olrik roll inside its receipt transaction, after replay lookup.
use super::{DurabilityError, NpcRulePlan};
use oteryn_gameplay_quests::quest::npc_rules::NpcRuleEffect;

pub(super) const ROLL: &str = "oteryn:quest-npc/postman/olrik/roll";
const CREDIT: &str = "oteryn:quest-progress/quest/u7_24/the_postman_missions/measurements_olrik";

pub(super) fn resolve(plan: &mut NpcRulePlan, face: i16) -> Result<(), DurabilityError> {
    let [win, loss] = plan.reply.as_slice() else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let template = match face {
        6 => win,
        1..=5 => loss,
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    plan.reply = vec![template.replace("{roll}", &face.to_string())];

    if face != 6 {
        // The wager is charged on either result. A losing roll records an
        // unchanged credit so the receipt can replay the paid result exactly.
        plan.track_changes.retain(|change| change.key == CREDIT);
        let [credit] = plan.track_changes.as_mut_slice() else {
            return Err(DurabilityError::InvalidStoredState);
        };
        credit.after = credit.before;
        plan.effects
            .retain(|effect| matches!(effect, NpcRuleEffect::TakeItem { .. }));
        plan.next_topic = 1;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_gameplay_quests::quest::npc_rules::{
        NpcInventorySnapshot, NpcItemKey, NpcRuleCatalog, NpcRuleInput, NpcRuleTrigger,
    };
    use std::collections::BTreeMap;

    #[test]
    fn paid_olrik_roll_only_awards_credit_on_six() -> Result<(), String> {
        let catalog = NpcRuleCatalog::runtime().map_err(|error| error.to_string())?;
        let mission = "oteryn:quest-progress/quest/u7_24/the_postman_missions/mission07";
        let tracks = BTreeMap::from([(mission.to_owned(), 5)]);
        let gold = NpcItemKey {
            item: "oteryn:item.tibia.i3031".into(),
            subtype: None,
        };
        let inventory = NpcInventorySnapshot {
            items: BTreeMap::from([(gold.clone(), 5)]),
            ..Default::default()
        };
        let source = catalog
            .plan_item_exchange(&NpcRuleInput {
                npc: "oteryn:npc.olrik",
                trigger: NpcRuleTrigger::Say,
                message: "yes",
                player_name: None,
                topic: 1,
                tracks: &tracks,
                inventory: &inventory,
                now_epoch_seconds: 0,
                calendar_month: 1,
                premium_current: false,
                level: 1,
                vocation: "",
                sex: 0,
                outfits: &BTreeMap::new(),
            })
            .map_err(|error| error.to_string())?
            .ok_or("paid Olrik roll")?;
        assert_eq!(source.selected_rule_keys, [ROLL]);
        for face in 1..=6 {
            let mut plan = source.clone();
            resolve(&mut plan, face).map_err(|error| format!("{error:?}"))?;
            assert_eq!(plan.inventory_after.items.get(&gold), Some(&0));
            assert_eq!(plan.next_topic, i32::from(face != 6));
            assert_eq!(
                plan.track_changes
                    .iter()
                    .find(|change| change.key == mission)
                    .map(|change| change.after),
                (face == 6).then_some(6),
            );
            assert_eq!(
                plan.track_changes
                    .iter()
                    .find(|change| change.key == CREDIT)
                    .map(|change| change.after),
                Some(i64::from(face == 6)),
            );
            let template = &source.reply[usize::from(face != 6)];
            assert_eq!(plan.reply, [template.replace("{roll}", &face.to_string())]);
        }
        Ok(())
    }
}
