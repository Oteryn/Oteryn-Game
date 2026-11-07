//! Current Wiki c5fbb7dd...: supported Pumpkin recipients get five pumpkins and ten worms.
//! This closed event exception leaves generic boss draws and recipient eligibility unchanged.
use super::*;
const BOSS: &str = "oteryn:creature.the_mutated_pumpkin";
const TABLE: &str = "oteryn:loot.creature.the_mutated_pumpkin";
pub fn pumpkin_common_guarantees(
    boss: &str,
    reference: &LootDefinitionRef,
    table: &LootTableDefinition,
    mut plan: LootPlan,
) -> Result<LootPlan, LootPlanError> {
    if boss != BOSS {
        return Ok(plan);
    }
    if reference.family != "Loot"
        || reference.production_key != TABLE
        || plan.loot_table != *reference
        || table.entries.iter().any(|e| !e.details.children.is_empty())
        || plan.entries.iter().any(|e| e.parent.is_some())
    {
        return Err(LootPlanError::InvalidQuantityRange);
    }
    for (id, count) in [(3594_u32, 5_u32), (8177, 10)] {
        let key = format!("oteryn:item.tibia.i{id}");
        let sources: Vec<_> = table
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.item.family == "Item" && e.item.production_key == key)
            .collect();
        let [(ordinal, entry)] = sources.as_slice() else {
            return Err(LootPlanError::InvalidQuantityRange);
        };
        if entry.min_count != count
            || entry.max_count != count
            || entry.probability_ppm != Some(1_000_000)
            || !entry.details.stackable
            || entry.details.container_capacity.is_some()
            || entry.details.charges.is_some()
            || entry.details.subtype.is_some()
            || entry.details.unique
        {
            return Err(LootPlanError::InvalidQuantityRange);
        }
        let existing = plan
            .entries
            .iter()
            .filter(|e| e.item.production_key == key)
            .count();
        if existing > 1 {
            return Err(LootPlanError::InvalidQuantityRange);
        }
        if let Some(selected) = plan
            .entries
            .iter_mut()
            .find(|e| e.item.production_key == key)
        {
            if selected.item != entry.item
                || selected.draw_ordinal
                    != u32::try_from(*ordinal).map_err(|_| LootPlanError::TooManyTableEntries)?
            {
                return Err(LootPlanError::InvalidQuantityRange);
            }
            selected.quantity = count;
            selected.stackable = true;
        } else {
            plan.entries.push(LootPlanEntry {
                item: entry.item.clone(),
                quantity: count,
                draw_ordinal: u32::try_from(*ordinal)
                    .map_err(|_| LootPlanError::TooManyTableEntries)?,
                purpose_key: key,
                parent: None,
                placement_ordinal: 0,
                subtype: None,
                charges: None,
                unique: false,
                stackable: true,
            });
        }
    }
    if plan.entries.len() > COMBAT01_LOOT_PLAN_ITEMS_MAX {
        return Err(LootPlanError::TooManyPlanItems);
    }
    let mut bytes = charge_plan_bytes(0, header_footprint_bytes(reference))?;
    for (index, entry) in plan.entries.iter_mut().enumerate() {
        entry.placement_ordinal =
            u32::try_from(index + 1).map_err(|_| LootPlanError::TooManyPlanItems)?;
        bytes = charge_plan_bytes(
            bytes,
            entry_footprint_bytes(&entry.item, &entry.purpose_key),
        )?;
    }
    Ok(plan)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn source() -> (LootDefinitionRef, LootTableDefinition) {
        let reference = LootDefinitionRef::new("Loot", TABLE, "definition-r1");
        let table = LootTableDefinition {
            algorithm: LootSelectionAlgorithm::IndependentBernoulliPpm,
            entries: [(3594, 5), (8177, 10)]
                .into_iter()
                .map(|(id, count)| LootTableEntry {
                    item: LootDefinitionRef::new(
                        "Item",
                        format!("oteryn:item.tibia.i{id}"),
                        "definition-r1",
                    ),
                    min_count: count,
                    max_count: count,
                    probability_ppm: Some(1_000_000),
                    details: oteryn_gameplay_domain::domain::loot_data::LootEntryDetails {
                        stackable: true,
                        ..Default::default()
                    },
                })
                .collect(),
        };
        (reference, table)
    }
    #[test]
    fn pumpkin_current_common_rewards_survive_zero_factor_in_real_boss_plan()
    -> Result<(), LootPlanError> {
        let (reference, table) = source();
        let rolled =
            plan_boss_loot_with_bonus([1; 16], [2; 16], &reference, &table, 0.0, false, 0, |_| {
                false
            })?;
        assert!(
            rolled.entries.is_empty(),
            "regression reaches the generic contribution gate"
        );
        let fixed = pumpkin_common_guarantees(BOSS, &reference, &table, rolled)?;
        assert_eq!(
            fixed
                .entries
                .iter()
                .map(|e| (e.item.production_key.as_str(), e.quantity, e.draw_ordinal))
                .collect::<Vec<_>>(),
            [
                ("oteryn:item.tibia.i3594", 5, 0),
                ("oteryn:item.tibia.i8177", 10, 1)
            ]
        );
        Ok(())
    }
    #[test]
    fn pumpkin_guarantee_is_idempotent_and_does_not_change_other_bosses()
    -> Result<(), LootPlanError> {
        let (reference, table) = source();
        let rolled =
            plan_boss_loot_with_bonus([1; 16], [2; 16], &reference, &table, 0.0, false, 0, |_| {
                false
            })?;
        assert_eq!(
            pumpkin_common_guarantees("oteryn:creature.other", &reference, &table, rolled.clone()),
            Ok(rolled.clone())
        );
        let once = pumpkin_common_guarantees(BOSS, &reference, &table, rolled)?;
        assert_eq!(
            pumpkin_common_guarantees(BOSS, &reference, &table, once.clone()),
            Ok(once)
        );
        Ok(())
    }
}
