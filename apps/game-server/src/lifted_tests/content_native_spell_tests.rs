#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
use crate::content::spell_data::native::{profiles, spell_from_bundle};
use crate::spell::native::{Facts, Plan};
use crate::spell::native_items as items;
use serde_json::json;

#[test]
fn wrong_facts_fail_before_rng_and_authoring_retains_native_profile() {
    let profile = profiles()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "Blood Rage")
        .unwrap();
    let compiled = spell_from_bundle(
        &json!({"spell": profile["spell"]}),
        &profile["dependencies"],
    )
    .unwrap();
    let mut calls = 0;
    assert!(
        compiled
            .plan(Facts::Food, &mut |_, _| {
                calls += 1;
                0
            })
            .is_err()
    );
    assert_eq!(calls, 0);
    assert!(
        crate::content::spell_data::authoring::spell_from_bundle(
            &json!({"spell": profile["spell"]}),
            &profile["dependencies"]
        )
        .is_ok()
    );
}

#[test]
fn food_and_barrier_dispatch_real_qualified_data() {
    let all = profiles().unwrap();
    let food = all.iter().find(|p| p["name"] == "Food").unwrap();
    let compiled =
        spell_from_bundle(&json!({"spell": food["spell"]}), &food["dependencies"]).unwrap();
    let mut draws = vec![1, 2, 7].into_iter();
    let Plan::Food { items, overflow } = compiled
        .plan(Facts::Food, &mut |_, _| draws.next().unwrap())
        .unwrap()
    else {
        panic!("wrong plan")
    };
    assert_eq!(items.len(), 2);
    assert_eq!(overflow, items::ItemGrantOverflow::DropOnCasterTile);
    let wall = all
        .iter()
        .find(|p| p["name"] == "Magic Wall Rune" && p["carrier"] == "rune")
        .unwrap();
    let compiled =
        spell_from_bundle(&json!({"spell": wall["spell"]}), &wall["dependencies"]).unwrap();
    let tile = items::BarrierTileFacts {
        exists: true,
        floor_change: false,
        creature_on_tile: false,
    };
    let Plan::Barrier(plan) = compiled
        .plan(
            Facts::Barrier {
                tile: &tile,
                optional_pvp: true,
                sample_seconds: 20,
                caster_name: "Caster",
            },
            &mut |_, _| panic!("barrier sampling already supplied"),
        )
        .unwrap()
    else {
        panic!("wrong plan")
    };
    assert_eq!(plan.item.numeric_id().unwrap(), 10181);
    assert_eq!(plan.duration_ms, 20000);
}
