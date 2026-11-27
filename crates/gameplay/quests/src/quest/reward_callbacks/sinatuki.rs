//! The first feeding costs 99 fish of either species. One further fish earns Jamjam.
use super::{InventoryRequirement, ItemAmount, RewardCallback};

const TROUT: &str = "oteryn:item.tibia.i7158";
const PERCH: &str = "oteryn:item.tibia.i7159";
const FIRST: u16 = 99;
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
    // Disjoint trout bounds select one exact payment even when both stacks exceed the cost.
    for trout in 0..=FIRST {
        let perch = FIRST - trout;
        let mut row = template.clone();
        row.key = format!("oteryn:quest-reward/sinatuki/feed-{trout}-{perch}");
        row.take = [cost(TROUT, trout), cost(PERCH, perch)]
            .into_iter()
            .filter(|item| item.count > 0)
            .collect();
        row.inventory = vec![
            carried(
                TROUT,
                trout,
                if trout == FIRST {
                    1_000_000
                } else {
                    trout.into()
                },
            ),
            carried(PERCH, perch, 1_000_000),
        ];
        row.storage[0].set = 1;
        row.give = None;
        row.achievement = None;
        row.completes = false;
        row.reply = vec![
            "Jinuma, suvituka siq chuqua!! Nguraka, nguraka! <happily takes the food from you>"
                .into(),
        ];
        result.push(row);
    }
    for (species, item) in [("trout", TROUT), ("perch", PERCH)] {
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
