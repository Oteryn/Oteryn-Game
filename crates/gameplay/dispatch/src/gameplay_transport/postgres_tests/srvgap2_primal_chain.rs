//! Production Primal region, qualified template, native cadence/death and durable return.
use super::fixture::{Harness, debug, id};
use super::srx3_activity_source_fixture::{SourceScene, install_scene};
use super::{Dependencies, TestResult};
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::durability::activity_handoff::ActivityReturnReason;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::runtime_actor_spell_types::{
    OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
};
use crate::foundation::{
    CharacterLease, CommandId, CommandRef, MovementLocalPosition, RuntimeScopeRefV1,
};
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::item_fence_of;
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink, record_batch_kills};
use crate::gameplay_transport::native_activity::NativeActivityOutcome;
use oteryn_gameplay_encounters::bosses::hazard_primal::primal_menace_think as rules;
use oteryn_protocol_oteryn::boss_encounter as wire;
use sha2::{Digest, Sha256};
use std::time::{Duration, Instant};

#[test]
fn srvgap2_primal_production_entry_cadence_death_settlement_and_return() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(async |admin| scenario(admin).await)
}

async fn scenario(admin: String) -> TestResult {
    let started = Instant::now();
    super::reset();
    // Qualify the real source scene before creating short-lived admission proofs.
    let (scene, activation_pin) = SourceScene::new_with_activation(
        PinnedActivitySourceRoom::Primal,
        MovementLocalPosition {
            x: 33548,
            y: 32752,
            floor: 14,
        },
    )?;
    let active = scene
        .controller
        .active()
        .ok_or("active production content")?;
    let native = active
        .native_gameplay()
        .ok_or("native production content")?;
    let template = active
        .qualify_primal_activity_template(&scene.room, "content-1")
        .map_err(debug)?;
    let h = Harness::create_with_admission_seed(admin, "srvgap2_primal_chain", true, false).await?;
    let evidence = super::house_transfer_fixtures::RecoverySource::start(&h).await?;
    super::house_transfer_fixtures::prepare_owned_session(&h).await?;
    assert_eq!(
        (
            template.minimum_level(),
            template.minimum_players(),
            template.after_kill_seconds()
        ),
        (500, 1, 60)
    );
    let lever = *template.lever_positions().first().ok_or("source lever")?;
    let deps = Dependencies::with_content_and_capacity(
        scene.pin.clone(),
        scene.room.clone(),
        native.spell_book().clone(),
        64,
    )?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut owner = deps.owner(&h, &authority);
    owner.evidence = &evidence.evidence;
    install_scene(&mut owner, &scene, PinnedActivitySourceRoom::Primal)?;
    // A qualified level-500 participant, with the actual native progression policy.
    let progression = native
        .progression()
        .ok_or("progression")?
        .policy_for("profile-1", "ruleset-1", "content-1")
        .map_err(debug)?;
    let xp = progression
        .thresholds
        .iter()
        .find(|row| row.level == 500)
        .ok_or("level 500 threshold")?
        .minimum_experience
        .get();
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_character_progression_state SET level=500,total_experience=$2,simulation_revision=$3,evidence_revision=$4,declaration_revision=$5,policy_revision=$6,reward_revision=$7 WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).bind(xp).bind(&progression.context.simulation).bind(&progression.context.evidence)
        .bind(&progression.context.declaration).bind(&progression.policy_revision).bind(&progression.reward_revision).execute(&mut *tx).await?;
    tx.commit().await?;
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence = item_fence_of(
        FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(admitted.game_session_id)
            .await?
            .0,
    );
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        BUNDLE_WORLD_OFFERED_CAPABILITIES,
        &[wire::CAPABILITY_BOSS_ENCOUNTER_V1],
    )
    .ok_or("boss capability")?;
    let fence = admitted.item_fence.ok_or("origin fence")?;
    {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .bind_attacker_lease(
                deps.actor,
                admitted.game_session_id,
                CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        runtime
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        assert_eq!(
            runtime
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position(),
            MovementLocalPosition {
                x: lever.x,
                y: lever.y,
                floor: i16::try_from(lever.z)?
            }
        );
        owner
            .states
            .spell_states
            .try_lock()
            .map_err(debug)?
            .initialize(
                &runtime,
                deps.actor,
                admitted.game_session_id,
                crate::spell::cast::CharacterCastFacts {
                    level: 500,
                    max_health: 1_000_000,
                    ..crate::gameplay_transport::actor_spell::tests::FACTS
                },
                (0, 0),
                owner.owner_now(),
            )
            .ok_or("qualified player state")?;
    }
    owner.admit_progression_session(&mut admitted).await;
    assert!(
        owner
            .progression_binding(admitted.game_session_id)
            .is_some(),
        "production progression admission"
    );
    super::item_look_fixtures::seed_main_backpack(&h, &[]).await?;
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1) ON CONFLICT DO NOTHING")
        .bind(id(41).as_slice()).execute(&h.pool).await?;
    owner
        .register_current_runtime()
        .await
        .map_err(|_| "register origin")?;
    owner
        .activity_runtimes
        .remember(admitted)
        .map_err(|_| "remember origin")?;
    // Activation uses the accepted source artifact pin; runtime movement uses
    // scene.pin's actual source-room frame. Admission owns catalogue publication.
    let map_revision = crate::map::overlay::map_revision(
        owner
            .bundle_map()
            .ok_or("actual source bundle map")?
            .overlay
            .base(),
    );
    super::boss_activity_fixtures::issue_content_and_reset_with_runtime_pin(
        &h,
        &activation_pin,
        &scene.pin,
        &map_revision,
    )
    .await?;
    let instance = super::activity_runtime_fixtures::allocate_ready_instance(&h, 88).await?;
    let origin_session = admitted.game_session_id;
    let (_, reply, barrier) = super::boss_activity_fixtures::exchange_command(
        &owner,
        &mut admitted,
        template.boss_key(),
        true,
        wire::COMMAND_TYPE_BOSS_LEVER,
    )
    .await?;
    assert!(matches!(reply, wire::BossActivityReply::Accepted { .. }));
    assert!(barrier.is_some());
    assert_eq!(
        admitted.item_fence.ok_or("Instance fence")?.runtime_scope,
        instance
    );
    assert_ne!(admitted.game_session_id, origin_session);
    let mut destination = owner
        .resolve_runtime_view(admitted)
        .await
        .map_err(|_| "resolve actual destination")?
        .ok_or("destination owner")?;
    assert!(
        destination
            .progression_binding(admitted.game_session_id)
            .is_some(),
        "progression binding follows the real handoff"
    );
    eprintln!("SRVGAP2_PRIMAL_STAGE source_entry_committed");
    let actor = admitted.runtime_actor.ok_or("Instance player")?;
    // Crystal :195-206 varies the count, rather than promising three beasts.
    // Resolve the source oracle against this exact native boss occurrence.
    let (boss, boss_position) = {
        let runtime = destination.runtime.lock().await;
        runtime
            .positioned_melee_census()
            .map_err(debug)?
            .iter()
            .find(|(actor, _, _)| {
                runtime
                    .companion_snapshot(*actor)
                    .is_ok_and(|row| row.state.policy.definition_key == rules::PRIMAL_MENACE_KEY)
            })
            .map(|row| (row.0, row.1.position()))
            .ok_or("source boss identity")?
    };
    // Crystal onThink uses whole-second os.time and a strict deadline. Use
    // explicit semantic fixture clocks; the three-second warmup is unchanged.
    let now = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(1_000_000);
    super::srx3_primal_fixtures::drain_actual_native_through(&destination, now.get(), true).await?;
    assert!(
        destination
            .states
            .spell_states
            .lock()
            .await
            .primal_callbacks
            .source_started_for_test(),
        "the real reserved onThink initializes the source wave clock"
    );
    // The wave draws are keyed by the artifact-wide native source digest, so
    // any accepted content change moves the scheduled cells. Select, inside
    // the same whole source second, the first occurrence whose source oracle
    // schedules one cell twice; the native duplicate-cell regression below is
    // then asserted against that occurrence rather than a digest-bound literal.
    let root_bytes: [u8; 32] = Sha256::new()
        .chain_update(b"srx3:primal:callbacks:v1")
        .chain_update(native.source_digest())
        .chain_update(boss.placement_identity())
        .finalize()
        .into();
    let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(root_bytes);
    let draw_budget =
        2 + u64::from(rules::BEAST_WAVE.count_max) * u64::from(rules::SPAWN_ATTEMPTS) * 4;
    let wave_draws = |wave_at: u64| -> TestResult<Vec<u32>> {
        let mut occurrence = [0; 16];
        occurrence[..8].copy_from_slice(&wave_at.to_be_bytes());
        let occurrence =
            oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(occurrence);
        let mut ordinal = 0;
        (0..draw_budget)
            .map(|_| -> TestResult<u32> {
                let limit = u64::MAX - u64::MAX % u64::from(rules::DRAW_PPM);
                for _ in 0..64 {
                    let value = oteryn_simulation_determinism::deterministic_decision_u64(
                        &root,
                        occurrence,
                        "primal_waves",
                        ordinal,
                    )
                    .map_err(debug)?;
                    ordinal += 1;
                    if value < limit {
                        return Ok((value % u64::from(rules::DRAW_PPM)) as u32);
                    }
                }
                Err("qualified source wave draw refused".into())
            })
            .collect()
    };
    let (wave_at, duplicate_cell, expected_cells) = {
        let runtime = destination.runtime.lock().await;
        let snapshot = runtime.companion_snapshot(boss).map_err(debug)?;
        let health = u64::try_from(snapshot.health)?;
        let max_health = u64::try_from(snapshot.maximum_health)?;
        let tile_free = |at| {
            crate::gameplay_transport::arena_match::walkable(&destination, &runtime, at)
                && runtime.native_summon_cell_free(at)
        };
        let facts = |now_ms| rules::PrimalMenaceThinkFacts {
            position: boss_position,
            now_ms,
            health,
            max_health,
            damage_map_levels: &[],
        };
        // Replay the source schedule, including its six placement attempts. The
        // three-second delay means earlier requests have not occupied cells yet.
        type SourceSchedule = (usize, Vec<(u64, MovementLocalPosition)>);
        let source_schedule = |wave_at: u64| -> TestResult<SourceSchedule> {
            let draws = wave_draws(wave_at)?;
            let draw = |ordinal: usize| -> TestResult<u32> {
                draws
                    .get(ordinal)
                    .copied()
                    .ok_or("source oracle draw budget".into())
            };
            let requested_wave = usize::try_from(rules::wave_count(
                &rules::BEAST_WAVE,
                rules::hazard_points(&[]),
                draw(0)?,
                draw(1)?,
            ))?;
            let mut draws = draws.iter().copied();
            let mut next_draw = || draws.next().unwrap_or(rules::DRAW_PPM);
            let mut schedule = rules::PrimalMenaceThinkOwner::default();
            schedule
                .think(
                    &facts(now.get() / 1_000_000 * 1000),
                    &mut next_draw,
                    tile_free,
                    |_| None,
                )
                .map_err(debug)?;
            let scheduled_spawns = schedule
                .think(
                    &facts(wave_at / 1_000_000 * 1000),
                    &mut next_draw,
                    tile_free,
                    |_| None,
                )
                .map_err(debug)?
                .into_iter()
                .enumerate()
                .filter_map(|(index, effect)| match effect {
                    rules::PrimalMenaceEffect::ScheduledSpawn {
                        kind: rules::WaveKind::Beast,
                        position,
                        ..
                    } => Some((index as u64 + 1, position)),
                    _ => None,
                })
                .collect();
            Ok((requested_wave, scheduled_spawns))
        };
        // Crystal the_primal_menace.lua:279 calls Game.createMonster with
        // normal placement. map.cpp:254-309 tries eight shuffled neighbours
        // after an occupied centre; it does not discard the second request.
        let source_terrain = active
            .qualify_activity_source_room(&scene.room, PinnedActivitySourceRoom::Primal)
            .map_err(debug)?;
        let source_world = source_terrain
            .source_world()
            .ok_or("qualified Primal source terrain")?;
        let source_admissions = |scheduled_spawns: &[(u64, MovementLocalPosition)]|
         -> TestResult<std::collections::BTreeSet<_>> {
            let mut admitted_cells = std::collections::BTreeSet::new();
            for &(callback, at) in scheduled_spawns {
                let mut occurrence = [0; 16];
                occurrence[..8].copy_from_slice(&callback.to_be_bytes());
                let occurrence =
                    oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(occurrence);
                let mut ordinal = 0;
                let plan =
                    oteryn_gameplay_world::movement::source_placement::SourcePlacementPlan::prepare(
                        at,
                        false,
                        false,
                        |bound| {
                            let limit = u64::MAX - u64::MAX % u64::from(bound);
                            for _ in 0..64 {
                                let value = oteryn_simulation_determinism::deterministic_decision_u64(
                                    &root,
                                    occurrence,
                                    "primal_source_placement",
                                    ordinal,
                                );
                                ordinal += 1;
                                if let Ok(value) = value
                                    && value < limit
                                {
                                    return (value % u64::from(bound)) as u32;
                                }
                            }
                            bound
                        },
                    )
                    .map_err(debug)?;
                let placed = plan
                    .select(
                        |at| {
                            let cell = crate::content::LogicalCell {
                                x: at.x, y: at.y, z: i32::from(at.floor),
                            };
                            let tile = source_world.immutable_tile(source_world.scope(), cell).ok()?;
                            let flags = tile.flags();
                            let teleport = source_world.pending_mutable_placements().iter().any(|placement| {
                                placement.source_position() == [cell.x, cell.y, cell.z]
                                    && placement.source_kind() == Some("teleport")
                            });
                            Some(oteryn_gameplay_world::movement::source_placement::SourcePlacementCell {
                                exists: true,
                                query_add: tile_free(at) && tile.ground_present()
                                    && !flags.immovable_block_solid && !flags.floor_change
                                    && !flags.protection_zone && !teleport
                                    && !admitted_cells.contains(&(at.x, at.y, at.floor)),
                                protection_zone: flags.protection_zone,
                                teleport,
                            })
                        },
                        |_, _| Some(true),
                    )
                    .map_err(debug)?;
                if let Some(at) = placed {
                    assert!(
                        admitted_cells.insert((at.x, at.y, at.floor)),
                        "normal placement cannot stack creatures"
                    );
                }
            }
            Ok(admitted_cells)
        };
        // Stay inside the 12th source second used by the fixed cadence below.
        // Select only an occurrence whose replayed source requests are all
        // scheduled, one cell is requested exactly twice and normal placement
        // admits that cell, so the regression below is exercised.
        let mut selected = None;
        for wave_at in 12_000_000..12_100_000 {
            let (requested_wave, scheduled_spawns) = source_schedule(wave_at)?;
            if scheduled_spawns.len() != requested_wave {
                continue;
            }
            let duplicate = scheduled_spawns
                .iter()
                .map(|(_, at)| *at)
                .find(|cell| scheduled_spawns.iter().filter(|(_, at)| at == cell).count() == 2);
            let Some(duplicate_cell) = duplicate else {
                continue;
            };
            let admitted_cells = source_admissions(&scheduled_spawns)?;
            if admitted_cells.contains(&(duplicate_cell.x, duplicate_cell.y, duplicate_cell.floor))
            {
                selected = Some((
                    wave_at,
                    requested_wave,
                    duplicate_cell,
                    scheduled_spawns,
                    admitted_cells,
                ));
                break;
            }
        }
        let (wave_at, requested_wave, duplicate_cell, scheduled_spawns, admitted_cells) =
            selected.ok_or("no source occurrence schedules an admitted duplicate cell")?;
        let wave_at = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(wave_at);
        let scheduled_cells = scheduled_spawns
            .iter()
            .map(|(_, at)| *at)
            .collect::<Vec<_>>();
        eprintln!(
            "SRVGAP2_PRIMAL_ORACLE wave_at_us={} root={root_bytes:?} center={boss_position:?} scheduled={scheduled_cells:?}",
            wave_at.get()
        );
        assert_eq!(
            scheduled_cells.len(),
            requested_wave,
            "source wave requests"
        );
        assert_eq!(
            scheduled_cells
                .iter()
                .filter(|at| **at == duplicate_cell)
                .count(),
            2,
            "corrected content schedules the duplicate cell twice"
        );
        (wave_at, duplicate_cell, admitted_cells)
    };
    let expected_wave = expected_cells.len();
    super::srx3_primal_fixtures::drain_actual_native_through(&destination, wave_at.get(), true).await?;
    assert_eq!(beasts(&destination).await?, 0, "source three-second warmup");
    let warmed_at = wave_at.get() + rules::SPAWN_WARMUP_MS * 1000;
    super::srx3_primal_fixtures::drain_actual_native_through(&destination, warmed_at, true).await?;
    let spawned = beast_positions(&destination).await?;
    assert_eq!(
        spawned.len(),
        expected_wave,
        "actual scheduled native Primal beasts match the source oracle"
    );
    assert_eq!(
        spawned.iter().filter(|at| **at == duplicate_cell).count(),
        1,
        "the duplicate scheduled cell admits exactly one beast"
    );
    assert_eq!(
        spawned
            .iter()
            .map(|at| (at.x, at.y, at.floor))
            .collect::<std::collections::BTreeSet<_>>(),
        expected_cells,
        "native beast positions match successful source admissions"
    );
    super::srx3_primal_fixtures::drain_actual_native_through(&destination, warmed_at, true).await?;
    assert_eq!(
        beasts(&destination).await?,
        expected_wave,
        "same owner turn cannot duplicate a wave"
    );
    eprintln!("SRVGAP2_PRIMAL_STAGE native_cadence_spawned_exact_wave");
    destination.clock_origin -= Duration::from_micros(warmed_at + 1000);
    let now = destination.owner_now();
    let boss_policy = native
        .policies()
        .policies()
        .iter()
        .find(|row| row.definition_key == rules::PRIMAL_MENACE_KEY)
        .ok_or("source boss policy")?;
    let rewards = native
        .reward_table()
        .row(rules::PRIMAL_MENACE_KEY)
        .map_err(debug)?;
    assert!(
        rewards.loot_table.entries.is_empty(),
        "source literal empty loot is retained"
    );
    {
        let mut runtime = destination.runtime.lock().await;
        let target = runtime
            .positioned_melee_census()
            .map_err(debug)?
            .into_iter()
            .find(|(target, _, _)| {
                runtime
                    .companion_snapshot(*target)
                    .is_ok_and(|row| row.state.policy.definition_key == rules::PRIMAL_MENACE_KEY)
            })
            .map(|row| row.0)
            .ok_or("living source boss")?;
        let current = admitted.item_fence.ok_or("Instance fence")?;
        let batch = OwnerCombatBatch {
            caster: actor,
            attacker: crate::foundation::CharacterId::decode(current.character_id.as_bytes())
                .map_err(debug)?,
            current_lease_generation: current.character_lease_generation,
            command: CommandRef::new(admitted.game_session_id, CommandId::new(91).map_err(debug)?),
            occurrence: SpellOccurrenceBinding {
                id: "srvgap2-primal-lethal".into(),
                revisions: ["r1", "r1", "r1", "r1", "r1"].map(str::to_owned),
            },
            binding: b"srvgap2-primal-lethal".to_vec(),
            anchor: None,
            now_us: Some(now),
            now_ms: now.get() / 1000,
            effects: vec![OwnerCombatEffect {
                target,
                sub_ordinal: 0,
                change: OwnerCombatChange::Damage {
                    target_atom: runtime.creature_spell_target_atom(target).map_err(debug)?,
                    magnitude: boss_policy.maximum_health,
                },
            }],
            deferred: None,
        };
        let staged = runtime.stage_spell_batch(&batch).map_err(debug)?;
        let receipt = runtime.commit_spell_batch(staged).map_err(debug)?;
        assert!(receipt.applied);
        assert!(receipt.effects.iter().any(|effect| {
            effect.target == target
                && effect
                    .health
                    .is_some_and(|health| health.health_before > 0 && health.health_after == 0)
        }));
        let attack = destination.states.attack.try_lock().map_err(debug)?;
        record_batch_kills(
            &mut runtime,
            KillSink {
                prey: None,
                activity: &[],
                queue: attack.kills(),
                rewards: native.reward_table(),
                charms: None,
            },
            &receipt,
            now.get() / 1000,
        );
        assert!(runtime.retains_corpse_projection(target));
    }
    assert_eq!(
        destination
            .drain_kill_rewards(admitted.game_session_id)
            .await,
        DrainOutcome::Drained
    );
    assert_eq!(sqlx::query_as::<_,(i16,i16)>("SELECT unlocked,selected FROM game_character_hazard WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?, (2,1));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_primal_hazard_receipts")
            .fetch_one(&h.pool)
            .await?,
        1
    );
    let quest = h
        .root
        .read_character_quest_state(&authority, fence.character_id)
        .await?;
    assert_eq!(
        quest.tracks().get(
            "oteryn:quest-progress/quest/u12_90/primal_ordeal/bosses/the_primal_menace_killed"
        ),
        Some(&1)
    );
    eprintln!("SRVGAP2_PRIMAL_STAGE native_death_and_hazard_quest_settlement");
    let activity: (i16, i64, i64) =
        sqlx::query_as("SELECT state,terminal_at,exit_deadline FROM game_boss_room_activities")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(activity.0, 2);
    assert_eq!(
        activity.2,
        activity.1 + 60_000,
        "unmodified source after-kill policy"
    );
    assert_eq!(destination.runtime.lock().await.visible_corpses().len(), 1);
    let RuntimeScopeRefV1::Instance { instance_id, .. } = instance else {
        return Err("actual corpse Instance".into());
    };
    let corpse_item = rewards
        .corpse_item
        .as_ref()
        .ok_or("Primal source corpse is required")?;
    assert_eq!(sqlx::query_scalar::<_,i64>(
        "SELECT count(*) FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE g.instance_id=encode($1,'hex')::uuid AND g.channel_id IS NULL AND i.definition_production_key=$2")
        .bind(instance_id.as_slice()).bind(&corpse_item.production_key).fetch_one(&h.pool).await?,1,
        "the source corpse is durably placed in this exact Instance");
    // Wait for the real durable source deadline; never shorten the captured policy.
    tokio::time::timeout(Duration::from_secs(90), async {
        loop {
            let due:bool=sqlx::query_scalar("SELECT exit_deadline <= floor(extract(epoch FROM clock_timestamp())*1000)::bigint FROM game_boss_room_activities").fetch_one(&h.pool).await?;
            if due { return Ok::<(),sqlx::Error>(()); }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }).await??;
    let result = destination
        .return_due_activity(admitted, ActivityReturnReason::Defeated)
        .await
        .map_err(|_| "source deadline return")?;
    let NativeActivityOutcome::Committed(commit) = result else {
        return Err(format!("return refused: {result:?}").into());
    };
    let returned = commit.connections.first().ok_or("returned session")?.1;
    owner
        .publish_activity_commit(&commit)
        .map_err(|_| "publish source return")?;
    assert!(matches!(
        returned.item_fence.ok_or("return fence")?.runtime_scope,
        RuntimeScopeRefV1::Channel { .. }
    ));
    let exit = template.exit_position();
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(returned.runtime_actor.ok_or("returned actor")?)
            .map_err(debug)?
            .position(),
        MovementLocalPosition {
            x: exit.x,
            y: exit.y,
            floor: i16::try_from(exit.z)?
        }
    );
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_activity_runtime_returns WHERE reason=3 AND command_id IS NULL").fetch_one(&h.pool).await?,1);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_primal_hazard_receipts")
            .fetch_one(&h.pool)
            .await?,
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i16>("SELECT state FROM game_boss_room_activities")
            .fetch_one(&h.pool)
            .await?,
        4
    );
    eprintln!(
        "SRVGAP2_PRIMAL_CHAIN POSTGRES_CASE_COMPLETED entry_cadence_death_settlement_return duration={:?}",
        started.elapsed()
    );
    Ok(())
}

async fn beasts(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
) -> TestResult<usize> {
    Ok(beast_positions(owner).await?.len())
}

async fn beast_positions(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
) -> TestResult<Vec<MovementLocalPosition>> {
    let runtime = owner.runtime.lock().await;
    Ok(runtime
        .positioned_melee_census()
        .map_err(debug)?
        .iter()
        .filter_map(|(actor, position, _)| {
            runtime.companion_snapshot(*actor).ok().and_then(|row| {
                rules::PRIMAL_BEAST_POOL
                    .contains(&row.state.policy.definition_key.as_str())
                    .then_some(position.position())
            })
        })
        .collect())
}
