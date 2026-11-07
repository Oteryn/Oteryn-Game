//! Reuse the ordinary physical policy fixture; it supplies no Drome profile qualification.
use super::*;
fn scaled() -> TestResult<(ChannelRuntimeV1, ExactActorRef, i64)> {
    let (mut runtime, _, _) = fixture(8)?;
    let actor = ordinary(&mut runtime, at(102, 100, 7))?;
    runtime
        .scale_tibiadrome_opponent(actor, 20)
        .map_err(debug)?;
    let maximum = runtime
        .companion_snapshot(actor)
        .map_err(debug)?
        .maximum_health;
    assert!(maximum > 106);
    runtime
        .set_test_creature_health(actor, maximum - 5)
        .map_err(debug)?;
    Ok((runtime, actor, maximum))
}
#[test]
fn srx3_drome_native_heal_current_maximum_read() -> TestResult {
    let (runtime, actor, maximum) = scaled()?;
    assert_eq!(
        runtime
            .read_source_creature_health(actor, "oteryn:creature.rat", 101)
            .map_err(debug)?,
        (maximum - 5) as u64
    );
    Ok(())
}
#[test]
fn srx3_drome_native_heal_current_maximum_area() -> TestResult {
    let (mut runtime, actor, maximum) = scaled()?;
    let receipts = runtime
        .commit_source_creature_heal_batch(
            [1; 32],
            &[(actor, "oteryn:creature.rat".into(), 101, 10)],
        )
        .map_err(debug)?;
    assert_eq!(receipts[0].health_after, maximum);
    assert_eq!(runtime.live_creature_health(actor), Some(maximum as u64));
    Ok(())
}
#[test]
fn srx3_drome_native_heal_current_maximum_self() -> TestResult {
    let (mut runtime, actor, maximum) = scaled()?;
    let registration = runtime
        .bind_creature_self_heal("oteryn:creature.rat", "oteryn:ability.rat.heal", 101, 0, 10)
        .map_err(debug)?;
    let mut ledger = CreatureSelfHealLedger::default();
    let receipt = runtime
        .commit_creature_self_heal(&mut ledger, &registration, actor, 1, 10)
        .map_err(debug)?;
    assert_eq!(receipt.health_after, maximum);
    assert_eq!(runtime.live_creature_health(actor), Some(maximum as u64));
    let replay = runtime
        .commit_creature_self_heal(&mut ledger, &registration, actor, 1, 10)
        .map_err(debug)?;
    assert!(!replay.applied);
    assert_eq!(runtime.live_creature_health(actor), Some(maximum as u64));
    Ok(())
}
