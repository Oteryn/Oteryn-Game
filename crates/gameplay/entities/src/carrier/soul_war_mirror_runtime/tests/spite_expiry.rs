//! Source Global expiry targets registered Spite independently of live combat actionability.
use super::*;
// Offline insertion into soul_war_mirror_runtime/tests.rs after41 freeze.
fn spite_expiry_fixture(instance: bool) -> (ChannelRuntimeV1, ExactActorRef) {
    let (mut runtime, _, _, image, _) = fixture_with_scope(9, instance);
    runtime
        .remove_encounter_actor(image)
        .expect("replace native image fixture");
    let key = crate::carrier::soul_war_spite::SPITE;
    let maximum = runtime
        .companion_policy(key)
        .expect("actual native Spite")
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
            .expect("actual native Spite slot"),
    );
    runtime
        .install_creature_policy(actor, key)
        .expect("current native source policy");
    (runtime, actor)
}
#[test]
fn srx3_spite_global_expiry_changes_current_world_actor_defense_without_instance_gate() {
    use crate::carrier::soul_war_spite::SpiteChange;
    let (mut runtime, boss) = spite_expiry_fixture(false);
    let before = runtime.companion_snapshot(boss).unwrap();
    let base = runtime.effective_creature_defense(&before).unwrap();
    assert!(runtime.source_creature_is_registered(boss).unwrap());
    runtime
        .change_soul_war_spite(&before, base, SpiteChange::FireExpired)
        .expect("registered source GlobalEvent has no Instance-only predicate");
    let after = runtime.companion_snapshot(boss).unwrap();
    assert_eq!(
        runtime.effective_creature_defense(&after).unwrap(),
        base + 10
    );
    assert_eq!(after.health, before.health);
}
#[test]
fn srx3_spite_global_expiry_can_add_defense_before_queued_death_without_reviving_completed_actor() {
    use crate::carrier::soul_war_spite::{SPITE, SpiteChange};
    let (mut runtime, boss) = spite_expiry_fixture(true);
    runtime.set_test_creature_health(boss, 1).unwrap();
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
        .unwrap();
    assert_eq!(hit.health_after, 0, "canonical lethal prerequisite");
    let lethal = runtime
        .borrow_combat_death()
        .committed_lethal_receipt(boss)
        .unwrap();
    let key = lethal.projection.occurrence.death_key().unwrap();
    runtime
        .borrow_combat_death()
        .project_committed_lethal(lethal)
        .unwrap();
    assert!(runtime.source_creature_is_registered(boss).unwrap());
    let before = runtime.companion_snapshot_including_dead(boss).unwrap();
    let base = runtime.effective_creature_defense(&before).unwrap();
    assert!(
        runtime
            .change_soul_war_spite(&before, base, SpiteChange::WeepingHeal)
            .is_err(),
        "positive addHealth must not revive HP0"
    );
    runtime
        .change_soul_war_spite(&before, base, SpiteChange::FireExpired)
        .expect("registered HP0 remains available to queued current-name lookup");
    let after = runtime.companion_snapshot_including_dead(boss).unwrap();
    assert_eq!(after.health, 0);
    assert_eq!(
        runtime.effective_creature_defense(&after).unwrap(),
        base + 10
    );
    runtime.complete_source_creature_death(boss, key).unwrap();
    assert!(!runtime.source_creature_is_registered(boss).unwrap());
    assert!(
        runtime
            .change_soul_war_spite(&after, base + 10, SpiteChange::FireExpired)
            .is_err(),
        "completed source actor is no longer the named Creature"
    );
}
