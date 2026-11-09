// Actual retained native health policy and canonical combined-event HP owner.
use super::*;
fn hatred_fixture() -> (
    ChannelRuntimeV1,
    ExactActorRef,
    GameSessionId,
    ExactActorRef,
    CommandRef,
) {
    let world = WorldId::decode(&id(1)).expect("world");
    let mut runtime = ChannelRuntimeV1::from_committed_scope_assignment(
        RuntimeScopeRefV1::instance(world, id(21)).expect("private Instance"),
        NodeId::decode(&id(3)).expect("node"),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        8,
        ChannelContentPin::test(world),
    )
    .expect("actual private carrier");
    runtime
        .install_companion_policies(policies())
        .expect("full retained native health metadata");
    let session = GameSessionId::decode(&id(4)).expect("session");
    let reserved = runtime.reserve_fresh_session(session).expect("reserve");
    let player = runtime.commit_fresh_session(reserved).expect("commit");
    let at = MovementLocalPosition {
        x: 33743,
        y: 31599,
        floor: 14,
    };
    runtime
        .initialize_pinned_test_position(player, at)
        .expect("actual Player position");
    runtime
        .bind_attacker_lease(
            player,
            session,
            CharacterLease::new(CharacterId::decode(&id(5)).expect("Character"), 1).expect("lease"),
        )
        .expect("actual bound Player source");
    let policy = runtime
        .companion_policy("oteryn:creature.goshnar_s_hatred")
        .expect("actual native Hatred");
    let boss = runtime
        .admit_pinned_monster_lab_creature(at, &policy.definition_key, policy.maximum_health)
        .expect("actual native Hatred actor");
    runtime
        .install_creature_policy(boss, &policy.definition_key)
        .expect("native passive seal");
    assert!(
        runtime
            .soul_war_health_callback(boss)
            .expect("current policy")
            .is_some_and(|policy| policy.is_empowered_boss()),
        "fixture must reach exact passive source policy"
    );
    let snapshot = runtime.companion_snapshot(boss).expect("current boss");
    runtime
        .change_soul_war_hatred_percent(
            &snapshot,
            0,
            crate::carrier::soul_war_hatred::HatredEmpowerment::Add(10),
        )
        .expect("current source percent");
    let command = CommandRef::new(session, CommandId::new(1).expect("command"));
    (runtime, player, session, boss, command)
}
#[test]
fn srx3_hatred_native_player_health_event_adds_defense_once_and_replay_keeps_it() {
    let (mut runtime, player, _, boss, command) = hatred_fixture();
    let before = runtime.companion_snapshot(boss).expect("before");
    let defense = runtime
        .effective_creature_defense(&before)
        .expect("canonical defense");
    let hit = OwnerDamageCommand {
        target: b"oteryn:creature.goshnar_s_hatred",
        occurrence: &[],
        binding: b"hatred.actual.native.combined-health",
        damage: 101,
    };
    let result = runtime
        .borrow_exact_actor_commit_at(200)
        .commit_damage_for_bound_attacker(boss, player, command, 0, hit)
        .expect("actual combined Player health event");
    assert!(result.applied);
    let after = runtime.companion_snapshot(boss).expect("after");
    assert_eq!(
        runtime
            .effective_creature_defense(&after)
            .expect("after defense"),
        defense + 10,
        "one combined callback adds the source percentage once, not per component"
    );
    assert_eq!(
        after.state.policy.defense, before.state.policy.defense,
        "source mutable defense must not rewrite immutable policy"
    );
    assert!(
        !runtime
            .borrow_exact_actor_commit_at(200)
            .commit_damage_for_bound_attacker(boss, player, command, 0, hit)
            .expect("same canonical receipt")
            .applied
    );
    let replay = runtime.companion_snapshot(boss).expect("replayed actor");
    assert_eq!(
        runtime
            .effective_creature_defense(&replay)
            .expect("replayed defense"),
        defense + 10
    );
}

#[test]
fn srx3_hatred_native_full_health_player_heal_stages_defense_before_clamp() {
    let (mut runtime, player, session, boss, _) = hatred_fixture();
    let before = runtime
        .companion_snapshot(boss)
        .expect("current full-health boss");
    let defense = runtime
        .effective_creature_defense(&before)
        .expect("canonical defense");
    let batch = runtime_actor_spell_types::OwnerCombatBatch {
        caster: player,
        attacker: CharacterId::decode(&id(5)).expect("actual Character"),
        current_lease_generation: 1,
        command: CommandRef::new(session, CommandId::new(2).expect("command")),
        occurrence: runtime_actor_spell_types::SpellOccurrenceBinding {
            id: "source.hatred.actual.heal".into(),
            revisions: ["rules:1", "content:1", "world:1", "formula:1", "sim:1"].map(str::to_owned),
        },
        binding: b"source.hatred.actual.heal".to_vec(),
        anchor: None,
        now_us: None,
        now_ms: 200,
        effects: vec![runtime_actor_spell_types::OwnerCombatEffect {
            target: boss,
            sub_ordinal: 0,
            change: runtime_actor_spell_types::OwnerCombatChange::Heal {
                target_atom: "oteryn:creature.goshnar_s_hatred".into(),
                magnitude: 1000,
            },
        }],
        deferred: None,
    };
    let staged = runtime
        .stage_spell_batch(&batch)
        .expect("canonical full-health heal staging");
    assert_eq!(
        runtime
            .effective_creature_defense(&before)
            .expect("unpublished defense"),
        defense,
        "staging cannot publish native health-callback defense"
    );
    runtime
        .commit_spell_batch(staged)
        .expect("one canonical heal commit");
    let after = runtime.companion_snapshot(boss).expect("same live boss");
    assert_eq!(after.health, before.health);
    assert_eq!(
        runtime
            .effective_creature_defense(&after)
            .expect("published defense"),
        defense + 10,
        "source health callback runs before the full-health clamp"
    );
    let replay_stage = runtime.stage_spell_batch(&batch).expect("replay staging");
    assert!(
        !runtime
            .commit_spell_batch(replay_stage)
            .expect("same immutable heal receipt")
            .applied
    );
    let replay = runtime.companion_snapshot(boss).expect("replayed boss");
    assert_eq!(
        runtime
            .effective_creature_defense(&replay)
            .expect("replay defense"),
        defense + 10
    );
}
