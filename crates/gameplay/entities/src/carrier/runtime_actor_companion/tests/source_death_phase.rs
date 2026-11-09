use super::*;

fn registered_source_fixture_at(scope_id: u8) -> (ChannelRuntimeV1, ExactActorRef) {
    let world = WorldId::decode(&id(1)).unwrap();
    let scope = RuntimeScopeRefV1::instance(world, id(scope_id)).unwrap();
    let mut runtime = ChannelRuntimeV1::from_committed_scope_assignment(
        scope,
        NodeId::decode(&id(3)).unwrap(),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        8,
        ChannelContentPin::test(world),
    )
    .unwrap();
    runtime
        .install_companion_policies(crate::carrier::soul_war_mirror_runtime::tests::policies())
        .unwrap();
    let key = "oteryn:creature.goshnar_s_megalomania_blue";
    let policy = runtime.companion_policy(key).unwrap();
    let actor = ExactActorRef(
        runtime
            .admit_encounter_fixture_actor(
                ActorState(1),
                key,
                policy.maximum_health,
                Some(LocalPosition {
                    x: 33710,
                    y: 31634,
                    floor: 14,
                }),
            )
            .unwrap(),
    );
    runtime.install_creature_policy(actor, key).unwrap();
    (runtime, actor)
}
fn canonical_lethal(
    runtime: &mut ChannelRuntimeV1,
    actor: ExactActorRef,
    key: &str,
) -> CreatureDeathOccurrenceKey {
    runtime.set_test_creature_health(actor, 1).unwrap();
    let hit = runtime
        .borrow_exact_actor_commit_at(200)
        .commit_damage(
            actor,
            OwnerDamageCommand {
                target: key.as_bytes(),
                occurrence: b"source-death-phase",
                binding: b"source-death-phase\0current",
                damage: 1,
            },
        )
        .unwrap();
    assert_eq!(hit.health_after, 0);
    let receipt = runtime
        .borrow_combat_death()
        .committed_lethal_receipt(actor)
        .unwrap();
    let key = receipt.projection.occurrence.death_key().unwrap();
    runtime
        .borrow_combat_death()
        .project_committed_lethal(receipt)
        .unwrap();
    key
}

#[test]
fn srx3_source_death_null_action_reserves_real_begin_without_player_session() {
    let (mut runtime, boss) = registered_source_fixture_at(14);
    canonical_lethal(
        &mut runtime,
        boss,
        "oteryn:creature.goshnar_s_megalomania_blue",
    );
    let notice = runtime.next_source_creature_death().unwrap().unwrap();
    let begin = runtime.begin_source_creature_death(&notice).unwrap();
    assert!(!begin.has_reward_damage_credit());
    let event = &begin.events()[0];
    let before = runtime.companion_snapshot_including_dead(boss).unwrap();
    let change = runtime
        .prepare_soul_war_megalomania_profile_change(
            &before,
            "oteryn:creature.goshnar_s_megalomania_purple",
        )
        .unwrap();
    assert!(
        runtime
            .reserve_source_death_action(&begin, event, &[boss; 9])
            .is_err(),
        "source reservation cannot exceed actual eight fixed slots"
    );
    let reservation = runtime
        .reserve_source_death_action(&begin, event, &[])
        .expect("actual NULL source BEGIN owns its reservation without a fake Player");
    let replay = reservation.clone();
    assert!(
        runtime
            .reserve_source_death_action(&begin, event, &[])
            .is_err(),
        "a concurrent action cannot borrow the reserved cursor"
    );
    runtime
        .validate_source_death_action_reservation(&reservation)
        .unwrap();
    assert!(
        runtime
            .commit_creature_profile_transformation(change)
            .is_err(),
        "detached SQL must not allow another owner to replace the source incarnation"
    );
    assert!(
        runtime
            .acknowledge_source_creature_death_event(&begin, event)
            .is_err(),
        "a pending physical outcome cannot advance the copied cursor"
    );
    runtime
        .release_source_death_action_reservation(reservation)
        .unwrap();
    assert!(
        runtime
            .release_source_death_action_reservation(replay)
            .is_err(),
        "a cloned observation cannot release an already settled reservation twice"
    );
    let change = runtime
        .prepare_soul_war_megalomania_profile_change(
            &before,
            "oteryn:creature.goshnar_s_megalomania_purple",
        )
        .unwrap();
    runtime
        .commit_creature_profile_transformation(change)
        .unwrap();
    runtime
        .acknowledge_source_creature_death_event(&begin, event)
        .unwrap();
}

#[test]
fn srx3_source_registered_death_phase_includes_zero_health_before_completion_and_survives_projection_eviction()
 {
    let (mut runtime, boss) = registered_source_fixture_at(2);
    assert!(runtime.source_creature_is_registered(boss).unwrap());
    let key = canonical_lethal(
        &mut runtime,
        boss,
        "oteryn:creature.goshnar_s_megalomania_blue",
    );
    assert!(runtime.retains_corpse_projection(boss));
    assert!(
        runtime.source_creature_is_registered(boss).unwrap(),
        "C++ source lookup includes HP0 until its queued onDeath finishes"
    );
    assert!(
        runtime
            .source_registered_creature_census()
            .unwrap()
            .iter()
            .any(|(actor, _)| *actor == boss)
    );
    let seal = runtime.complete_source_creature_death(boss, key).unwrap();
    assert_eq!(seal.death_key(), key);
    assert_eq!(seal.actor(), boss);
    assert_eq!(
        seal.definition_key(),
        "oteryn:creature.goshnar_s_megalomania_blue"
    );
    assert!(!runtime.source_creature_is_registered(boss).unwrap());
    assert!(
        !runtime
            .source_registered_creature_census()
            .unwrap()
            .iter()
            .any(|(actor, _)| *actor == boss)
    );
    assert_eq!(
        runtime
            .complete_source_creature_death(boss, key)
            .unwrap()
            .death_key(),
        key
    );

    for _ in 0..129 {
        let rat = "oteryn:creature.rat";
        let policy = runtime.companion_policy(rat).unwrap();
        let actor = ExactActorRef(
            runtime
                .admit_encounter_fixture_actor(
                    ActorState(1),
                    rat,
                    policy.maximum_health,
                    Some(LocalPosition {
                        x: 33711,
                        y: 31634,
                        floor: 14,
                    }),
                )
                .unwrap(),
        );
        runtime.install_creature_policy(actor, rat).unwrap();
        canonical_lethal(&mut runtime, actor, rat);
        runtime.remove_encounter_actor(actor).unwrap();
    }
    assert!(
        !runtime.retains_corpse_projection(boss),
        "actual bounded projection owner evicts the original unpinned projection"
    );
    assert_eq!(
        runtime
            .companion_snapshot_including_dead(boss)
            .unwrap()
            .health,
        0,
        "the pinned-style physical incarnation remains present"
    );
    assert!(
        !runtime.source_creature_is_registered(boss).unwrap(),
        "logical onDeath completion cannot expire with projection retention"
    );
}

#[test]
fn srx3_source_registered_death_phase_refuses_foreign_or_uncommitted_death_identity() {
    let (mut runtime, boss) = registered_source_fixture_at(2);
    let (mut other, alien) = registered_source_fixture_at(3);
    canonical_lethal(
        &mut runtime,
        boss,
        "oteryn:creature.goshnar_s_megalomania_blue",
    );
    let alien_key = canonical_lethal(
        &mut other,
        alien,
        "oteryn:creature.goshnar_s_megalomania_blue",
    );
    assert!(
        runtime
            .complete_source_creature_death(boss, alien_key)
            .is_err(),
        "an actual dead slot cannot borrow a foreign scope death key"
    );
    assert!(runtime.source_creature_is_registered(boss).unwrap());
}

#[test]
fn srx3_source_death_fifo_records_actual_lethal_before_reward_or_first_think() {
    let (mut runtime, boss) = registered_source_fixture_at(6);
    let death = canonical_lethal(
        &mut runtime,
        boss,
        "oteryn:creature.goshnar_s_megalomania_blue",
    );
    let notice = runtime.next_source_creature_death().unwrap().expect(
        "canonical HP publication retains source death without a reward consumer or first think",
    );
    assert_eq!(notice.actor(), boss);
    assert_eq!(notice.death_key(), death);
    assert_eq!(notice.enqueued_at_ms(), Some(200));
    assert_eq!(
        runtime
            .next_source_creature_death()
            .unwrap()
            .unwrap()
            .death_key(),
        death
    );
    assert!(runtime.source_creature_is_registered(boss).unwrap());
}

#[test]
fn srx3_source_death_begin_freezes_profile_and_copied_events_once_before_handlers() {
    let (mut runtime, boss) = registered_source_fixture_at(7);
    let death = canonical_lethal(
        &mut runtime,
        boss,
        "oteryn:creature.goshnar_s_megalomania_blue",
    );
    let notice = runtime.next_source_creature_death().unwrap().unwrap();
    let begin = runtime.begin_source_creature_death(&notice).unwrap();
    assert!(
        !begin.has_reward_damage_credit(),
        "a NULL lethal source freezes no ordinary reward principal at BEGIN"
    );
    assert_eq!(
        begin.definition_key(),
        "oteryn:creature.goshnar_s_megalomania_blue"
    );
    assert_eq!(begin.events().len(), 1);
    assert_eq!(begin.events()[0].name(), "MegalomaniaDeath");
    let before = runtime.companion_snapshot_including_dead(boss).unwrap();
    let change = runtime
        .prepare_soul_war_megalomania_profile_change(
            &before,
            "oteryn:creature.goshnar_s_megalomania_purple",
        )
        .unwrap();
    runtime
        .commit_creature_profile_transformation(change)
        .unwrap();
    let retry = runtime.begin_source_creature_death(&notice).unwrap();
    assert_eq!(retry.definition_key(), begin.definition_key());
    assert_eq!(
        retry.events(),
        begin.events(),
        "CPP keeps its copied event vector while the actor may change name/type"
    );
    assert_eq!(
        runtime.source_creature_death_event_index(&begin).unwrap(),
        Some(0)
    );
    assert!(
        runtime
            .acknowledge_source_creature_death_event(&begin, &begin.events()[0])
            .unwrap()
    );
    assert!(
        !runtime
            .acknowledge_source_creature_death_event(&begin, &begin.events()[0])
            .unwrap()
    );
    let completed = runtime.finish_source_creature_death(&begin).unwrap();
    assert_eq!(completed.death_key(), death);
    assert_eq!(completed.definition_key(), begin.definition_key());
    assert!(!runtime.source_creature_is_registered(boss).unwrap());
    assert!(runtime.next_source_creature_death().unwrap().is_none());
    let completion = runtime.finish_source_creature_death_task(&begin).unwrap();
    runtime.remove_encounter_actor(boss).unwrap();
    assert!(
        completion.current_for(&runtime),
        "known phase completion survives physical retirement under the same actual scope owner"
    );
    assert_eq!(completion.death_key(), death);
    assert_eq!(completion.begin().definition_key(), begin.definition_key());
}

#[test]
fn srx3_source_death_copied_handler_must_complete_before_registration_finishes() {
    let (mut runtime, boss) = registered_source_fixture_at(11);
    canonical_lethal(
        &mut runtime,
        boss,
        "oteryn:creature.goshnar_s_megalomania_blue",
    );
    let notice = runtime.next_source_creature_death().unwrap().unwrap();
    let begin = runtime.begin_source_creature_death(&notice).unwrap();
    assert_eq!(begin.events().len(), 1);
    assert_eq!(begin.events()[0].name(), "MegalomaniaDeath");
    assert!(
        runtime.finish_source_creature_death(&begin).is_err(),
        "a copied handler has not completed; SQL drop retention must not hide its source actor"
    );
    assert!(runtime.source_creature_is_registered(boss).unwrap());
    assert_eq!(runtime.next_source_creature_death().unwrap(), Some(notice));
}

#[test]
fn srx3_source_death_fifo_uses_publication_order_instead_of_actor_identity() {
    let (mut runtime, first) = registered_source_fixture_at(8);
    let key = "oteryn:creature.goshnar_s_megalomania_blue";
    let policy = runtime.companion_policy(key).unwrap();
    let second = ExactActorRef(
        runtime
            .admit_encounter_fixture_actor(
                ActorState(2),
                key,
                policy.maximum_health,
                Some(LocalPosition {
                    x: 33712,
                    y: 31634,
                    floor: 14,
                }),
            )
            .unwrap(),
    );
    runtime.install_creature_policy(second, key).unwrap();
    let second_death = canonical_lethal(&mut runtime, second, key);
    let first_death = canonical_lethal(&mut runtime, first, key);
    let notice = runtime.next_source_creature_death().unwrap().unwrap();
    assert_eq!(notice.actor(), second);
    assert_eq!(notice.death_key(), second_death);
    let begin = runtime.begin_source_creature_death(&notice).unwrap();
    for event in begin.events() {
        runtime
            .acknowledge_source_creature_death_event(&begin, event)
            .unwrap();
    }
    runtime.finish_source_creature_death(&begin).unwrap();
    let notice = runtime.next_source_creature_death().unwrap().unwrap();
    assert_eq!(notice.actor(), first);
    assert_eq!(notice.death_key(), first_death);
}

#[test]
fn srx3_source_completed_creature_is_null_attacker_even_while_sql_drop_retains_slot() {
    use crate::carrier::soul_war_health::HealthSourceKind;
    use crate::carrier::soul_war_player_source::SoulWarPlayerSourceOrigin;
    let (mut runtime, boss) = registered_source_fixture_at(9);
    let death = canonical_lethal(
        &mut runtime,
        boss,
        "oteryn:creature.goshnar_s_megalomania_blue",
    );
    let before = runtime
        .carrier
        .soul_war_health_context(
            &runtime.continuity,
            Some(boss),
            None,
            Some(200),
            SoulWarPlayerSourceOrigin::Condition,
            runtime.content_pin(),
        )
        .unwrap();
    assert!(matches!(before.kind, HealthSourceKind::NonPlayer));
    runtime.complete_source_creature_death(boss, death).unwrap();
    assert!(
        runtime.companion_snapshot_including_dead(boss).is_ok(),
        "SQL drop still retains actual physical slot"
    );
    let after = runtime
        .carrier
        .soul_war_health_context(
            &runtime.continuity,
            Some(boss),
            None,
            Some(201),
            SoulWarPlayerSourceOrigin::Condition,
            runtime.content_pin(),
        )
        .unwrap();
    assert!(
        matches!(after.kind, HealthSourceKind::Null),
        "CPP Game creature lookup no longer exposes a completed onDeath source"
    );
    assert_eq!(after.actor, None);
}
// Append to existing source_death_phase.rs only after coordinated source freeze.
#[test]
fn srx3_registered_hp0_source_monster_keeps_actual_top_stack_until_death_completion() {
    let (mut runtime, boss) = registered_source_fixture_at(22);
    let cell = runtime.read_actor_position(boss).unwrap().position();
    let session = GameSessionId::decode(&id(23)).unwrap();
    let reservation = runtime.reserve_fresh_session(session).unwrap();
    let player = runtime.commit_fresh_session(reservation).unwrap();
    runtime
        .initialize_pinned_test_position(player, cell)
        .unwrap();
    // A real physical exit/reentry puts the source creature above the Player.
    let before = runtime.read_actor_position(boss).unwrap();
    let away = MovementLocalPosition {
        x: cell.x + 1,
        ..cell
    };
    let moved = runtime
        .borrow_movement_position()
        .commit_creature_relocation(boss, before, away)
        .unwrap();
    runtime
        .borrow_movement_position()
        .commit_creature_relocation(boss, moved, cell)
        .unwrap();
    assert_eq!(runtime.current_top_actor_on_cell(cell).unwrap(), Some(boss));
    let death = canonical_lethal(
        &mut runtime,
        boss,
        "oteryn:creature.goshnar_s_megalomania_blue",
    );
    assert!(runtime.source_creature_is_registered(boss).unwrap());
    assert!(runtime.retains_corpse_projection(boss));
    assert_eq!(
        runtime.current_top_actor_on_cell(cell).unwrap(),
        Some(boss),
        "registered HP0 source creature must occlude the lower Player before its queued onDeath"
    );
    runtime.complete_source_creature_death(boss, death).unwrap();
    assert!(!runtime.source_creature_is_registered(boss).unwrap());
    assert!(runtime.retains_corpse_projection(boss));
    assert_eq!(
        runtime.current_top_actor_on_cell(cell).unwrap(),
        Some(player),
        "completed source corpse-pinned slot is no longer a physical creature occupant"
    );
}
