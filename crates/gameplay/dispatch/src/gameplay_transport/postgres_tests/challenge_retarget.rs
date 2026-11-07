//! C17d: staged Challenge reaches the real Channel monster think and expires.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_entities::runtime_actor_spell_types::{
    MonsterAiState, OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
};

fn cell(x: i32) -> MovementLocalPosition {
    MovementLocalPosition {
        x,
        y: 12,
        floor: -7,
    }
}

#[test]
fn staged_challenge_retargets_the_production_monster_cycle_until_expiry() -> TestResult {
    // The full dispatch future runs on its own stack, including under nextest.
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            run(async |admin| {
                let harness =
                    Harness::create_with_admission_seed(admin, "challenge_retarget", false, false)
                        .await?;
                let result = scenario(&harness).await;
                let cleanup = harness.cleanup().await;
                result?;
                cleanup?;
                Ok(())
            })
            .map_err(|error| format!("{error:?}"))
        })?
        .join()
        .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
        .map_err(Into::into)
}

async fn scenario(harness: &Harness) -> TestResult {
    prepare_current_session(harness).await?;
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = harness
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) =
        crate::content::native_gameplay::test_support::activated_loop_monster(world, false);
    let active = controller.active().ok_or("active generation")?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let mut deps = Dependencies::new()?;
    deps.room = pin.qualified_room().clone();
    deps.combat_room = Some(deps.room.clone());
    let channel_pin = ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        active.identity().server_artifact_digest(),
        active.identity().client_artifact_digest(),
        pin.frame_binding().digest(),
        deps.room.map_revision_digest(),
        (2, 11, -7),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        deps.admitted.channel_id,
        crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        8,
        channel_pin,
    )
    .map_err(debug)?;
    native
        .install_companion_policies(&mut runtime)
        .map_err(debug)?;
    let reservation = runtime
        .reserve_fresh_session(deps.admitted.game_session_id)
        .map_err(debug)?;
    deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_source_pinned_lab_player_position(
            deps.actor,
            deps.admitted.game_session_id,
            cell(0),
        )
        .map_err(debug)?;
    let second_session = GameSessionId::decode(&id(51)).map_err(debug)?;
    let reservation = runtime
        .reserve_fresh_session(second_session)
        .map_err(debug)?;
    let second = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_source_pinned_lab_player_position(second, second_session, cell(3))
        .map_err(debug)?;
    let rat_health = u64::try_from(
        runtime
            .companion_policy("oteryn:creature.rat")
            .map_err(debug)?
            .maximum_health,
    )?;
    runtime
        .begin_activation_spawn_sources(vec![(
            vec![(
                (2, 12, -7),
                60_000,
                "oteryn:creature.rat".into(),
                rat_health,
                false,
            )],
            5_000,
        )])
        .map_err(debug)?;
    assert_eq!(
        runtime
            .realize_activation_spawn_window(0)
            .map_err(debug)?
            .realized,
        1
    );
    let rat = runtime.spawn_point_creatures()[0].2.ok_or("spawned rat")?;
    runtime.companion_snapshot(rat).map_err(debug)?;
    deps.runtime = tokio::sync::Mutex::new(runtime);
    let mut owner = deps.owner(harness, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = deps.room.movement_cells();
    // Only B is initially eligible, so the real first think must select B.
    deps.initialize_player(&owner, second, second_session).await;
    owner.clock_origin -= std::time::Duration::from_secs(10);
    owner.drain_monster_melee().await;
    assert_eq!(target(&owner, rat).await?.0, Some(second));
    {
        let runtime = deps.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        states
            .initialize(
                &runtime,
                deps.actor,
                deps.admitted.game_session_id,
                crate::spell::cast::CharacterCastFacts {
                    vocation: crate::spell::Vocation::Knight,
                    ..crate::gameplay_transport::actor_spell::tests::FACTS
                },
                (0, 0),
                owner.owner_now(),
            )
            .ok_or("knight spell state")?;
    }
    let now_ms = owner.owner_now().get() / 1_000;
    let expires_ms = now_ms + 5_000;
    {
        let mut runtime = deps.runtime.lock().await;
        let batch = OwnerCombatBatch {
            caster: deps.actor,
            attacker: crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
            current_lease_generation: 1,
            command: crate::foundation::CommandRef::new(
                deps.admitted.game_session_id,
                crate::foundation::CommandId::new(1).map_err(debug)?,
            ),
            occurrence: SpellOccurrenceBinding {
                id: "spell:challenge-retarget".into(),
                revisions: ["rules:1", "content:1", "world:1", "formula:1", "sim:1"]
                    .map(str::to_owned),
            },
            binding: b"challenge-retarget".to_vec(),
            anchor: None,
            now_us: None,
            now_ms,
            effects: vec![OwnerCombatEffect {
                target: rat,
                sub_ordinal: 0,
                change: OwnerCombatChange::MonsterAi(MonsterAiState {
                    forced_distance: None,
                    challenged_to: Some((deps.actor, expires_ms)),
                }),
            }],
            deferred: None,
        };
        let staged = runtime.stage_spell_batch(&batch).map_err(debug)?;
        assert!(runtime.commit_spell_batch(staged).map_err(debug)?.applied);
    }
    for _ in 0..2 {
        owner.clock_origin -= std::time::Duration::from_secs(1);
        owner.drain_monster_melee().await;
        assert_eq!(
            target(&owner, rat).await?,
            (Some(deps.actor), 1),
            "Challenge selects knight A over current target B"
        );
    }
    owner.clock_origin -= std::time::Duration::from_secs(4);
    owner.drain_monster_melee().await;
    assert_eq!(
        target(&owner, rat).await?.1,
        0,
        "expired challenge releases forced targeting"
    );
    // With A outside perception, normal targeting can select B again.
    {
        let mut runtime = deps.runtime.lock().await;
        for x in (-47..0).rev() {
            let before = runtime.read_actor_position(deps.actor).map_err(debug)?;
            runtime
                .borrow_movement_position()
                .commit_cardinal(before, cell(x))
                .map_err(debug)?;
        }
    }
    owner.clock_origin -= std::time::Duration::from_secs(1);
    owner.drain_monster_melee().await;
    assert_eq!(target(&owner, rat).await?.0, Some(second));
    Ok(())
}

async fn target(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    rat: ExactActorRef,
) -> TestResult<(Option<ExactActorRef>, usize)> {
    let states = owner.states.spell_states.lock().await;
    let state = states
        .monster_melee
        .creature_ai
        .state(rat)
        .ok_or("production rat think")?;
    Ok((state.target(), state.override_count()))
}
