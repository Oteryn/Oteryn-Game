//! Each nupi hands over one fish. The hundredth earns Jamjam.
use super::{InventoryRequirement, ItemAmount, RewardCallback};

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
        let mut row = template.clone();
        row.key = format!("oteryn:quest-reward/sinatuki/final-{species}");
        row.take = vec![cost(item, 1)];
        row.inventory = vec![carried(item, 1, 1_000_000)];
        if item == PERCH {
            row.inventory.push(carried(TROUT, 0, 0));
        }
        for fed in 0..i64::from(FIRST) {
            let mut feeding = row.clone();
            feeding.key = format!("oteryn:quest-reward/sinatuki/{species}-fish-{}", fed + 1);
            let mut total = feeding.storage[0].clone();
            total.track = CONTRIBUTION.into();
            total.minimum = fed;
            total.maximum = fed;
            total.set = fed + 1;
            feeding.storage[0].maximum = 0;
            feeding.storage[0].set = i64::from(total.set == i64::from(FIRST));
            feeding.storage.push(total);
            feeding.give = None;
            feeding.achievement = None;
            feeding.completes = false;
            feeding.reply = vec![
                "Jinuma, suvituka siq chuqua!! Nguraka, nguraka! <happily takes the food from you>"
                    .into(),
            ];
            result.push(feeding);
        }
        row.storage[0].minimum = 1;
        row.storage[0].maximum = 1;
        row.reply = vec!["Jinuma, suvituka siq chuqua!! Nguraka, nguraka! <happily takes the food from you and gives you five glimmering crystals>".into()];
        result.push(row);
    }
    Ok(result)
}
