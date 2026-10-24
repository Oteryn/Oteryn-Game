#[test]
fn creature_damage_preserves_actual_logical_stance_reservation() {
    use crate::domain::{CharacterId, CharacterRevision};
    use crate::durability::character_progression::CurrentCharacterGameplayFence;
    use crate::foundation::{ConnectionGeneration, RuntimeScopeRefV1};
    use crate::spell::stance_execution::PreparedStance;
    let (mut runtime, actor, session) = runtime_with_player(0x35);
    let mut states = ChannelSpellStates::default();
    states
        .initialize(
            &runtime,
            actor,
            session,
            CharacterCastFacts {
                vocation: Vocation::EliteKnight,
                level: 50,
                magic_level: 0,
                max_health: 500,
                max_mana: 600,
                max_soul: 100,
            },
            (0, 0),
            now(0),
        )
        .expect("state");
    let document: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../tools/content-schema/spell-authoring/samples/native-spell-profiles.json"
    ))
    .expect("profiles");
    let profile = document["profiles"]
        .as_array()
        .expect("profiles")
        .iter()
        .find(|r| r["name"] == "Protector")
        .expect("Protector");
    let mut spell = oteryn_gameplay_content::content::spell_data::authoring::spell_from_bundle(
        &serde_json::json!({"spell":profile["spell"]}),
        &profile["dependencies"],
    )
    .expect("source stance");
    // Header-only unit fixture: no commercial proof or durable receipt is fabricated.
    spell.premium = false;
    let state = states.get_mut(&runtime, actor, session).expect("state");
    let operational = crate::spell::OperationalCastFacts {
        caster_position: crate::spell::chain::TilePosition {
            x: 0,
            y: 0,
            floor: 7,
        },
        target_position: None,
        target: None,
        line_of_sight_clear: None,
        direction_available: false,
        wheel_unlocked: None,
        in_protection_zone: false,
        target_tile_solid: None,
        target_tile_creature: None,
    };
    let paid = crate::spell::cast::prepare_native_owner_cast(
        state,
        &spell,
        &operational,
        crate::spell::native::Facts::Stance {
            active: None,
            vocation: Vocation::EliteKnight,
        },
        now(0),
        &mut |_, _| 0,
    )
    .expect("prepare real stance plan");
    let binding = runtime.binding();
    let prepared = PreparedStance::new(
        state,
        paid,
        &spell,
        actor,
        session,
        1,
        NonZeroU32::new(1).expect("spell"),
        CurrentCharacterGameplayFence {
            character_id: CharacterId::from_bytes(uuid_v7(0x36)).expect("character"),
            game_session_id: session,
            connection_generation: ConnectionGeneration::new(1).expect("connection"),
            character_lease_generation: 1,
            runtime_scope: RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id()),
            scope_ownership_generation: binding.scope_generation(),
            expected_character_revision: CharacterRevision::new(1).expect("revision"),
        },
        "content:1".into(),
        "policy:1".into(),
        now(0),
    )
    .expect("retained stance");
    state
        .reserve_stance(prepared.clone())
        .expect("actual logical reservation");
    assert!(!runtime.actor_spell_reserved(actor));
    assert!(states.has_pending_spell_commit(actor, session));
    assert!(!states.spell_idle(&runtime, actor, session));
    let before = states
        .get(&runtime, actor, session)
        .expect("before")
        .clone();
    assert_eq!(
        states.apply_creature_damage(
            &mut runtime,
            actor,
            session,
            8,
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0),
        ),
        None
    );
    assert_eq!(states.get(&runtime, actor, session), Some(&before));
    assert!(
        states
            .get_mut(&runtime, actor, session)
            .expect("state")
            .cancel_stance(&prepared)
    );
    let hit = states
        .apply_creature_damage(
            &mut runtime,
            actor,
            session,
            8,
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0),
        )
        .expect("unreserved positive control");
    assert_eq!(
        (
            hit.damage.applied,
            hit.damage.health_after,
            hit.vitals_revision,
            hit.death
        ),
        (8, 492, 2, None)
    );
}

#[test]
fn creature_damage_preserves_both_caster_and_target_physical_reservations() {
    use crate::foundation::runtime_actor_spell_types::{
        OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellAnchor, SpellOccurrenceBinding,
    };
    use crate::foundation::{CharacterId, CommandId, CommandRef, MovementLocalPosition};

    let (mut runtime, caster, caster_session) = runtime_with_player(0x32);
    let target_session = GameSessionId::decode(&uuid_v7(0x33)).expect("target session");
    let target = runtime
        .reserve_fresh_session(target_session)
        .expect("reserve target");
    let target = runtime.commit_fresh_session(target).expect("commit target");
    for (actor, x) in [(caster, 10), (target, 11)] {
        runtime
            .initialize_movement_test_position(actor, MovementLocalPosition { x, y: 10, floor: 7 })
            .expect("position");
    }
    let mut states = ChannelSpellStates::default();
    for (actor, session) in [(caster, caster_session), (target, target_session)] {
        states
            .initialize(&runtime, actor, session, FACTS, (0, 0), now(0))
            .expect("vitals");
    }
    let batch = OwnerCombatBatch {
        caster,
        attacker: CharacterId::decode(&uuid_v7(0x34)).expect("character"),
        current_lease_generation: 1,
        command: CommandRef::new(caster_session, CommandId::new(1).expect("command")),
        occurrence: SpellOccurrenceBinding {
            id: "spell:retained".into(),
            revisions: ["rules:1", "content:1", "world:1", "formula:1", "sim:1"].map(str::to_owned),
        },
        binding: b"retained-owner-reservation".to_vec(),
        anchor: Some(SpellAnchor {
            expected_revision: 1,
            next_revision: 2,
            paid_mana: 0,
            paid_soul: 0,
            cooldown_deadlines: vec![],
        }),
        now_us: None,
        now_ms: 0,
        effects: vec![OwnerCombatEffect {
            target,
            sub_ordinal: 0,
            change: OwnerCombatChange::ManaShield(
                crate::foundation::runtime_actor_spell_types::ManaShieldState {
                    capacity: 50,
                    expires_ms: 1_000,
                },
            ),
        }],
        deferred: None,
    };
    let mut staged = runtime.stage_spell_batch(&batch).expect("stage real batch");
    runtime
        .reserve_spell_batch(&mut staged)
        .expect("reserve actual slots");
    for (actor, session) in [(caster, caster_session), (target, target_session)] {
        assert!(runtime.actor_spell_reserved(actor));
        // The target has no own pending intent: another caster's reservation
        // independently prevents a hit from invalidating its exact before-state.
        assert!(!states.has_pending_spell_commit(actor, session));
        let before = states.get(&runtime, actor, session).expect("state").clone();
        assert_eq!(
            states.apply_creature_damage(
                &mut runtime,
                actor,
                session,
                8,
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0),
            ),
            None
        );
        assert_eq!(states.get(&runtime, actor, session), Some(&before));
    }
    runtime
        .validate_staged_spell_batch(&staged)
        .expect("retained physical proof unchanged");
}
