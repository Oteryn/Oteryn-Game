use super::fixture::{Harness, debug, run};

use super::srx3_activity_source_fixture::{SourceScene, install_scene, prepare_player};

use super::{Dependencies, TestResult, reset};

use crate::content::bosses::activity_template::PinnedActivitySourceRoom;

use crate::foundation::{MovementLocalPosition, RuntimeScopeRefV1};

use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};

use oteryn_protocol_oteryn::boss_encounter as wire;

pub(super) fn bakragore_deadline_case() -> TestResult {
    scenario_with_step(
        PinnedActivitySourceRoom::Bakragore,
        MovementLocalPosition {
            x: 33010,
            y: 32392,
            floor: 15,
        },
        "oteryn:creature.bakragore",
        true,
        false,
        Extra::FinalDeadline,
    )
}

#[derive(Clone, Copy)]
pub(super) enum Extra {
    None,
    PortalStep,
    Footprints,
    FootprintsRestart,
    DueBeforeDeath,
    ControlLost,
    FinalDeadline,
    CopiedDeath {
        credited: bool,
        prepared_bak: bool,
        full_queue: bool,
    },
}

pub(super) fn copied_death(
    kind: PinnedActivitySourceRoom,
    origin: MovementLocalPosition,
    boss: &'static str,
    credited: bool,
    prepared_bak: bool,
) -> TestResult {
    scenario_with_step(
        kind,
        origin,
        boss,
        kind == PinnedActivitySourceRoom::Bakragore,
        false,
        Extra::CopiedDeath {
            credited,
            prepared_bak,
            full_queue: false,
        },
    )
}

pub(super) fn copied_death_with_full_queue() -> TestResult {
    scenario_with_step(
        PinnedActivitySourceRoom::Bakragore,
        MovementLocalPosition {
            x: 33010,
            y: 32392,
            floor: 15,
        },
        "oteryn:creature.bakragore",
        true,
        false,
        Extra::CopiedDeath {
            credited: true,
            prepared_bak: true,
            full_queue: true,
        },
    )
}

pub(super) fn footprints_restart() -> TestResult {
    scenario_with_step(
        PinnedActivitySourceRoom::Vemiath,
        MovementLocalPosition {
            x: 33078,
            y: 32333,
            floor: 15,
        },
        "oteryn:creature.vemiath",
        false,
        false,
        Extra::FootprintsRestart,
    )
}

pub(super) fn footprints(
    kind: PinnedActivitySourceRoom,
    origin: MovementLocalPosition,
    boss: &'static str,
) -> TestResult {
    scenario_with_step(
        kind,
        origin,
        boss,
        kind == PinnedActivitySourceRoom::Bakragore,
        false,
        Extra::Footprints,
    )
}

pub(super) fn scenario_with_step(
    kind: PinnedActivitySourceRoom,
    origin: MovementLocalPosition,
    boss: &'static str,
    final_boss: bool,
    return_only: bool,
    extra: Extra,
) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("srx3-rotten-advanced", move || {
        run(async |admin| {
            reset();
            // Exact source lever waiting position; actual pinned map collision.
            let scene = SourceScene::new(kind, origin)?;
            let active = scene
                .controller
                .active()
                .ok_or("active source generation")?;
            let revision = active
                .native_gameplay()
                .and_then(|native| native.training_formula())
                .map(|formula| formula.content_revision())
                .ok_or("native source content revision")?;
            active
                .native_gameplay()
                .ok_or("native source gameplay")?
                .reward_table()
                .row(boss)
                .map_err(|error| format!("actual {kind:?} source reward row: {error:?}"))?;
            if kind == PinnedActivitySourceRoom::Vemiath {
                let raw = crate::content::LogicalCell {
                    x: 33043,
                    y: 32344,
                    z: 15,
                };
                let effective =
                    crate::map::floor::resolve_teleport_arrival(scene.room.movement_cells(), raw)
                        .map_err(|error| {
                        format!("actual Vemiath source Teleport arrival: {error:?}")
                    })?;
                assert_eq!(
                    effective,
                    crate::content::LogicalCell {
                        x: 34001,
                        y: 31820,
                        z: 15
                    }
                );
                let template = active
                    .qualify_vemiath_activity_template(&scene.room, revision, effective)
                    .map_err(|error| format!("actual Vemiath composed exit template: {error:?}"))?;
                assert_eq!(template.exit_position(), effective);
                assert_eq!(
                    template.source_exit_position(),
                    raw,
                    "literal Lua exit evidence remains distinct from its composed landing"
                );
                assert!(
                    active
                        .qualify_vemiath_activity_template(
                            &scene.room,
                            revision,
                            crate::content::LogicalCell {
                                x: 34002,
                                ..effective
                            }
                        )
                        .is_err(),
                    "a caller cannot select another exit"
                );
            } else if kind == PinnedActivitySourceRoom::Ichgahal {
                let raw = crate::content::LogicalCell {
                    x: 33004,
                    y: 32343,
                    z: 15,
                };
                let effective =
                    crate::map::floor::resolve_teleport_arrival(scene.room.movement_cells(), raw)
                        .map_err(|error| {
                        format!("actual Ichgahal source Teleport arrival: {error:?}")
                    })?;
                assert_eq!(
                    effective,
                    crate::content::LogicalCell {
                        x: 34065,
                        y: 31717,
                        z: 15
                    }
                );
                let template = active
                    .qualify_ichgahal_activity_template(&scene.room, revision, effective)
                    .map_err(|error| {
                        format!("actual Ichgahal composed exit template: {error:?}")
                    })?;
                assert_eq!(template.exit_position(), effective);
                assert_eq!(
                    template.source_exit_position(),
                    raw,
                    "literal Lua exit evidence remains distinct from its composed landing"
                );
                assert!(
                    active
                        .qualify_ichgahal_activity_template(
                            &scene.room,
                            revision,
                            crate::content::LogicalCell {
                                x: 34066,
                                ..effective
                            }
                        )
                        .is_err(),
                    "a caller cannot select another Ichgahal exit"
                );
            } else {
                active
                    .qualify_rotten_blood_activity_template(&scene.room, revision, kind)
                    .map_err(|error| format!("actual {kind:?} source entry template: {error:?}"))?;
            }
            let deps: Dependencies = scene.dependencies()?;
            let h =
                Harness::create_with_admission_seed(admin, "srx3_rotten_advanced", false, false)
                    .await?;
            let evidence = super::house_transfer_fixtures::RecoverySource::start(&h).await?;
            super::house_transfer_fixtures::prepare_owned_session(&h).await?;
            let prepared_bak = matches!(
                extra,
                Extra::CopiedDeath {
                    prepared_bak: true,
                    ..
                }
            );
            if prepared_bak {
                assert!(final_boss);
                super::boss_activity_fixtures::issue_content_and_reset(&h, &scene.pin).await?;
                // The established defensive historical fixture supplies one
                // original public flavor before any fresh authority. The real
                // FIRST Think below alone captures the live88888 value.
                super::srx3_rotten_preparation_fixtures::seed_historical_defensive_global_phase(&h, &deps)
                    .await?;
            }
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.evidence = &evidence.evidence;
            install_scene(&mut owner, &scene, kind)?;
            if !prepared_bak {
                super::boss_activity_fixtures::issue_content_and_reset(&h, &scene.pin).await?;
            }
            let mut admitted = deps.admitted;
            admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                BUNDLE_WORLD_OFFERED_CAPABILITIES,
                &[
                    4,
                    6,
                    7,
                    10,
                    11,
                    15,
                    18,
                    23,
                    92,
                    wire::CAPABILITY_BOSS_ENCOUNTER_V1,
                ],
            )
            .ok_or("actual native boss capabilities")?;
            prepare_player(&h, &owner, &mut admitted, 1999)
                .await
                .map_err(|error| format!("native Character preparation: {error}"))?;
            // First initialize the native Character through its real owner.
            // A root with no progression row can only initialize at revision1;
            // fabricating revision2 before this step is an invalid fixture.
            let states = std::sync::Arc::clone(&owner.states);
            let sequencer = std::sync::Arc::clone(&owner.revision_sequencer);
            drop(owner);
            drop(authority);
            drop(seal);
            // Import only historical Quest access, then take NEW proofs for the
            // actual tested entry. The current native root revision is unchanged.
            let mut prior = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *prior)
                .await?;
            let imported=sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,'oteryn:quest-progress/rotten-blood/access','oteryn:quest/rotten-blood',4,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid AND character_revision>=2")
                .bind(super::fixture::id(41).as_slice()).execute(&mut *prior).await?.rows_affected();
            assert_eq!(imported, 1, "actual native root already advanced");
            if final_boss {
                // Imported historical completion is an entry precondition, not
                // a fabricated kill, reward receipt or current encounter result.
                sqlx::query("INSERT INTO game_character_rotten_blood(character_id,child_flags) VALUES(encode($1,'hex')::uuid,15)")
                    .bind(super::fixture::id(41).as_slice()).execute(&mut *prior).await?;
            }
            prior.commit().await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.states = states;
            owner.revision_sequencer = sequencer;
            owner.evidence = &evidence.evidence;
            install_scene(&mut owner, &scene, kind)?;
            owner
                .register_current_runtime()
                .await
                .map_err(|_| "register resealed current source runtime")?;
            owner
                .activity_runtimes
                .remember(admitted)
                .map_err(|_| "remember actual resealed source connection")?;

            let instance = super::activity_runtime_fixtures::allocate_ready_instance(&h, 213)
                .await
                .map_err(|error| format!("ready activity allocation: {error}"))?;
            let mut frames = Vec::new();
            let (_, reply, barrier) = super::boss_activity_fixtures::exchange_capture(
                &owner,
                &mut admitted,
                boss,
                true,
                &mut frames,
            )
            .await?;
            assert!(
                matches!(reply, wire::BossActivityReply::Accepted { .. }),
                "actual {boss} source entry: {reply:?}"
            );
            assert!(barrier.is_some());
            let fence = admitted.item_fence.ok_or("actual Instance fence")?;
            assert_eq!(fence.runtime_scope, instance);
            assert!(matches!(instance, RuntimeScopeRefV1::Instance { .. }));
            let registered = owner
                .activity_runtimes
                .get(instance, fence.scope_ownership_generation.get())
                .ok_or("actual source owner")?;
            if matches!(extra, Extra::FinalDeadline) {
                let RuntimeScopeRefV1::Instance { instance_id, .. } = instance else {
                    return Err("actual Bakragore Instance".into());
                };
                let (fight_ms, room_deadline, entry_deadline): (i64, i64, i64) = sqlx::query_as(
                    "SELECT a.deadline - a.opened_at, a.deadline, e.deadline
                     FROM game_boss_room_activities a
                     JOIN game_activity_runtime_entries e ON e.handoff_id=a.admission
                     WHERE a.instance_id=encode($1,'hex')::uuid AND a.boss_key=$2",
                )
                .bind(instance_id.as_slice())
                .bind("oteryn:boss.bakragore")
                .fetch_one(&h.pool)
                .await?;
                // Tibia Fandom archive, Rotten Blood Quest/Spoiler #Bakragore.
                assert_eq!(fight_ms, 1_500_000);
                assert_eq!(room_deadline, entry_deadline);
                assert!(matches!(
                    reply,
                    wire::BossActivityReply::Accepted { deadline_unix_ms: Some(deadline), .. }
                        if i64::try_from(deadline) == Ok(room_deadline)
                ));
                drop(owner);
                drop(authority);
                drop(seal);
                return h.cleanup().await;
            }
            if return_only {
                super::srx3_vemiath_return::exercise(&h, &mut owner, &mut admitted).await?;
                drop(owner);
                drop(authority);
                drop(seal);
                return h.cleanup().await;
            }
            if matches!(extra, Extra::PortalStep) {
                // Replay the native Think and summon placement draws observed on main f4a89b3e9.
                const MAIN_SEED: [u8; 32] = [
                    0x38, 0x86, 0x5b, 0x88, 0x17, 0x3a, 0x1b, 0xd5, 0x66, 0xe7, 0xfb, 0xa1, 0x4a,
                    0x04, 0xf9, 0xff, 0xf5, 0x55, 0xbb, 0x4b, 0x4a, 0xc0, 0xef, 0xb4, 0x20, 0x3b,
                    0x01, 0x4b, 0x95, 0x30, 0x8f, 0xf0,
                ];
                let turn = super::srx3_rotten_portal_step::exercise(&h, &owner, &admitted);
                crate::gameplay_transport::monster_profile_cycle::with_native_ai_seed(MAIN_SEED, turn)
                    .await?;
                drop(owner);
                drop(authority);
                drop(seal);
                return h.cleanup().await;
            }
            if matches!(extra, Extra::Footprints | Extra::FootprintsRestart) {
                super::srx3_rotten_footprints_fixtures::exercise(
                    &h,
                    &mut owner,
                    &admitted,
                    kind,
                    matches!(extra, Extra::FootprintsRestart),
                )
                .await?;
                drop(owner);
                drop(authority);
                drop(seal);
                return h.cleanup().await;
            }
            if matches!(extra, Extra::ControlLost) {
                let view = owner.runtime_view(&registered);
                let native = view
                    .active_generation
                    .and_then(|generation| generation.native_gameplay())
                    .ok_or("retained Rotten Content")?;
                let mut runtime = view.runtime.lock().await;
                let player = admitted.runtime_actor.ok_or("actual Rotten Player")?;
                let at = runtime
                    .read_actor_position(player)
                    .map_err(debug)?
                    .position();
                let parent = runtime
                    .visible_entities()
                    .creatures
                    .into_iter()
                    .find(|entry| {
                        runtime.matches_live_creature_identity(entry.actor, boss.as_bytes())
                    })
                    .ok_or("actual Rotten boss")?
                    .actor;
                let source = runtime
                    .qualify_rotten_blood_spawn_source(parent, native)
                    .map_err(debug)?;
                let players: Vec<_> = runtime
                    .positioned_melee_census()
                    .map_err(debug)?
                    .into_iter()
                    .filter_map(|(actor, _, session)| session.map(|session| (actor, session)))
                    .collect();
                assert!(!players.is_empty());
                for (actor, session) in players {
                    runtime
                        .record_control_loss(
                            actor,
                            session,
                            crate::foundation::ControlLossMark {
                                epoch: 1,
                                grace_deadline: 100,
                            },
                        )
                        .map_err(debug)?;
                }
                assert!(
                    runtime
                        .positioned_actor_census()
                        .map_err(debug)?
                        .iter()
                        .all(|(_, _, session)| session.is_none())
                );
                let child = "oteryn:creature.elder_bloodjaw";
                let plan = oteryn_gameplay_world::movement::source_placement::SourcePlacementPlan::prepare(
                    at, false, false, |_| 0,
                ).map_err(debug)?;
                let cell = |runtime: &crate::foundation::ChannelRuntimeV1,
                            at: MovementLocalPosition| {
                    let walkable = view.movement_cells.index().lookup(
                        view.movement_cells.scope(),
                        crate::content::LogicalCell {
                            x: at.x,
                            y: at.y,
                            z: i32::from(at.floor),
                        },
                    ) == Ok(crate::content::CollisionClass::Walkable);
                    let occupancy = runtime
                        .current_native_source_occupancy(native, child, at, None)
                        .ok()?;
                    Some(
                        oteryn_gameplay_world::movement::source_placement::SourcePlacementCell {
                            exists: walkable,
                            query_add: walkable && occupancy.admits_occupants,
                            protection_zone: false,
                            teleport: false,
                        },
                    )
                };
                let selected = plan
                    .select(|at| cell(&runtime, at), |_, _| Some(true))
                    .map_err(debug)?
                    .ok_or("free native neighboring cell")?;
                let spawned = runtime.spawn_rotten_blood_child_from_source(
                    &source,
                    child,
                    selected,
                    |runtime, at| cell(runtime, at).map(|cell| cell.query_add),
                );
                assert_eq!(
                    (selected != at, spawned.is_ok()),
                    (true, true),
                    "control-lost Players occupy their cell and retain the encounter: {spawned:?}",
                );
                let spawned = spawned.map_err(debug)?;
                assert_eq!(
                    runtime
                        .read_actor_position(spawned)
                        .map_err(debug)?
                        .position(),
                    selected
                );
                drop(runtime);
                drop(view);
                drop(owner);
                drop(authority);
                drop(seal);
                return h.cleanup().await;
            }
            if matches!(extra, Extra::DueBeforeDeath) {
                super::srx3_rotten_due::exercise(&h, &mut owner, &admitted).await?;
                drop(owner);
                drop(authority);
                drop(seal);
                return h.cleanup().await;
            }
            super::srx3_activity_source_fixture::drive_native_cycle(
                &owner.runtime_view(&registered),
            )
            .await?;
            if final_boss {
                super::srx3_rotten_bak_claim::assert_mask(&h, instance, 0).await?;
                let player = admitted.runtime_actor.ok_or("actual Bakragore Player")?;
                {
                    let view = owner.runtime_view(&registered);
                    let runtime = view.runtime.lock().await;
                    assert_eq!(
                        runtime
                            .read_actor_position(player)
                            .map_err(debug)?
                            .position(),
                        MovementLocalPosition {
                            x: 33044,
                            y: 32407,
                            floor: 15
                        }
                    );
                    assert!(
                        runtime.visible_entities().creatures.iter().all(|c| !runtime
                            .matches_live_creature_identity(
                                c.actor,
                                b"oteryn:creature.elder_bloodjaw"
                            )),
                        "source idle boss does not run onThink outside11 tiles"
                    );
                }
                // Pinned boss33044/32394 and arrival33044/32407 are13Y apart.
                // The real player approaches into Crystal's11-tile wake range.
                for _ in 0..2 {
                    owner.clock_origin = owner
                        .clock_origin
                        .checked_sub(std::time::Duration::from_secs(1))
                        .ok_or("Bakragore approach clock")?;
                    let view = owner.runtime_view(&registered);
                    let (step, _) = view
                        .ordinary_paced_step(
                            player,
                            admitted.game_session_id,
                            oteryn_protocol_oteryn::world_spatial::StepDirection::North,
                            None,
                            None,
                        )
                        .await;
                    assert_eq!(
                        step.disposition,
                        oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved,
                        "real source Bakragore approach"
                    );
                }
                let view = owner.runtime_view(&registered);
                let runtime = view.runtime.lock().await;
                assert_eq!(
                    runtime
                        .read_actor_position(player)
                        .map_err(debug)?
                        .position(),
                    MovementLocalPosition {
                        x: 33044,
                        y: 32405,
                        floor: 15
                    }
                );
            }
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(2))
                .ok_or("owner clock")?;
            let view = owner.runtime_view(&registered);
            super::srx3_activity_source_fixture::drive_native_cycle(&view).await?;
            if final_boss {
                view.drain_bak_first_think_claims().await;
                #[cfg(test)]
                {
                    let runtime = view.runtime.lock().await;
                    if let Ok(states) = view.states.spell_states.try_lock() {
                        let boss_actor = runtime
                            .visible_entities()
                            .creatures
                            .iter()
                            .find(|c| {
                                runtime.matches_live_creature_identity(c.actor, boss.as_bytes())
                            })
                            .map(|c| c.actor);
                        let native = view
                            .active_generation
                            .and_then(|active| active.native_gameplay());
                        eprintln!(
                            "SRX3_BAK_FIRST_THINK_PRE_ASSERT now={} next_ai={} next_swing={} source_work={:?} profile_due={:?} profile={:?} movement_pending={} movement_due={:?}",
                            view.owner_now().get(),
                            states.next_monster_ai_pass_us,
                            states.monster_melee.next_swing_pass_us,
                            native.map(|native| {
                                crate::gameplay_transport::source_death::next_source_work(
                                    &runtime, &states, native,
                                )
                            }),
                            states.native_profile_pass.next_due(&runtime),
                            states.native_profile_pass.pending(),
                            runtime.rotten_movement_pending(),
                            runtime
                                .pending_rotten_movement(1)
                                .map(|events| events.first().map(|event| event.observed_us()))
                        );
                        eprintln!(
                            "SRX3_BAK_FIRST_THINK_PRE_ASSERT boss={boss_actor:?} position={:?} ai={:?} due={:?} advanced={:?}",
                            boss_actor.map(|actor| runtime.read_actor_position(actor)),
                            boss_actor
                                .and_then(|actor| states.monster_melee.creature_ai.state(actor)),
                            boss_actor
                                .and_then(|actor| states.monster_melee.creature_ai.due(actor)),
                            states
                                .rotten_blood_callbacks
                                .portal_diagnostics_for_test(&runtime)
                        );
                        let player = admitted.runtime_actor;
                        eprintln!(
                            "SRX3_BAK_FIRST_THINK_PRE_ASSERT player={player:?} position={:?} health={:?} dead={:?} pending={:?} unreserved={:?} staff_visibility={:?}",
                            player.map(|actor| runtime.read_actor_position(actor)),
                            player
                                .and_then(|actor| states.get(
                                    &runtime,
                                    actor,
                                    admitted.game_session_id
                                ))
                                .map(|state| state.vitals().health),
                            player.map(|actor| states.is_dead(actor)),
                            player.map(|actor| states
                                .has_pending_spell_commit(actor, admitted.game_session_id)),
                            player.map(|actor| runtime.assert_actor_spell_unreserved(actor)),
                            player.map(|actor| runtime.actor_staff_visibility(actor))
                        );
                    } else {
                        eprintln!("SRX3_BAK_FIRST_THINK_PRE_ASSERT spell_states_busy=true");
                    }
                }
                if prepared_bak {
                    let RuntimeScopeRefV1::Instance { instance_id, .. } = instance else {
                        return Err("prepared source Bakragore must enter an Instance".into());
                    };
                    let masks: Vec<i16> = sqlx::query_scalar(
                        "SELECT p.essence_mask FROM game_rotten_bak_phase_preparations p \
                         JOIN game_activity_handoff_groups g USING(handoff_id) \
                         WHERE g.instance_id=encode($1,'hex')::uuid",
                    )
                    .bind(instance_id.as_slice())
                    .fetch_all(&h.pool)
                    .await?;
                    assert_eq!(
                        masks,
                        vec![1],
                        "only actual FIRST Think captures the historical flavor"
                    );
                } else {
                    super::srx3_rotten_bak_claim::assert_mask(&h, instance, 1).await?;
                }
                super::srx3_activity_source_fixture::drive_native_cycle(&view).await?;
            }
            if let Extra::CopiedDeath {
                credited,
                full_queue,
                ..
            } = extra
            {
                drop(view);
                super::srx3_rotten_death_fixtures::exercise(
                    &h,
                    &owner,
                    &admitted,
                    boss,
                    credited,
                    u8::from(prepared_bak),
                    full_queue,
                )
                .await?;
                drop(owner);
                drop(authority);
                drop(seal);
                return h.cleanup().await;
            }
            let runtime = view.runtime.lock().await;
            let creatures = runtime.visible_entities().creatures;
            let count = |key: &[u8]| {
                creatures
                    .iter()
                    .filter(|c| runtime.matches_live_creature_identity(c.actor, key))
                    .count()
            };
            assert_eq!(count(boss.as_bytes()), 1);
            let jaws = count(b"oteryn:creature.elder_bloodjaw");
            let pillars = count(b"oteryn:creature.pillar_of_dark_energy");
            if jaws == 0 {
                let states = view.states.spell_states.lock().await;
                let boss_actor = creatures
                    .iter()
                    .find(|c| runtime.matches_live_creature_identity(c.actor, boss.as_bytes()))
                    .ok_or("actual source boss")?
                    .actor;
                eprintln!(
                    "SRX3_ROTTEN_ACTUAL_NATIVE_DIAGNOSTIC boss={boss} ai={:?} due={:?} advanced={:?} callback_due={:?} now={}",
                    states.monster_melee.creature_ai.state(boss_actor),
                    states.monster_melee.creature_ai.due(boss_actor),
                    states
                        .rotten_blood_callbacks
                        .advanced
                        .source_state_for_test(),
                    states.rotten_blood_callbacks.next_due_time(),
                    owner.owner_now().get()
                );
            }
            assert!(
                (1..=2).contains(&jaws),
                "source taint0 native Bloodjaw top-up: {jaws}"
            );
            assert!(
                (1..=4).contains(&pillars),
                "source four-Pillar floor has real native consumers: {pillars}"
            );
            if final_boss {
                let mushrooms = count(b"oteryn:creature.mushroom");
                assert!(
                    (1..=4).contains(&mushrooms),
                    "Bakragore's Mushroom floor has real native consumers: {mushrooms}"
                );
            } else {
                let actor = admitted.runtime_actor.ok_or("actual private player")?;
                assert!(
                    runtime
                        .rotten_blood_keyed_icons(actor)
                        .map_err(debug)?
                        .iter()
                        .flatten()
                        .any(|(key, _, _, count)| *key == "agony-stacks" && *count > 0),
                    "stationary real player receives source Agony stack presentation"
                );
                let states = view.states.spell_states.lock().await;
                let (before, after, amount) = states
                    .rotten_blood_callbacks
                    .actual_agony_commit_for_test(actor)
                    .ok_or("actual native source Agony HP commit, not merely an icon")?;
                let maximum = states
                    .get(&runtime, actor, admitted.game_session_id)
                    .ok_or("actual source Player HP")?
                    .vitals()
                    .max_health;
                assert_eq!(amount, u32::try_from(u64::from(maximum) * 3 / 100)?);
                assert_eq!(
                    before - after,
                    amount,
                    "canonical source Agony bypasses block/mitigation and commits real HP"
                );
            }
            assert!(
                creatures
                    .iter()
                    .all(|c| c.actor.runtime_scope() == instance)
            );
            drop(runtime);
            drop(view);
            if kind == PinnedActivitySourceRoom::Vemiath {
                super::srx3_rotten_portal::exercise(&h, &mut owner, &admitted).await?;
            }
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    });
    eprintln!(
        "SRX3_ROTTEN_ADVANCED_NATIVE_POSTGRES_CASE_EXECUTED duration={:?} passed={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
