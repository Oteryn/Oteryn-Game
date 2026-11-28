//! Feed whole stacks until 99 fish have been credited, then offer one final fish.
use super::{
    InventoryRequirement, ItemAmount, RewardCallback, StateCondition, StateMutation, StateOperation,
};

const TROUT: &str = "oteryn:item.tibia.i7158";
const PERCH: &str = "oteryn:item.tibia.i7159";
const FIRST: u16 = 99;
pub(super) const CONTRIBUTION: &str = "oteryn:quest-progress/qc/sinatuki/contribution";
const NPC: &str = "oteryn:npc.sinatuki";
const HISTORICAL: &str = "oteryn:quest-reward/qc-sinatuki-";

pub(super) fn historical(row: &RewardCallback) -> bool {
    row.npc == NPC && row.key.starts_with(HISTORICAL)
}

pub(super) fn advances_contribution(row: &RewardCallback) -> bool {
    let ([cost], [rule], [op]) = (&row.take[..], &row.inventory[..], &row.state_ops[..]) else {
        return false;
    };
    let value = i64::from(cost.count);
    row.consumes_fish_stack()
        && row.npc == NPC
        && super::is_sinatuki_fish(&cost.item)
        && (1..FIRST).contains(&cost.count)
        && rule.item == cost.item
        && rule.minimum == u32::from(cost.count)
        && rule.maximum == rule.minimum
        && op.track == CONTRIBUTION
        && op.effect == (StateMutation::Add { value })
        && op.from
            == (StateCondition::Between {
                minimum: 0,
                maximum: i64::from(FIRST) - value - 1,
            })
}

fn cost(item: &str, count: u16) -> ItemAmount {
    ItemAmount {
        item: item.into(),
        count,
        fluid_subtype: None,
    }
}

fn carried(item: &str, minimum: u16, maximum: u32) -> InventoryRequirement {
    InventoryRequirement {
        item: item.into(),
        minimum: minimum.into(),
        maximum,
    }
}

pub(super) fn feeding_rows(rows: &[RewardCallback]) -> Result<Vec<RewardCallback>, String> {
    let template = rows
        .iter()
        .find(|row| row.key == "oteryn:quest-reward/qc-sinatuki-trout-99")
        .ok_or("missing Sinatuki reward definition")?;
    let mut result = Vec::new();
    for (species, item) in [("trout", TROUT), ("perch", PERCH)] {
        let mut row = template.clone();
        row.key = format!("oteryn:quest-reward/sinatuki/final-{species}");
        row.take = vec![cost(item, 1)];
        row.inventory = vec![carried(item, 1, 1_000_000)];
        if item == PERCH {
            row.inventory.push(carried(TROUT, 0, 0));
        }
        for quantity in 1..=100_u16 {
            let threshold = i64::from(FIRST.saturating_sub(quantity));
            for finish in [false, true] {
                if !finish && threshold == 0 {
                    continue;
                }
                let mut feeding = row.clone();
                let phase = if finish { "ready" } else { "feed" };
                feeding.key =
                    format!("oteryn:quest-reward/sinatuki/stack-{species}-{quantity}-{phase}");
                feeding.take[0].count = quantity;
                feeding.inventory = vec![carried(item, quantity, quantity.into())];
                feeding.storage[0].maximum = 0;
                feeding.storage[0].set = i64::from(finish);
                if finish {
                    let mut total = feeding.storage[0].clone();
                    total.track = CONTRIBUTION.into();
                    total.minimum = threshold;
                    total.maximum = i64::from(FIRST) - 1;
                    total.set = i64::from(FIRST);
                    feeding.storage.push(total);
                } else {
                    feeding.state_ops.push(StateOperation {
                        track: CONTRIBUTION.into(),
                        from: StateCondition::Between {
                            minimum: 0,
                            maximum: threshold - 1,
                        },
                        effect: StateMutation::Add {
                            value: quantity.into(),
                        },
                    });
                }
                feeding.give = None;
                feeding.achievement = None;
                feeding.completes = false;
                feeding.reply = vec!["Jinuma, suvituka siq chuqua!! Nguraka, nguraka! <happily takes the food from you>".into()];
                result.push(feeding);
            }
        }
        row.storage[0].minimum = 1;
        row.storage[0].maximum = 1;
        row.reply = vec!["Jinuma, suvituka siq chuqua!! Nguraka, nguraka! <happily takes the food from you and gives you five glimmering crystals>".into()];
        result.push(row);
    }
    Ok(result)
}
