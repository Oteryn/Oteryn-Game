//! Actual source entry and native callback effects for the three advanced bosses.
use super::fixture::{Harness, debug, run};
use super::srx3_activity_source_fixture::{SourceScene, install_scene, prepare_player};
use super::{Dependencies, TestResult, reset};
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::foundation::{MovementLocalPosition, RuntimeScopeRefV1};
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use oteryn_protocol_oteryn::boss_encounter as wire;
#[test]
fn srx3_vemiath_actual_source_native_effects_postgres() -> TestResult {
    scenario(
        PinnedActivitySourceRoom::Vemiath,
        MovementLocalPosition {
            x: 33078,
            y: 32333,
            floor: 15,
        },
        "oteryn:creature.vemiath",
        false,
        false,
    )
}
#[test]
fn srx3_chagorz_actual_source_native_effects_postgres() -> TestResult {
    scenario(
        PinnedActivitySourceRoom::Chagorz,
        MovementLocalPosition {
            x: 33074,
            y: 32367,
            floor: 15,
        },
        "oteryn:creature.chagorz",
        false,
        false,
    )
}
#[test]
fn srx3_bakragore_actual_source_native_effects_postgres() -> TestResult {
    scenario(
        PinnedActivitySourceRoom::Bakragore,
        MovementLocalPosition {
            x: 33010,
            y: 32392,
            floor: 15,
        },
        "oteryn:creature.bakragore",
        true,
        false,
    )
}
#[test]
fn srx3_vemiath_actual_source_teleport_exit_return_postgres() -> TestResult {
    scenario(
        PinnedActivitySourceRoom::Vemiath,
        MovementLocalPosition {
            x: 33078,
            y: 32333,
            floor: 15,
        },
        "oteryn:creature.vemiath",
        false,
        true,
    )
}
#[derive(Clone, Copy)]
enum Extra {
    None,
    PortalStep,
    Footprints,
    DueBeforeDeath,
    CopiedDeath { credited: bool, prepared_bak: bool },
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
        },
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
#[test]
fn srx3_vemiath_actual_source_portal_step_postgres() -> TestResult {
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
        Extra::PortalStep,
    )
}
#[test]
fn srx3_vemiath_original_portal_due_precedes_death_and_late_mint_postgres() -> TestResult {
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
        Extra::DueBeforeDeath,
    )
}
fn scenario(
    kind: PinnedActivitySourceRoom,
    origin: MovementLocalPosition,
    boss: &'static str,
    final_boss: bool,
    return_only: bool,
) -> TestResult {
    scenario_with_step(kind, origin, boss, final_boss, return_only, Extra::None)
}
fn scenario_with_step(
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
    let result = super::kill_reward_e2e::scenario_thread::run("srx3-rotten-advanced", move || {
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
            } else {
                active
                    .qualify_rotten_blood_activity_template(&scene.room, revision, kind)
                    .map_err(|error| format!("actual {kind:?} source entry template: {error:?}"))?;
            }
            let deps: Dependencies = scene.dependencies()?;
            let h =
                Harness::create_with_admission_seed(admin, "srx3_rotten_advanced", false, false)
                    .await?;
            let evidence = super::house_transfer::RecoverySource::start(&h).await?;
            super::house_transfer::prepare_owned_session(&h).await?;
            let prepared_bak = matches!(
                extra,
                Extra::CopiedDeath {
                    prepared_bak: true,
                    ..
                }
            );
            if prepared_bak {
                assert!(final_boss);
                super::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
                // The established defensive historical fixture supplies one
                // original public flavor before any fresh authority. The real
                // FIRST Think below alone captures the live88888 value.
                super::srx3_rotten_preparation::seed_historical_defensive_global_phase(&h, &deps)
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
                super::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
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

            let instance = super::activity_runtime::allocate_ready_instance(&h, 213)
                .await
                .map_err(|error| format!("ready activity allocation: {error}"))?;
            let mut frames = Vec::new();
            let (_, reply, barrier) = super::boss_activity::exchange_capture(
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
            if return_only {
                super::srx3_vemiath_return::exercise(&h, &mut owner, &mut admitted).await?;
                drop(owner);
                drop(authority);
                drop(seal);
                return h.cleanup().await;
            }
            if matches!(extra, Extra::PortalStep) {
                super::srx3_rotten_portal_step::exercise(&h, &mut owner, &admitted).await?;
                drop(owner);
                drop(authority);
                drop(seal);
                return h.cleanup().await;
            }
            if matches!(extra, Extra::Footprints) {
                super::srx3_rotten_footprints::exercise(&h, &mut owner, &admitted, kind).await?;
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
            owner.runtime_view(&registered).drain_monster_melee().await;
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
            view.drain_monster_melee().await;
            if final_boss {
                view.drain_bak_first_think_claims().await;
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
                view.drain_monster_melee().await;
            }
            if let Extra::CopiedDeath { credited, .. } = extra {
                drop(view);
                super::srx3_rotten_death::exercise(
                    &h,
                    &owner,
                    &admitted,
                    boss,
                    credited,
                    u8::from(prepared_bak),
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
