//! Synthetic policies for the composed wave regression.
use super::{TestResult, debug};
use crate::foundation::{ChannelRuntimeV1, CompiledCreaturePolicies, ExactActorRef};

const EXPECTED: [(&str, i64, i64); 5] = [
    ("oteryn:creature.domestikion", 750, 1100),
    ("oteryn:creature.hoodinion", 800, 1173),
    ("oteryn:creature.mearidion", 850, 1246),
    ("oteryn:creature.murmillion", 1100, 1613),
    ("oteryn:creature.scissorion", 950, 1393),
];

pub(super) fn fixture_policies(
    original: &CompiledCreaturePolicies,
) -> TestResult<CompiledCreaturePolicies> {
    let mut rows = original
        .policies()
        .iter()
        .map(|row| row.as_ref().clone())
        .collect::<Vec<_>>();
    // Test-only infrastructure for fields Global does not publish. There are no
    // attacks or placements authored from this table, and it is never activated.
    let base = rows
        .iter()
        .find(|row| row.definition_key == "oteryn:creature.rat")
        .ok_or("fixture rat policy")?
        .clone();
    for (key, health, _) in EXPECTED {
        let mut row = base.clone();
        row.definition_key = key.to_owned();
        row.display_name = key.to_owned();
        row.maximum_health = health;
        rows.push(row);
    }
    CompiledCreaturePolicies::from_active_artifact(original.source_digest(), rows).map_err(debug)
}

pub(super) fn assert_wave(
    runtime: &ChannelRuntimeV1,
    targets: &[ExactActorRef],
    wave: u32,
) -> TestResult {
    for &target in targets {
        let snapshot = runtime.companion_snapshot(target).map_err(debug)?;
        let key = &snapshot.state.policy.definition_key;
        let (_, _, expected) = EXPECTED
            .iter()
            .find(|(native, _, _)| *native == key.as_str())
            .ok_or_else(|| {
                format!("Drome spawned an opponent outside the five native keys: {key}")
            })?;
        assert_eq!(
            (snapshot.health, snapshot.maximum_health),
            (*expected, *expected),
            "{key} in wave {wave}"
        );
    }
    Ok(())
}
