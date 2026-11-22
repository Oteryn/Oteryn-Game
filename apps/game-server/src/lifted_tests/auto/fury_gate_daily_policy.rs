use crate::durability::world_event::changes;
use oteryn_gameplay_world_change::daily::roll_calendar;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

#[test]
fn fury_gate_reset_uses_authored_activation_and_one_city_for_entry_and_return()
-> Result<(), Box<dyn std::error::Error>> {
    let source: Value = serde_json::from_str(include_str!(
        "../../../../../content/encounters/world-changes/fury_gates/selection-source.json"
    ))?;
    let chance = u128::from(
        source["trigger"]["chance_permille"]
            .as_u64()
            .ok_or("Fury chance")?,
    );
    let cities = source["variants"].as_array().ok_or("Fury city list")?;
    assert_eq!(cities.len(), 10);

    let world = crate::content::native_gameplay_test_world_id()?;
    let mut hash = Sha256::new();
    hash.update(b"oteryn.world_change.daily.v1\0");
    hash.update(world.as_bytes());
    let root = GameplayDecisionRoot::from_bytes(hash.finalize().into());
    let mut seen = [false; 10];
    for epoch in 1..=128_u64 {
        let (active, city) = expected_selection(&root, epoch, chance, cities.len())?;
        seen[city] = true;
        let city_key = cities[city]["variant_key"]
            .as_str()
            .ok_or("Fury city key")?;
        let layer = format!("fury_gates/{city_key}");
        let expected_entry = active.then_some(layer.as_str());
        // Reset writes both the daily row and the native occurrence for map projection.
        for day in [
            changes::roll(world, epoch, 0)?,
            roll_calendar(world.as_bytes(), epoch, 1, 1),
        ] {
            assert_eq!(day.fury_gates, expected_entry, "epoch {epoch}");
            assert_eq!(day.exit_layers(), [layer.as_str()], "epoch {epoch}");
        }
    }
    assert!(seen.into_iter().all(|city| city));
    Ok(())
}

fn expected_selection(
    root: &GameplayDecisionRoot,
    epoch: u64,
    chance: u128,
    city_count: usize,
) -> Result<(bool, usize), Box<dyn std::error::Error>> {
    let mut occurrence = [0; 16];
    occurrence[8..].copy_from_slice(&epoch.to_be_bytes());
    let draw = |purpose| {
        deterministic_decision_u64(
            root,
            DecisionOccurrenceId::from_bytes(occurrence),
            purpose,
            0,
        )
        .map(u128::from)
    };
    let active = (draw("fury-gates-active")? * 1000) >> 64 < chance;
    let city = usize::try_from((draw("fury-gates-town")? * city_count as u128) >> 64)?;
    Ok((active, city))
}
