//! Crystal game.cpp7400-7401: absorbed heal has NULL source; only a nonzero
//! residual Player loss executes that Player's health-change event.
use super::*;
use runtime_actor_spell_types::OwnerCombatChange;

fn apply(runtime: &mut ChannelRuntimeV1, batch: &runtime_actor_spell_types::OwnerCombatBatch) {
    let prepared = runtime
        .stage_spell_batch(batch)
        .expect("canonical composed HP preparation");
    assert!(
        runtime
            .commit_spell_batch(prepared)
            .expect("known canonical HP receipt")
            .applied
    );
}
#[test]
fn srx3_mirror_full_absorption_null_heal_does_not_invoke_player_health_event() {
    let (mut runtime, player, session, image, _) = fixture(8);
    let before = runtime
        .companion_snapshot(image)
        .expect("actual native Mirror");
    runtime
        .set_test_creature_health(image, before.maximum_health - 1000)
        .expect("current injury");
    let batch = cloak_health_batch(
        player,
        session,
        image,
        1,
        OwnerCombatChange::DamageWithHealing {
            target_atom: before.state.policy.definition_key.clone(),
            damage: 0,
            healing: 100,
        },
    );
    apply(&mut runtime, &batch);
    let after = runtime
        .companion_snapshot(image)
        .expect("NULL absorbed heal must retain the original Mirror actor");
    assert_eq!(after.actor, image);
    assert_eq!(
        after.state.policy.definition_key,
        before.state.policy.definition_key
    );
    assert_eq!(after.health, before.maximum_health - 900);
}
#[test]
fn srx3_mirror_positive_residual_keeps_actual_player_health_event() {
    let (mut runtime, player, session, image, _) = fixture(8);
    let before = runtime
        .companion_snapshot(image)
        .expect("actual native Mirror");
    let batch = cloak_health_batch(
        player,
        session,
        image,
        1,
        OwnerCombatChange::DamageWithHealing {
            target_atom: before.state.policy.definition_key.clone(),
            damage: 100,
            healing: 200,
        },
    );
    apply(&mut runtime, &batch);
    assert!(
        runtime.companion_snapshot(image).is_err(),
        "actual Player residual must still invoke Mirror's replacement event"
    );
}
fn hatred(damage: i64) -> (ChannelRuntimeV1, ExactActorRef, i64) {
    let (mut runtime, player, session, _, _) = fixture_with_scope(8, true);
    let key = "oteryn:creature.goshnar_s_hatred";
    let policy = runtime.companion_policy(key).expect("actual native Hatred");
    let at = runtime
        .read_actor_position(player)
        .expect("current owner")
        .position();
    let boss = runtime
        .admit_pinned_monster_lab_creature(at, key, policy.maximum_health)
        .expect("native boss");
    runtime
        .install_creature_policy(boss, key)
        .expect("sealed native HH policy");
    runtime
        .set_test_creature_health(boss, policy.maximum_health - 1000)
        .expect("actual injury");
    let before = runtime
        .companion_snapshot(boss)
        .expect("current source boss");
    runtime
        .change_soul_war_hatred_percent(
            &before,
            0,
            super::super::super::soul_war_hatred::HatredEmpowerment::Add(10),
        )
        .expect("source prior percent");
    let defense = runtime
        .effective_creature_defense(&before)
        .expect("current base defense");
    assert_eq!(defense, 160);
    let batch = cloak_health_batch(
        player,
        session,
        boss,
        1,
        OwnerCombatChange::DamageWithHealing {
            target_atom: key.into(),
            damage,
            healing: 200,
        },
    );
    apply(&mut runtime, &batch);
    (runtime, boss, before.health + 200 - damage)
}
#[test]
fn srx3_hatred_full_absorption_null_heal_does_not_advance_defense() {
    let (runtime, boss, health) = hatred(0);
    let after = runtime
        .companion_snapshot(boss)
        .expect("same native Hatred");
    assert_eq!(
        after.health, health,
        "NULL absorbed heal must still update canonical HP"
    );
    assert_eq!(
        runtime
            .effective_creature_defense(&after)
            .expect("current defense"),
        160,
        "zero residual has no Player health-change event"
    );
}
#[test]
fn srx3_hatred_positive_residual_advances_defense_exactly_once() {
    let (runtime, boss, health) = hatred(100);
    let after = runtime
        .companion_snapshot(boss)
        .expect("same native Hatred");
    assert_eq!(after.health, health);
    assert_eq!(
        runtime
            .effective_creature_defense(&after)
            .expect("current defense"),
        170,
        "positive residual retains exactly one actual Player event"
    );
}
