//! The first feeding costs 99 fish of either species. One further fish earns Jamjam.
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
        for count in 1..=FIRST {
            let remaining = i64::from(FIRST - count);
            // Stop at 99 even if the last stack contains the final reward fish.
            let mut full = template.clone();
            full.key = format!("oteryn:quest-reward/sinatuki/{species}-finish-feeding-{count}");
            full.inventory = vec![carried(item, count, 1_000_000)];
            if item == PERCH {
                full.inventory.push(carried(TROUT, 0, 0));
            }
            full.take = vec![cost(item, count)];
            full.storage[0].maximum = 0;
            full.storage[0].set = 1;
            full.state_ops = vec![StateOperation {
                track: CONTRIBUTION.into(),
                from: StateCondition::Between {
                    minimum: remaining,
                    maximum: remaining,
                },
                effect: StateMutation::Add {
                    value: count.into(),
                },
            }];
            full.give = None;
            full.achievement = None;
            full.completes = false;
            full.reply = vec![
                "Jinuma, suvituka siq chuqua!! Nguraka, nguraka! <happily takes the food from you>"
                    .into(),
            ];
            if remaining > 0 {
                let mut partial = full.clone();
                partial.key = format!("oteryn:quest-reward/sinatuki/{species}-feed-{count}");
                partial.inventory[0].maximum = count.into();
                partial.storage[0].maximum = -1;
                partial.storage[0].set = 0;
                partial.state_ops[0].from = StateCondition::Between {
                    minimum: 0,
                    maximum: remaining - 1,
                };
                // Both intake phases advance through a closed transition. The contribution
                // track carries the total across visits, species changes and reconnects.
                let mut next = partial.clone();
                next.key.push_str("-next");
                next.storage[0].minimum = 0;
                next.storage[0].maximum = 0;
                next.storage[0].set = -1;
                result.extend([partial, next]);
            }
            result.push(full);
        }
        let mut row = template.clone();
        row.key = format!("oteryn:quest-reward/sinatuki/final-{species}");
        row.storage[0].minimum = 1;
        row.storage[0].maximum = 1;
        row.take = vec![cost(item, 1)];
        row.inventory = vec![carried(item, 1, 1_000_000)];
        if item == PERCH {
            row.inventory.push(carried(TROUT, 0, 0));
        }
        row.reply = vec!["Jinuma, suvituka siq chuqua!! Nguraka, nguraka! <happily takes the food from you and gives you five glimmering crystals>".into()];
        result.push(row);
    }
    Ok(result)
}
