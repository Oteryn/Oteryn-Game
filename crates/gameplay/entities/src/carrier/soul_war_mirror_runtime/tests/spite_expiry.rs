//! Source Global expiry targets registered Spite independently of live combat actionability.
use super::*;
// Offline insertion into soul_war_mirror_runtime/tests.rs after41 freeze.
fn spite_expiry_fixture(instance: bool) -> Result<(ChannelRuntimeV1, ExactActorRef), String> {
    let (mut runtime, _, _, image, _) = fixture_with_scope(9, instance);
    runtime
        .remove_encounter_actor(image)
        .map_err(|error| format!("{error:?}"))?;
    let key = crate::carrier::soul_war_spite::SPITE;
    let maximum = runtime
        .companion_policy(key)
        .map_err(|error| format!("{error:?}"))?
        .maximum_health;
    let actor = ExactActorRef(
        runtime
            .admit_encounter_fixture_actor(
                ActorState(2),
                key,
                maximum,
                Some(LocalPosition {
                    x: 33743,
                    y: 31628,
                    floor: 14,
                }),
            )
            .map_err(|error| format!("{error:?}"))?,
    );
    runtime
        .install_creature_policy(actor, key)
        .map_err(|error| format!("{error:?}"))?;
    Ok((runtime, actor))
}
#[test]
fn srx3_spite_global_expiry_changes_current_world_actor_defense_without_instance_gate()
-> Result<(), String> {
    use crate::carrier::soul_war_spite::SpiteChange;
    let (mut runtime, boss) = spite_expiry_fixture(false)?;
    let before = runtime
        .companion_snapshot(boss)
        .map_err(|error| format!("{error:?}"))?;
    let base = runtime
        .effective_creature_defense(&before)
        .map_err(|error| format!("{error:?}"))?;
    assert!(
        runtime
            .source_creature_is_registered(boss)
            .map_err(|error| format!("{error:?}"))?
    );
    runtime
        .change_soul_war_spite(&before, base, SpiteChange::FireExpired)
        .map_err(|error| format!("{error:?}"))?;
    let after = runtime
        .companion_snapshot(boss)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        runtime
            .effective_creature_defense(&after)
            .map_err(|error| format!("{error:?}"))?,
        base + 10
    );
    assert_eq!(after.health, before.health);
    Ok(())
}
#[test]
fn srx3_spite_global_expiry_can_add_defense_before_queued_death_without_reviving_completed_actor()
-> Result<(), String> {
    use crate::carrier::soul_war_spite::{SPITE, SpiteChange};
    let (mut runtime, boss) = spite_expiry_fixture(true)?;
    runtime
        .set_test_creature_health(boss, 1)
        .map_err(|error| format!("{error:?}"))?;
    let hit = runtime
        .borrow_exact_actor_commit_at(200)
        .commit_damage(
            boss,
            OwnerDamageCommand {
                target: SPITE.as_bytes(),
                occurrence: b"spite-expiry-lethal",
                binding: b"spite-expiry-lethal\0current",
                damage: 1,
            },
        )
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(hit.health_after, 0, "canonical lethal prerequisite");
    let lethal = runtime
        .borrow_combat_death()
        .committed_lethal_receipt(boss)
        .map_err(|error| format!("{error:?}"))?;
    let key = lethal
        .projection
        .occurrence
        .death_key()
        .map_err(|error| format!("{error:?}"))?;
    runtime
        .borrow_combat_death()
        .project_committed_lethal(lethal)
        .map_err(|error| format!("{error:?}"))?;
    assert!(
        runtime
            .source_creature_is_registered(boss)
            .map_err(|error| format!("{error:?}"))?
    );
    let before = runtime
        .companion_snapshot_including_dead(boss)
        .map_err(|error| format!("{error:?}"))?;
    let base = runtime
        .effective_creature_defense(&before)
        .map_err(|error| format!("{error:?}"))?;
    assert!(
        runtime
            .change_soul_war_spite(&before, base, SpiteChange::WeepingHeal)
            .is_err(),
        "positive addHealth must not revive HP0"
    );
    runtime
        .change_soul_war_spite(&before, base, SpiteChange::FireExpired)
        .map_err(|error| format!("{error:?}"))?;
    let after = runtime
        .companion_snapshot_including_dead(boss)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(after.health, 0);
    assert_eq!(
        runtime
            .effective_creature_defense(&after)
            .map_err(|error| format!("{error:?}"))?,
        base + 10
    );
    runtime
        .complete_source_creature_death(boss, key)
        .map_err(|error| format!("{error:?}"))?;
    assert!(
        !runtime
            .source_creature_is_registered(boss)
            .map_err(|error| format!("{error:?}"))?
    );
    assert!(
        runtime
            .change_soul_war_spite(&after, base + 10, SpiteChange::FireExpired)
            .is_err(),
        "completed source actor is no longer the named Creature"
    );
    Ok(())
}
