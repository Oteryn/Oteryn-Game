//! W22a: the real auto-attack kill drains large and nested loot plans through SQL custody guards.
use super::w22a_loot_fixtures::Case;

use super::TestResult;

use super::kill_reward_e2e_fixtures::{scenario, scenario_thread};

use super::w22a_loot_fixtures::CASE;

#[test]
fn w22a_configured_loot_rate_scales_the_table_used_by_death_settlement() -> TestResult {
    // Crystal 201189f8 `data/libs/functions/functions.lua:68-70`: RATE_LOOT multiplies the
    // effective chance. This is the same helper called by kill_reward.rs before PG settlement.
    let mut table = crate::combat::LootTableDefinition {
        algorithm: crate::combat::LootSelectionAlgorithm::IndependentBernoulliPpm,
        entries: vec![crate::combat::LootTableEntry {
            item: crate::combat::LootDefinitionRef::new(
                "Item",
                "oteryn:item.tibia.i3031",
                "definition-r1",
            ),
            min_count: 1,
            max_count: 1,
            probability_ppm: Some(400_000),
            details: Default::default(),
        }],
    };
    oteryn_gameplay_items::loot_plan::apply_configured_loot_rate(&mut table, 200);
    assert_eq!(table.entries[0].probability_ppm, Some(800_000));
    Ok(())
}

#[test]
fn w22a_more_than_sixteen_entries_drop_into_the_kill_corpse() -> TestResult {
    scenario_thread::run("w22a-large", || {
        CASE.set(Some(Case::Large));
        scenario()
    })
}
#[test]
fn w22a_nested_container_drop_mints_its_children_inside_the_container() -> TestResult {
    scenario_thread::run("w22a-nested", || {
        CASE.set(Some(Case::Nested));
        scenario()
    })
}
#[test]
fn w22a_charged_loot_with_a_subtype_mints_with_its_full_charges() -> TestResult {
    scenario_thread::run("w22a-charged-subtype", || {
        CASE.set(Some(Case::ChargedSubtype));
        scenario()
    })
}
#[test]
fn w22a_fluid_loot_with_a_subtype_mints_the_filled_container() -> TestResult {
    scenario_thread::run("w22a-fluid-subtype", || {
        CASE.set(Some(Case::FluidSubtype));
        scenario()
    })
}
#[test]
fn w22b_gut_widens_creature_product_rolls_on_the_real_kill_path() -> TestResult {
    scenario_thread::run("w22b-gut", || {
        CASE.set(Some(Case::Gut));
        scenario()
    })
}
#[test]
fn w22c_wealth_duplex_rolls_the_kill_loot_once_more() -> TestResult {
    scenario_thread::run("w22c-duplex-on", || {
        CASE.set(Some(Case::DuplexOn));
        scenario()
    })
}
#[test]
fn w22c_a_kill_without_wealth_duplex_rolls_its_loot_once() -> TestResult {
    scenario_thread::run("w22c-duplex-off", || {
        CASE.set(Some(Case::DuplexOff));
        scenario()
    })
}
#[test]
fn w22b_wealth_duplex_ignores_wall_clock_time_spent_offline() -> TestResult {
    scenario_thread::run("w22b-duplex-offline", || {
        CASE.set(Some(Case::DuplexOfflineIgnored));
        scenario()
    })
}
#[test]
fn w22b_wealth_duplex_ends_after_one_online_hour() -> TestResult {
    scenario_thread::run("w22b-duplex-online-spent", || {
        CASE.set(Some(Case::DuplexOnlineSpent));
        scenario()
    })
}
#[test]
fn w22a_mint_into_a_container_outside_the_kill_corpse_is_refused() -> TestResult {
    scenario_thread::run("w22a-outside", || {
        CASE.set(Some(Case::Outside));
        scenario()
    })
}

const SHIPPED_OUTLIERS: [&str; 2] = [
    "oteryn:creature.enraged_arbaziloth",
    "oteryn:creature.the_percht_queen",
];

#[test]
fn w22a_shipped_outliers_settle_via_native_kill_and_personal_custody() -> TestResult {
    use super::boss_rewards_fixtures::{Case, run_case};

    for creature in SHIPPED_OUTLIERS {
        run_case(Case::ShippedLootOutlier(creature))?;
    }
    Ok(())
}

/// Check the same admitted reward definitions that the kill owner reads. The manifest
/// can retain source tables whose corpse or item closure refuses settlement.
#[test]
fn w22a_census_plans_every_admitted_shipped_loot_table() -> TestResult {
    use super::fixture::{debug, fence};
    use crate::content::creature_reward::LootTablesSection;
    use oteryn_gameplay_items::loot_plan::{
        COMBAT01_LOOT_PLAN_ENTRIES_MAX, LootPlanDeathKey, LootTableEntry, plan_boss_loot,
        plan_creature_loot,
    };
    use std::collections::{BTreeMap, BTreeSet};

    fn entries_in_tree(entries: &[LootTableEntry]) -> usize {
        entries.len()
            + entries
                .iter()
                .map(|entry| entries_in_tree(&entry.details.children))
                .sum::<usize>()
    }

    let world = fence(1)?.runtime_scope.world_id();
    let (controller, _) =
        super::death_ml_loss_composed_fixtures::production_generation_with_pin(world, "content-1")?;
    let native = controller
        .active()
        .and_then(|active| active.native_gameplay())
        .ok_or("admitted shipped gameplay")?;
    let section: LootTablesSection = serde_json::from_slice(include_bytes!(
        "../../../../../../tools/content-schema/native-gameplay/loot-tables.json"
    ))?;
    let source: BTreeSet<_> = section
        .tables
        .iter()
        .map(|table| table.identity.key.as_str())
        .collect();
    let mut admitted = BTreeSet::new();
    let mut refused = BTreeSet::new();
    let mut reasons = BTreeMap::new();
    let mut outliers = BTreeMap::new();
    let mut largest = (0, "");
    let mut largest_plan = 0;
    for binding in &section.creature_loot {
        let Some(reference) = &binding.loot else {
            continue;
        };
        let table_key = reference.key.as_str();
        let creature = &binding.creature.key;
        let rewards = native.reward_table();
        let row = match rewards.row(creature) {
            Ok(row) => row,
            Err(reason) => {
                refused.insert(table_key);
                *reasons.entry(reason.as_str()).or_insert(0) += 1;
                continue;
            }
        };
        assert!(source.contains(table_key));
        admitted.insert(table_key);
        let reference = row
            .loot_table_ref
            .as_ref()
            .ok_or("admitted table identity")?;
        assert_eq!(reference.production_key, table_key);
        let count = entries_in_tree(&row.loot_table.entries);
        assert!(
            count <= COMBAT01_LOOT_PLAN_ENTRIES_MAX,
            "{table_key}: {count}"
        );
        largest = largest.max((count, table_key));
        let private = rewards.reward_boss_corpse(creature).is_some();
        if crate::combat::check_corpse_container_capacity(row.loot_table.entries.len()).is_err() {
            outliers.insert(creature.as_str(), (count, private));
        }
        for seed in 1..=8 {
            let plan = if private {
                plan_boss_loot([seed; 16], [2; 16], reference, &row.loot_table, 1.0, true)
            } else {
                plan_creature_loot(
                    LootPlanDeathKey::new([seed; 16], [2; 16], 1, 1, 1),
                    reference,
                    &row.loot_table,
                )
            }
            .map_err(|error| format!("{table_key}: {error}"))?;
            largest_plan = largest_plan.max(plan.entries.len());
            assert!(
                plan.entries
                    .iter()
                    .all(|entry| !entry.needs_subtype_state()),
                "{table_key}"
            );
            if !private {
                let direct = plan
                    .entries
                    .iter()
                    .filter(|entry| entry.parent.is_none())
                    .count();
                crate::combat::check_corpse_container_capacity(direct).map_err(debug)?;
            }
        }
    }
    assert_eq!(
        admitted.union(&refused).copied().collect::<BTreeSet<_>>(),
        source
    );
    assert!(admitted.contains("oteryn:loot.creature.rat"));
    assert!(largest.0 > 16 && largest_plan > 0);
    assert_eq!(
        outliers.keys().copied().collect::<Vec<_>>(),
        SHIPPED_OUTLIERS,
        "every admitted table over the ordinary corpse bound needs kill coverage"
    );
    assert!(outliers.values().all(|(_, private)| *private));
    eprintln!(
        "W22A_SHIPPED_CENSUS source={} admitted={} refused_only={} reasons={reasons:?} largest={largest:?} largest_plan={largest_plan} raw_corpse_cap_outliers={outliers:?}",
        source.len(),
        admitted.len(),
        refused.difference(&admitted).count(),
    );
    Ok(())
}
