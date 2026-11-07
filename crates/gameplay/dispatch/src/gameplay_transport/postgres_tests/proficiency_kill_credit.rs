//! P08: real owner damage, death capture, dispatch settlement and a training checkpoint.
#![allow(
    clippy::expect_used,
    reason = "PostgreSQL fixtures construct valid content and actors"
)]
use crate::bestiary_postgres_harness::{
    self as fixture, CHANNEL, Harness, TestResult, WORLD, debug, id,
};
use crate::foundation::{
    ChannelId, ChannelRuntimeV1, CommandId, CommandRef, MovementLocalPosition, WorldId,
};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, FirstEntryOutcome, SessionContinuity},
    kill_reward::{DrainOutcome, KillSink, TakeOutcome},
};
use oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use std::time::Duration;
use tokio::sync::Mutex;

#[derive(Clone, Copy)]
enum Scenario {
    CheckpointReplay,
    FenceReadFailure,
    GonePrincipal,
}

#[test]
fn proficiency_kill_credit_settlement_checkpoint_and_replay() -> TestResult {
    run(Scenario::CheckpointReplay, "proficiency_kill_credit")
}

#[test]
fn proficiency_kill_credit_fence_read_failure_is_retryable() -> TestResult {
    run(Scenario::FenceReadFailure, "proficiency_fence_failure")
}

#[test]
fn proficiency_kill_credit_surviving_contributor_when_xp_principal_is_gone() -> TestResult {
    run(Scenario::GonePrincipal, "proficiency_gone_principal")
}

fn run(case: Scenario, name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .name(name.into())
        .stack_size(64 << 20)
        .spawn(move || {
            let runtime = fixture::runtime().map_err(|error| error.to_string())?;
            let result: TestResult = runtime.block_on(async {
                let admin = fixture::configured_admin().ok_or("PostgreSQL route required")?;
                let h = Harness::create(admin, name, false).await?;
                let result = scenario(&h, case).await;
                h.cleanup().await?;
                result
            });
            result.map_err(|error: Box<dyn std::error::Error>| error.to_string())
        })?
        .join()
        .map_err(|_| "proficiency scenario thread panicked")?
        .map_err(Into::into)
}

async fn scenario(h: &Harness, case: Scenario) -> TestResult {
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let first =
        crate::world_party_runtime_postgres_cases::committed_fresh_player(h, 60, 63, 70, now)
            .await?;
    let second =
        crate::world_party_runtime_postgres_cases::committed_fresh_player(h, 61, 64, 71, now)
            .await?;
    let third =
        crate::world_party_runtime_postgres_cases::committed_fresh_player(h, 62, 65, 72, now)
            .await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let recovery = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
    let channel = ChannelId::decode(&id(CHANNEL)).map_err(debug)?;
    let (controller, pin) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(
            world,
            Some(include_bytes!(
                "../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
            )),
        );
    let active = controller.active().expect("active");
    let native = active.native_gameplay().expect("native");
    let room = pin.qualified_room().clone();
    let position = MovementLocalPosition {
        x: 2,
        y: 11,
        floor: -7,
    };
    let pin = ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        pin.frame_binding().digest(),
        room.map_revision_digest(),
        (position.x, position.y, position.floor),
    );
    sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,NULL,$4,$5,$6)")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(pin.activation_sequence().to_string())
        .bind(pin.server_artifact_digest().as_slice()).bind(pin.client_artifact_digest().as_slice()).bind(pin.frame_binding_digest().as_slice())
        .execute(&h.pool).await?;
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        channel,
        h.node.fact().node_id(),
        h.node.fact().registration_revision(),
        1,
        1,
        "runtime-scope-assignment:1",
        5,
        pin.clone(),
    )
    .map_err(debug)?;
    native
        .install_companion_policies(&mut runtime)
        .map_err(debug)?;
    runtime
        .realize_native_qualification_spawn(
            pin.clone(),
            "oteryn:creature.rat",
            MovementLocalPosition {
                x: 1,
                y: 11,
                floor: -7,
            },
        )
        .map_err(debug)?;
    let mut players = Vec::new();
    for (index, current) in [&first, &second, &third].into_iter().enumerate() {
        let session = current.current_game_session_id();
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .bind_attacker_lease(actor, session, current.current_character_lease())
            .map_err(debug)?;
        runtime
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(current.current_character_lease().character_id().as_bytes().as_slice()).execute(&mut *tx).await?;
        let item = id(110 + u8::try_from(index)?);
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i3264','definition-r1',1,1,encode($1,'hex')::uuid)").bind(item.as_slice()).bind(world.as_bytes().as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_character_equipment_slots(character_id,world_id,slot,item_instance_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,6,encode($3,'hex')::uuid,encode($3,'hex')::uuid)").bind(current.current_character_lease().character_id().as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(item.as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        players.push((actor, session));
    }
    let creature = runtime
        .visible_entities()
        .creatures
        .first()
        .copied()
        .ok_or("rat")?;
    let rat = oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
        identity: creature.actor.placement_identity(),
        generation: creature.generation,
    };
    let door_content = crate::world_runtime::ReferenceContentGeneration::from_content(room.door())
        .map_err(debug)?;
    let door_fence = crate::world_runtime::ScopeContentGenerationFence::for_test(
        first.current_runtime_scope(),
        first.current_scope_generation(),
        door_content,
    );
    let door = Mutex::new(
        crate::world_runtime::bind_native_entry_door(
            room.door(),
            &door_fence,
            first.current_runtime_scope(),
            first.current_scope_generation(),
        )
        .map_err(debug)?,
    );
    let descriptor=crate::native_admission_source::descriptor::ProducerDescriptor::new("platform-test".into(),("127.0.0.1".into(),1),"source.test".into(),"source.test".into(),
        vec![rustls::pki_types::CertificateDer::from(include_bytes!("../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/ca.der").to_vec())],
        vec![rustls::pki_types::CertificateDer::from(include_bytes!("../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/client.der").to_vec())],
        rustls::pki_types::PrivateKeyDer::try_from(include_bytes!("../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/client-key.der").to_vec())?).map_err(debug)?;
    let evidence = crate::gameplay_transport::fresh_evidence::FreshEvidenceSource::new(descriptor);
    let spells = native.spell_book();
    let achievements =
        crate::achievement_catalogue::AchievementCatalogue::embedded().map_err(debug)?;
    let charms =
        crate::content::charm_source::CanonicalCharmCatalogue::embedded().map_err(debug)?;
    let runtime = Mutex::new(runtime);
    let mut owner = crate::gameplay_transport::composed_test_owner::ComposedTestOwner {
        root: &h.root,
        character: &recovery,
        holder: &h.node,
        evidence: &evidence,
        world_id: world,
        channel_id: channel,
        required_capabilities:
            crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
                &[],
                first.current_runtime_scope(),
                1,
                "policy-1",
                crate::gameplay_transport::world_required_capabilities::offered_for_scope(false, None),
            )
            .map_err(debug)?,
        runtime: &runtime,
        door: &door,
        spells,
        achievements: &achievements,
        imported_charms: &charms,
        room: &room,
    }
    .build();
    owner.active_generation = Some(active);
    owner.clock_origin = std::time::Instant::now()
        .checked_sub(Duration::from_secs(400))
        .expect("clock");
    owner.proficiency_catalogue =
        crate::gameplay_transport::weapon_proficiency::ProductionProficiencyCatalogue::embedded();
    for (index, (actor, session)) in players.iter().copied().enumerate() {
        owner
            .admit_progression_session(&mut AdmittedSession {
                game_session_id: session,
                world_id: world,
                channel_id: channel,
                runtime_actor: Some(actor),
                first_entry: FirstEntryOutcome::NotApplicable,
                controller: None,
                continuity: SessionContinuity::FRESH,
                item_fence: None,
            })
            .await;
        let (facts, build) =
            initialize_build(&owner, session, "knight", 90 + u8::try_from(index)?).await?;
        {
            let runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            assert!(
                states
                    .initialize(&runtime, actor, session, facts, (0, 0), owner.owner_now())
                    .is_some()
            );
            assert!(states.load_owned_build(&runtime, actor, session, &build));
            if index == 0 {
                assert_eq!(
                    owner.states.attack.lock().await.set_target(
                        &runtime,
                        Some(&room),
                        actor,
                        session,
                        owner.owner_now(),
                        Some(rat),
                        1
                    ),
                    oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
                );
            }
        }
        if index == 0 {
            owner.refresh_attack_weapon(actor, session).await;
        }
        // Equipment owner admission must publish the weapon independently of a held
        // attack target. Invalidating the swing cache must not erase that projection.
        let effects = owner
            .read_admission_effects(session)
            .await
            .map_err(|_| "effects")?;
        {
            let runtime = owner.runtime.lock().await;
            owner
                .install_admission_suppressions(&runtime, actor, session, effects)
                .await
                .ok_or("install equipment")?;
        }
        owner
            .states
            .attack
            .lock()
            .await
            .forget_weapon(actor, session);
    }
    // The first contributor changes weapons after the cached attack read. Publish through
    // the same equipment-owner installation used after a committed equipment move.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_item_instances SET definition_production_key='oteryn:item.tibia.i3289' WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(110).as_slice()).execute(&mut *tx).await?;
    sqlx::query("UPDATE game_character_equipment_state SET revision=revision+1 WHERE character_id=encode($1,'hex')::uuid")
        .bind(first.current_character_lease().character_id().as_bytes().as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    let effects = owner
        .read_admission_effects(players[0].1)
        .await
        .map_err(|_| "changed equipment")?;
    {
        let runtime = owner.runtime.lock().await;
        owner
            .install_admission_suppressions(&runtime, players[0].0, players[0].1, effects)
            .await
            .ok_or("install changed weapon")?;
    }
    if matches!(case, Scenario::GonePrincipal) {
        let queue = owner.states.attack.lock().await.kills().clone();
        {
            let mut runtime = owner.runtime.lock().await;
            damage(&mut runtime, creature.actor, players[0], 15, 399_999)?;
            runtime
                .remove_terminal_session(players[0].1, players[0].0)
                .map_err(debug)?;
            damage(&mut runtime, creature.actor, players[1], 100, 400_000)?;
            record(
                &mut runtime,
                &queue,
                native.reward_table(),
                creature.actor,
                crate::combat::CapturedRewardPrincipal {
                    character: second.current_character_lease().character_id(),
                    lease_generation: second.current_character_lease().generation(),
                    session: players[1].1,
                    actor: players[1].0,
                },
            );
        }
        assert_eq!(
            owner.drain_kill_rewards(players[1].1).await,
            DrainOutcome::Drained
        );
        assert_eq!(
            owner
                .states
                .proficiencies
                .lock()
                .map_err(debug)?
                .get(&players[1].1)
                .and_then(|state| state.track("oteryn:item.tibia.i3264"))
                .map(|track| track.progress),
            Some(30),
            "B earns independently of gone XP principal A"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_xp_receipts")
                .fetch_one(&h.pool)
                .await?,
            0,
            "XP must not be reassigned to B"
        );
        assert_eq!(
            owner
                .save_weapon_proficiency(players[1].0, players[1].1, true)
                .await,
            crate::gameplay_transport::actor_spell::TrainingSave::Saved
        );
        return Ok(());
    }
    // The oldest contributor is one ms outside the window; the other is exactly on it.
    let queue = owner.states.attack.lock().await.kills().clone();
    {
        let mut runtime = owner.runtime.lock().await;
        let mut scope = crate::foundation::scoped_fence_fixture(
            first.current_runtime_scope(),
            first.current_scope_generation(),
        );
        let ordinal = scope
            .accept_input(first.current_scope_generation())
            .map_err(debug)?;
        let spec = crate::foundation::NativeSummonAdmissionSpec::qualified(
            "oteryn:creature.rat",
            "oteryn:creature.rat",
            20,
            0,
            1,
            1,
            runtime.content_pin().server_artifact_digest(),
        )
        .map_err(debug)?;
        let summon = runtime
            .admit_native_summon(
                &scope,
                scope.stamp(ordinal),
                creature.actor,
                &spec,
                MovementLocalPosition {
                    x: 1,
                    y: 10,
                    floor: -7,
                },
            )
            .map_err(debug)?;
        runtime
            .borrow_exact_actor_commit_at(399_999)
            .commit_damage_for_bound_attacker(
                summon,
                players[0].0,
                CommandRef::new(players[0].1, CommandId::new(1).map_err(debug)?),
                0,
                crate::foundation::OwnerDamageCommand {
                    target: b"oteryn:creature.rat",
                    occurrence: b"summon-kill",
                    binding: b"summon-kill",
                    damage: 100,
                },
            )
            .map_err(debug)?;
        crate::gameplay_transport::kill_reward::record_creature_kill(
            &mut runtime,
            KillSink {
                prey: None,
                activity: &[],
                queue: &queue,
                rewards: native.reward_table(),
                charms: None,
            },
            summon,
            crate::combat::CapturedRewardPrincipal {
                character: first.current_character_lease().character_id(),
                lease_generation: first.current_character_lease().generation(),
                session: players[0].1,
                actor: players[0].0,
            },
            399_999,
            None,
        );
        assert!(
            !queue.has_session(players[0].1),
            "a summon death queues no reward or proficiency credit"
        );
    }
    {
        let mut runtime = owner.runtime.lock().await;
        for (index, at, damage) in [(2, 99_999, 1), (1, 100_000, 1), (0, 400_000, 100)] {
            let (actor, session) = players[index];
            runtime
                .borrow_exact_actor_commit_at(at)
                .commit_damage_for_bound_attacker(
                    creature.actor,
                    actor,
                    CommandRef::new(session, CommandId::new(2).map_err(debug)?),
                    0,
                    crate::foundation::OwnerDamageCommand {
                        target: b"oteryn:creature.rat",
                        occurrence: session.as_bytes(),
                        binding: b"proficiency-kill-credit",
                        damage,
                    },
                )
                .map_err(debug)?;
        }
        crate::gameplay_transport::kill_reward::record_creature_kill(
            &mut runtime,
            KillSink {
                prey: None,
                activity: &[],
                queue: &queue,
                rewards: native.reward_table(),
                charms: None,
            },
            creature.actor,
            crate::combat::CapturedRewardPrincipal {
                character: first.current_character_lease().character_id(),
                lease_generation: first.current_character_lease().generation(),
                session: players[0].1,
                actor: players[0].0,
            },
            400_000,
            Some(oteryn_simulation_determinism::SemanticTimeMicros::from_micros(400_000_000)),
        );
    }
    let replay = match queue.take(players[0].1, first.current_character_lease().generation()) {
        TakeOutcome::Taken(held) => held.entry().cloned().ok_or("kill entry")?,
        _ => return Err("real kill was not queued".into()),
    };
    // A contributor's actor-end hook must settle the other principal's queued kill before
    // saving; otherwise that contributor can log out before receiving its proficiency credit.
    assert!(matches!(
        owner
            .save_spell_training(players[1].0, players[1].1, true)
            .await,
        crate::gameplay_transport::actor_spell::TrainingSave::Saved
            | crate::gameplay_transport::actor_spell::TrainingSave::NotApplicable
    ));
    assert_eq!(
        owner.drain_kill_rewards(players[0].1).await,
        DrainOutcome::Drained
    );
    if matches!(case, Scenario::FenceReadFailure) {
        // Break the actual fence read, then restore it before asserting so cleanup works.
        sqlx::query("ALTER TABLE game_durability_reconnect_sessions RENAME TO proficiency_unavailable_sessions")
            .execute(&h.pool).await?;
        let saved = owner
            .save_weapon_proficiency(players[0].0, players[0].1, true)
            .await;
        sqlx::query("ALTER TABLE proficiency_unavailable_sessions RENAME TO game_durability_reconnect_sessions")
            .execute(&h.pool).await?;
        assert_eq!(
            saved,
            crate::gameplay_transport::actor_spell::TrainingSave::Unknown
        );
        // A successfully read terminal session is an actual fence loss, with the
        // same unsaved points still pending.
        sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=3, current_transport_ref=NULL WHERE game_session_id=encode($1,'hex')::uuid")
            .bind(players[0].1.as_bytes().as_slice()).execute(&h.pool).await?;
        assert_eq!(
            owner
                .save_weapon_proficiency(players[0].0, players[0].1, true)
                .await,
            crate::gameplay_transport::actor_spell::TrainingSave::FencedOut
        );
        sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=2, current_transport_ref=$2 WHERE game_session_id=encode($1,'hex')::uuid")
            .bind(players[0].1.as_bytes().as_slice()).bind(first.current_transport().ok_or("transport")?.to_bytes().as_slice()).execute(&h.pool).await?;
        assert_eq!(
            owner
                .save_weapon_proficiency(players[0].0, players[0].1, true)
                .await,
            crate::gameplay_transport::actor_spell::TrainingSave::Saved
        );
        return Ok(());
    }
    for (index, (actor, session)) in players.iter().copied().enumerate() {
        assert_eq!(
            owner
                .states
                .proficiencies
                .lock()
                .map_err(debug)?
                .get(&session)
                .and_then(|s| s.track(if index == 0 {
                    "oteryn:item.tibia.i3289"
                } else {
                    "oteryn:item.tibia.i3264"
                }))
                .map(|t| t.progress),
            Some(if index < 2 { 30 } else { 0 })
        );
        if index == 0 {
            // Shutdown must include proficiency-only progress in its in-memory prefilter.
            let fence = owner
                .current_quest_fence(session)
                .await
                .map_err(|_| "fence")?
                .ok_or("current fence")?;
            assert!(owner.track_restored(
                session,
                actor,
                &crate::gameplay_transport::last_position::AdmissionPosition::FirstLogin,
                fence,
            ));
            {
                let runtime = owner.runtime.lock().await;
                assert!(
                    !owner
                        .states
                        .spell_states
                        .lock()
                        .await
                        .has_training_to_flush(&runtime, actor, session)
                );
            }
            let mut settled = std::collections::HashSet::new();
            assert_eq!(owner.flush_spell_training(&mut settled).await, 0);
            assert!(settled.contains(&session));
            assert_eq!(
                owner.save_weapon_proficiency(actor, session, true).await,
                crate::gameplay_transport::actor_spell::TrainingSave::NotApplicable
            );
        } else {
            assert_eq!(
                owner.save_weapon_proficiency(actor, session, true).await,
                crate::gameplay_transport::actor_spell::TrainingSave::NotApplicable
            );
        }
        owner
            .states
            .attack
            .lock()
            .await
            .forget_weapon(actor, session);
        owner.refresh_attack_weapon(actor, session).await;
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM game_character_proficiency_receipts WHERE cause='training'"
        )
        .fetch_one(&h.pool)
        .await?,
        2
    );
    let lines: Vec<(String, i64)> = sqlx::query_as(
        "SELECT item_key,progress_after FROM game_character_proficiency_receipt_lines ORDER BY item_key",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(
        lines,
        vec![
            ("oteryn:item.tibia.i3264".to_owned(), 30),
            ("oteryn:item.tibia.i3289".to_owned(), 30)
        ]
    );
    assert_eq!(
        queue.append(replay),
        crate::gameplay_transport::kill_reward::AppendOutcome::Queued
    );
    assert_eq!(
        owner.drain_kill_rewards(players[0].1).await,
        DrainOutcome::Drained
    );
    for (actor, session) in &players {
        assert_eq!(
            owner.save_weapon_proficiency(*actor, *session, true).await,
            crate::gameplay_transport::actor_spell::TrainingSave::NotApplicable
        );
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_proficiency_receipts")
            .fetch_one(&h.pool)
            .await?,
        2
    );
    eprintln!("P08_POSTGRES_ASSERTIONS_COMPLETED training_receipts=2");
    Ok(())
}

async fn initialize_build(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: crate::foundation::GameSessionId,
    vocation: &str,
    seed: u8,
) -> TestResult<(
    crate::spell::cast::CharacterCastFacts,
    crate::durability::character_build::DurableBuildState,
)> {
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|_| "current fence")?
        .ok_or("fence")?;
    let binding = owner
        .progression_binding(session)
        .ok_or("progression binding")?;
    owner
        .root
        .initialize_character_progression(
            owner.character,
            owner.holder,
            fence,
            crate::durability::character_progression::ProgressionInitializationRequest {
                context: binding.context.clone(),
                policy_revision: binding.policy_revision.clone(),
                reward_revision: binding.reward_revision.clone(),
                policy: binding.policy.clone(),
            },
        )
        .await
        .map_err(debug)?;
    let before = owner
        .root
        .read_character_build_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    let formula = crate::spell::mana_training::CompiledTrainingFormula::from_profile(
        include_bytes!(
            "../../../../../../tools/content-schema/native-gameplay/build-training.json"
        ),
        "content-1",
    )
    .map_err(debug)?;
    let after = crate::durability::character_build::convert_vocation(&formula, &before, vocation)
        .map_err(debug)?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    slot.commit_build(
        owner.root,
        owner.character,
        owner.holder,
        fence,
        crate::durability::character_build::BuildChangeRequest {
            occurrence: crate::durability::character_build::BuildOccurrence::from_bytes(id(seed))
                .map_err(debug)?,
            cause: crate::durability::character_build::BuildCause::VocationChoice,
            before,
            after: after.clone(),
            pruned_stance: None,
        },
        &formula,
        None,
    )
    .await
    .map_err(debug)?;
    let crate::gameplay_transport::CastFactsLoad::Ready { facts, .. } =
        crate::gameplay_transport::load_character_cast_facts(
            owner.root,
            owner.character,
            fence.character_id,
        )
        .await
    else {
        return Err("official classed Character facts".into());
    };
    Ok((facts, after))
}

fn damage(
    runtime: &mut ChannelRuntimeV1,
    target: crate::foundation::ExactActorRef,
    player: (
        crate::foundation::ExactActorRef,
        crate::foundation::GameSessionId,
    ),
    amount: i64,
    at: u64,
) -> TestResult {
    runtime
        .borrow_exact_actor_commit_at(at)
        .commit_damage_for_bound_attacker(
            target,
            player.0,
            CommandRef::new(player.1, CommandId::new(3).map_err(debug)?),
            0,
            crate::foundation::OwnerDamageCommand {
                target: b"oteryn:creature.rat",
                occurrence: player.1.as_bytes(),
                binding: b"proficiency-review",
                damage: amount,
            },
        )
        .map_err(debug)?;
    Ok(())
}

fn record(
    runtime: &mut ChannelRuntimeV1,
    queue: &crate::gameplay_transport::kill_reward::KillSettlementQueue,
    rewards: &crate::content::creature_reward::CreatureRewardTable,
    target: crate::foundation::ExactActorRef,
    principal: crate::combat::CapturedRewardPrincipal,
) {
    crate::gameplay_transport::kill_reward::record_creature_kill(
        runtime,
        KillSink {
            prey: None,
            activity: &[],
            queue,
            rewards,
            charms: None,
        },
        target,
        principal,
        400_000,
        Some(oteryn_simulation_determinism::SemanticTimeMicros::from_micros(400_000_000)),
    );
}
