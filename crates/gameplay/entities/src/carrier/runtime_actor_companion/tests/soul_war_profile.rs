use super::*;

#[test]
fn srx3_megalomania_source_form_change_rebinds_actual_native_health_callback() {
    let world = WorldId::decode(&id(1)).expect("actual test World");
    let scope = RuntimeScopeRefV1::instance(world, id(2)).expect("private accepted scope shape");
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
    .expect("actual current native owner");
    runtime
        .install_companion_policies(crate::carrier::soul_war_mirror_runtime::tests::policies())
        .expect("full pinned native bodies");
    const PURPLE: &str = "oteryn:creature.goshnar_s_megalomania_purple";
    const GREEN: &str = "oteryn:creature.goshnar_s_megalomania_green";
    let policy = runtime
        .companion_policy(PURPLE)
        .expect("actual pinned Purple");
    let boss = ExactActorRef(
        runtime
            .admit_encounter_fixture_actor(
                ActorState(1),
                PURPLE,
                policy.maximum_health,
                Some(LocalPosition {
                    x: 33710,
                    y: 31634,
                    floor: 14,
                }),
            )
            .expect("real native boss slot"),
    );
    runtime
        .install_creature_policy(boss, PURPLE)
        .expect("actual bounded source callback installation");
    assert!(
        runtime
            .soul_war_health_callback(boss)
            .expect("current source qualifier")
            .is_some_and(|callback| callback.is_empowered_boss())
    );
    runtime
        .set_test_creature_health(boss, policy.maximum_health - 100)
        .unwrap();
    let before = runtime.companion_snapshot(boss).unwrap();
    let prepared = runtime
        .prepare_creature_profile_transformation(&before, GREEN)
        .expect("existing canonical type preparation");
    let after = runtime
        .commit_creature_profile_transformation(prepared)
        .expect("existing canonical type publication");
    assert_eq!(after.actor, boss);
    assert_eq!(
        after.health, before.health,
        "setType(false) preserves current HP"
    );
    assert_eq!(
        after.maximum_health, before.maximum_health,
        "source does not reset the current maximum"
    );
    assert_eq!(after.state.policy.definition_key, GREEN);
    assert!(
        runtime
            .soul_war_health_callback(boss)
            .is_ok_and(|callback| callback.is_some_and(|callback| callback.matches(
                runtime.content_pin().server_artifact_digest(),
                GREEN,
                "definition-r1"
            ))),
        "actual successor form must carry its own sealed native health callback, never the stale Purple token"
    );
}

#[test]
fn srx3_megalomania_queued_source_type_change_preserves_committed_lethal_receipt() {
    let world = WorldId::decode(&id(1)).expect("actual test World");
    let scope = RuntimeScopeRefV1::instance(world, id(2)).expect("private accepted scope shape");
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
    .expect("actual current native owner");
    runtime
        .install_companion_policies(crate::carrier::soul_war_mirror_runtime::tests::policies())
        .expect("full pinned native bodies");
    const BLUE: &str = "oteryn:creature.goshnar_s_megalomania_blue";
    const PURPLE: &str = "oteryn:creature.goshnar_s_megalomania_purple";
    let policy = runtime
        .companion_policy(BLUE)
        .expect("actual pinned vulnerable Blue");
    let boss = ExactActorRef(
        runtime
            .admit_encounter_fixture_actor(
                ActorState(1),
                BLUE,
                policy.maximum_health,
                Some(LocalPosition {
                    x: 33710,
                    y: 31634,
                    floor: 14,
                }),
            )
            .expect("real native boss slot"),
    );
    runtime
        .install_creature_policy(boss, BLUE)
        .expect("actual bounded source callback installation");
    assert!(
        runtime
            .soul_war_health_callback(boss)
            .expect("current source qualifier")
            .is_some_and(|callback| callback.is_empowered_boss())
    );

    // The source seven-second return may run after a vulnerable Blue hit
    // reaches zero but before its queued onDeath task. No Purple immunity
    // override is introduced or used as a combat fixture.
    runtime.set_test_creature_health(boss, 1).unwrap();
    let lethal = runtime
        .borrow_exact_actor_commit_at(200)
        .commit_damage(
            boss,
            OwnerDamageCommand {
                target: BLUE.as_bytes(),
                occurrence: b"source-queued-death",
                binding: b"source-queued-death\0megalomania",
                damage: 1,
            },
        )
        .expect("actual committed native HP owner");
    assert_eq!(lethal.health_after, 0);
    let original = runtime
        .borrow_combat_death()
        .committed_lethal_receipt(boss)
        .expect("sealed original HP receipt");
    let before = runtime
        .companion_snapshot_including_dead(boss)
        .expect("source lookup still sees registered zero-health creature");
    let prepared = runtime
        .prepare_soul_war_megalomania_profile_change(&before, PURPLE)
        .expect("source setType(false) accepts the still-registered dead native boss");
    let after = runtime
        .commit_creature_profile_transformation(prepared)
        .expect("one canonical source type publication");
    assert_eq!(after.health, 0);
    assert_eq!(after.maximum_health, before.maximum_health);
    assert_eq!(after.actor, boss);
    assert_eq!(after.state.policy.definition_key, PURPLE);
    runtime
        .borrow_combat_death()
        .project_committed_lethal(original)
        .expect("original HP death identity survives actual drop-time source form");
    assert!(runtime.retains_corpse_projection(boss));
}
#[test]
fn srx3_megalomania_retained_aspect_respawn_is_source_family_bound_and_nonforced() {
    let world = WorldId::decode(&id(1)).unwrap();
    let scope = RuntimeScopeRefV1::instance(world, id(2)).unwrap();
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
    const BOSS: &str = "oteryn:creature.goshnar_s_megalomania_blue";
    const ASPECT: &str = "oteryn:creature.aspect_of_power";
    let policy = runtime.companion_policy(BOSS).unwrap();
    let boss = ExactActorRef(
        runtime
            .admit_encounter_fixture_actor(
                ActorState(1),
                BOSS,
                policy.maximum_health,
                Some(LocalPosition {
                    x: 33710,
                    y: 31634,
                    floor: 14,
                }),
            )
            .unwrap(),
    );
    runtime.install_creature_policy(boss, BOSS).unwrap();
    assert!(
        runtime
            .soul_war_health_callback(boss)
            .unwrap()
            .is_some_and(|p| p.is_empowered_boss())
    );
    let retained = runtime.qualify_soul_war_spawn_source(boss).unwrap();
    let center = runtime.read_actor_position(boss).unwrap().position();
    let at = MovementLocalPosition {
        x: center.x + 1,
        ..center
    };
    let actor = runtime
        .spawn_soul_war_child_from_source(&retained, ASPECT, at, |_, _| Some(true))
        .expect("actual pinned Megalomania onDeath respawns only its source Aspect family");
    assert_eq!(
        runtime
            .companion_snapshot(actor)
            .unwrap()
            .state
            .policy
            .definition_key,
        ASPECT
    );
    assert!(
        runtime
            .spawn_soul_war_child_from_source(&retained, ASPECT, center, |_, _| Some(true))
            .is_err(),
        "default Game.createMonster callback cannot force through the current boss occupancy"
    );
    runtime.remove_encounter_actor(boss).unwrap();
    let successor = runtime
        .spawn_soul_war_child_from_source(&retained, ASPECT, center, |_, _| Some(true))
        .expect(
            "source delayed Aspect respawn survives original boss retirement in the same Instance",
        );
    assert_eq!(
        runtime
            .companion_snapshot(successor)
            .unwrap()
            .state
            .policy
            .definition_key,
        ASPECT
    );
    let unrelated_free = MovementLocalPosition {
        x: center.x + 2,
        ..center
    };
    assert!(
        runtime
            .spawn_soul_war_child_from_source(
                &retained,
                "oteryn:creature.soul_sphere",
                unrelated_free,
                |_, _| Some(true)
            )
            .is_err(),
        "Megalomania source is not a generic native creature birth grant"
    );
}
