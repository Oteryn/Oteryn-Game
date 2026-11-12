use super::*;

#[test]
fn live_echo_children_expire_from_each_actual_spawn_without_removing_natural_actors() -> TestResult
{
    let (mut runtime, player, session) = fixture(32)?;
    let natural = ordinary(&mut runtime, at(101, 100, 7))?;
    activate(&mut runtime, player, session, 0)?;
    let first = runtime.echo_wardens.spawned.clone();
    assert_eq!(first.len(), 2);
    for step in 1..=6 {
        runtime
            .drive_echo_wardens(30_000 + step * 400, |_| true)
            .map_err(debug)?;
    }
    let later = runtime.echo_wardens.spawned[2];
    runtime
        .drive_echo_wardens(629_999, |_| true)
        .map_err(debug)?;
    for actor in &first {
        assert!(runtime.companion_snapshot(*actor).is_ok());
    }
    runtime
        .drive_echo_wardens(630_000, |_| true)
        .map_err(debug)?;
    for actor in first {
        assert!(
            runtime.companion_snapshot(actor).is_err(),
            "live raid child must expire at ten minutes"
        );
    }
    assert_eq!(runtime.next_echo_due_ms(), Some(630_400));
    assert!(runtime.companion_snapshot(later).is_ok());
    assert!(runtime.companion_snapshot(natural).is_ok());
    runtime
        .drive_echo_wardens(632_400, |_| true)
        .map_err(debug)?;
    assert!(runtime.echo_wardens.spawned.is_empty());
    assert!(runtime.companion_snapshot(natural).is_ok());
    assert_eq!(runtime.next_echo_due_ms(), None);
    Ok(())
}

#[test]
fn expiry_preserves_dead_child_until_pending_corpse_settlement_finishes() -> TestResult {
    let (mut runtime, player, session) = fixture(32)?;
    activate(&mut runtime, player, session, 0)?;
    let warden = runtime.echo_wardens.wardens.first().ok_or("warden")?.0;
    death(&mut runtime, warden, 31_000)?;
    runtime.pin_corpse_projection(warden).map_err(debug)?;
    runtime
        .drive_echo_wardens(630_000, |_| true)
        .map_err(debug)?;
    assert!(runtime.companion_snapshot_including_dead(warden).is_ok());
    runtime.unpin_corpse_projection(warden);
    runtime
        .drive_echo_wardens(630_400, |_| true)
        .map_err(debug)?;
    assert!(runtime.companion_snapshot_including_dead(warden).is_err());
    Ok(())
}

#[test]
fn expiry_cannot_remove_a_replacement_actor_in_the_same_slot() -> TestResult {
    let (mut runtime, player, session) = fixture(4)?;
    activate(&mut runtime, player, session, 0)?;
    let retired = runtime.echo_wardens.wardens.first().ok_or("warden")?.0;
    runtime.remove_encounter_actor(retired).map_err(debug)?;
    let replacement = ordinary(&mut runtime, at(102, 100, 7))?;
    assert_eq!(retired.0.actor_local_id, replacement.0.actor_local_id);
    assert_ne!(
        retired.0.actor_local_generation,
        replacement.0.actor_local_generation
    );
    runtime
        .drive_echo_wardens(630_000, |_| true)
        .map_err(debug)?;
    assert!(runtime.companion_snapshot(replacement).is_ok());
    assert!(runtime.companion_snapshot(retired).is_err());
    Ok(())
}
