//! Actual pinned Murcion entry and native scheduler; no invented boss/catalogue.
use super::fixture::{Harness, debug, run};
use super::srx3_activity_source_fixture::{SourceScene, install_scene, prepare_player};
use super::{Dependencies, TestResult, reset};
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::foundation::{MovementLocalPosition, RuntimeScopeRefV1};
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::map::boot::BundleCollisionIndexExt;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_protocol_oteryn::boss_encounter as wire;

#[test]
fn srx3_murcion_actual_source_entry_native_think_spawns_owned_children_postgres() -> TestResult {
    run_murcion_source_entry(false, false)
}

#[test]
fn srx3_murcion_published_whole_world_entry_native_children_postgres() -> TestResult {
    run_murcion_source_entry(true, false)
}

#[test]
fn srx3_murcion_native_mushroom_fuse_removes_original_children_postgres() -> TestResult {
    run_murcion_source_entry(false, true)
}

#[test]
fn srx3_murcion_native_mushroom_normal_damage_and_effects_postgres() -> TestResult {
    run_murcion_source_entry_checks(
        PinnedActivitySourceRoom::Murcion,
        false,
        false,
        true,
        false,
        false,
        false,
    )
}

#[test]
fn srx3_murcion_native_mushroom_retry_keeps_original_cohort_and_damage_postgres() -> TestResult {
    run_murcion_source_entry_checks(
        PinnedActivitySourceRoom::Murcion,
        false,
        false,
        true,
        false,
        false,
        true,
    )
}

#[test]
fn srx3_murcion_actual_native_lethal_retains_canonical_source_death_notice_postgres() -> TestResult
{
    run_murcion_source_entry_checks(
        PinnedActivitySourceRoom::Murcion,
        false,
        false,
        false,
        true,
        false,
        false,
    )
}

#[test]
fn srx3_murcion_native_death_barrier_runs_only_originally_eligible_mushroom_fuse_postgres()
-> TestResult {
    run_murcion_source_entry_checks(
        PinnedActivitySourceRoom::Murcion,
        false,
        false,
        true,
        false,
        true,
        false,
    )
}

#[test]
fn srx3_ichgahal_actual_source_entry_native_think_spawns_owned_children_postgres() -> TestResult {
    run_murcion_source_entry_checks(
        PinnedActivitySourceRoom::Ichgahal,
        false,
        false,
        false,
        false,
        false,
        false,
    )
}

fn retain_murcion_native_lethal(
    runtime: &mut crate::foundation::ChannelRuntimeV1,
    caster: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
    character: [u8; 16],
    lease: u64,
    now: oteryn_simulation_determinism::SemanticTimeMicros,
) -> TestResult {
    use crate::foundation::runtime_actor_spell_types::{
        OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
    };
    use crate::foundation::{CommandId, CommandRef};
    let boss = runtime
        .visible_entities()
        .creatures
        .into_iter()
        .find(|entry| {
            runtime.matches_live_creature_identity(entry.actor, b"oteryn:creature.murcion")
        })
        .ok_or("actual native Murcion")?
        .actor;
    let maximum = runtime
        .companion_snapshot(boss)
        .map_err(debug)?
        .maximum_health;
    assert!(
        runtime
            .next_source_creature_death()
            .map_err(debug)?
            .is_none(),
        "no older invented death notice"
    );
    let batch = OwnerCombatBatch {
        caster,
        attacker: crate::foundation::CharacterId::decode(&character).map_err(debug)?,
        current_lease_generation: lease,
        command: CommandRef::new(session, CommandId::new(93).map_err(debug)?),
        occurrence: SpellOccurrenceBinding {
            id: "srx3:murcion-native-source-death".into(),
            revisions: ["r1", "r1", "r1", "r1", "r1"].map(str::to_owned),
        },
        binding: b"srx3:murcion-native-source-death".to_vec(),
        anchor: None,
        now_us: Some(now),
        now_ms: now.get() / 1000,
        effects: vec![OwnerCombatEffect {
            target: boss,
            sub_ordinal: 0,
            change: OwnerCombatChange::Damage {
                target_atom: runtime.creature_spell_target_atom(boss).map_err(debug)?,
                magnitude: maximum,
            },
        }],
        deferred: None,
    };
    let stage = runtime.stage_spell_batch(&batch).map_err(debug)?;
    let receipt = runtime.commit_spell_batch(stage).map_err(debug)?;
    assert!(
        receipt
            .effects
            .iter()
            .any(|effect| effect.health.is_some_and(|health| health.health_after == 0)),
        "real canonical native Murcion HP lethal"
    );
    let notice = runtime.next_source_creature_death().map_err(debug)?.expect(
        "CPP queues actual native Murcion death before physical drop, including registered source handlers",
    );
    assert_eq!(
        notice.enqueued_at_us(),
        Some(now.get()),
        "actual HP publication clock, no later synthetic event"
    );
    Ok(())
}

fn run_murcion_source_entry(published_world: bool, mushroom_fuse: bool) -> TestResult {
    run_murcion_source_entry_checks(
        PinnedActivitySourceRoom::Murcion,
        published_world,
        mushroom_fuse,
        false,
        false,
        false,
        false,
    )
}

fn run_murcion_source_entry_checks(
    source_room: PinnedActivitySourceRoom,
    published_world: bool,
    mushroom_fuse: bool,
    mushroom_damage: bool,
    source_notice: bool,
    mushroom_cutoff: bool,
    mushroom_retry: bool,
) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("srx3-murcion-native", move || {
        run(async |admin| {
            reset();
            // Crystal scripts/quests/rotten_blood_quest/rotten_murcion.lua:
            // first literal waiting position; the actual map supplies its collision.
            let (boss_key, origin) = match source_room {
                PinnedActivitySourceRoom::Murcion => (
                    "oteryn:creature.murcion",
                    MovementLocalPosition {
                        x: 32974,
                        y: 32365,
                        floor: 15,
                    },
                ),
                // Crystal rotten_ichgahal.lua:8 literal first waiting pad.
                PinnedActivitySourceRoom::Ichgahal => (
                    "oteryn:creature.ichgahal",
                    MovementLocalPosition {
                        x: 32978,
                        y: 32333,
                        floor: 15,
                    },
                ),
                _ => return Err("unsupported real source entry fixture".into()),
            };
            let destination_scene = SourceScene::new(source_room, origin)?;
            let published_scene = if published_world {
                Some(super::srx3_soul_war_world_fixture::source_world(origin)?)
            } else {
                None
            };
            let scene = published_scene.as_ref().unwrap_or(&destination_scene);
            let deps: Dependencies = scene.dependencies()?;
            let h = Harness::create_with_admission_seed(admin, "srx3_murcion_native", false, false)
                .await?;
            let evidence = super::house_transfer::RecoverySource::start(&h).await?;
            super::house_transfer::prepare_owned_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.evidence = &evidence.evidence;
            owner.active_generation = scene.controller.active();
            owner.rooms =
                crate::gameplay_transport::SeamRooms::new(Some(&scene.room), Some(&scene.room));
            // The Channel uses the authenticated whole World when requested;
            // its private destination still uses the exact pinned room capture.
            owner
                .activity_runtimes
                .register_source_room(source_room, &destination_scene.room)
                .map_err(|_| "register authentic Murcion destination capture")?;
            super::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
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
            owner.active_generation = scene.controller.active();
            owner.rooms =
                crate::gameplay_transport::SeamRooms::new(Some(&scene.room), Some(&scene.room));
            // The Channel uses the authenticated whole World when requested;
            // its private destination still uses the exact pinned room capture.
            owner
                .activity_runtimes
                .register_source_room(source_room, &destination_scene.room)
                .map_err(|_| "register authentic Murcion destination capture")?;
            owner
                .register_current_runtime()
                .await
                .map_err(|_| "register resealed actual source runtime")?;
            owner
                .activity_runtimes
                .remember(admitted)
                .map_err(|_| "remember resealed actual source connection")?;

            let instance = super::activity_runtime::allocate_ready_instance(&h, 213)
                .await
                .map_err(|error| format!("ready activity allocation: {error}"))?;
            let mut frames = Vec::new();
            let (_, reply, barrier) = super::boss_activity::exchange_capture(
                &owner,
                &mut admitted,
                boss_key,
                true,
                &mut frames,
            )
            .await?;
            assert!(
                matches!(reply, wire::BossActivityReply::Accepted { .. }),
                "actual Murcion source entry: {reply:?}"
            );
            assert!(barrier.is_some());
            let fence = admitted.item_fence.ok_or("actual Instance fence")?;
            assert_eq!(fence.runtime_scope, instance);
            assert!(matches!(instance, RuntimeScopeRefV1::Instance { .. }));
            let registered = owner
                .activity_runtimes
                .get(instance, fence.scope_ownership_generation.get())
                .ok_or("actual source owner")?;
            owner.runtime_view(&registered).drain_monster_melee().await;
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(2))
                .ok_or("owner clock")?;
            let view = owner.runtime_view(&registered);
            view.drain_monster_melee().await;
            let runtime = view.runtime.lock().await;
            let creatures = runtime.visible_entities().creatures;
            let count = |key: &[u8]| {
                creatures
                    .iter()
                    .filter(|c| runtime.matches_live_creature_identity(c.actor, key))
                    .count()
            };
            assert_eq!(count(boss_key.as_bytes()), 1);
            // murcion.lua:140-141 starts summonInterval at zero: the first
            // real think tops up four, then attempts the three random wave
            // placements. Game.createMonster may refuse an occupied cell.
            let mushrooms = count(b"oteryn:creature.mushroom");
            assert!(
                (4..=7).contains(&mushrooms),
                "source first think has four floor Mushrooms plus up to three admitted wave placements: {mushrooms}"
            );
            assert_eq!(
                count(b"oteryn:creature.elder_bloodjaw"),
                2,
                "source taint0 uses two real Bloodjaws"
            );
            assert!(
                creatures
                    .iter()
                    .all(|c| c.actor.runtime_scope() == instance)
            );
            let original_mushrooms = creatures
                .iter()
                .filter(|c| {
                    runtime.matches_live_creature_identity(c.actor, b"oteryn:creature.mushroom")
                })
                .map(|c| c.actor)
                .collect::<Vec<_>>();
            drop(runtime);
            drop(view);
            if source_notice {
                let view = owner.runtime_view(&registered);
                let mut runtime = view.runtime.lock().await;
                retain_murcion_native_lethal(
                    &mut runtime,
                    admitted.runtime_actor.ok_or("actual live caster")?,
                    admitted.game_session_id,
                    *fence.character_id.as_bytes(),
                    fence.character_lease_generation,
                    view.owner_now(),
                )?;
                drop(runtime);
                drop(view);
                drop(registered);
                drop(owner);
                drop(authority);
                drop(seal);
                return h.cleanup().await;
            }
            if mushroom_damage {
                let actor = admitted.runtime_actor.ok_or("actual Mushroom victim")?;
                let (center, route) = {
                    let view = owner.runtime_view(&registered);
                    let runtime = view.runtime.lock().await;
                    let start = runtime
                        .read_actor_position(actor)
                        .map_err(debug)?
                        .position();
                    let occupied = runtime
                        .visible_entities()
                        .creatures
                        .into_iter()
                        .map(|c| runtime.read_actor_position(c.actor).map(|p| p.position()))
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(debug)?;
                    let centers = original_mushrooms
                        .iter()
                        .map(|child| runtime.read_actor_position(*child).map(|p| p.position()))
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(debug)?;
                    let mut choices = Vec::new();
                    for center in &centers {
                        for dx in -1..=1 {
                            for dy in -1..=1 {
                                if dx == 0 && dy == 0 {
                                    continue;
                                }
                                let goal = MovementLocalPosition {
                                    x: center.x + dx,
                                    y: center.y + dy,
                                    floor: center.floor,
                                };
                                // Isolate one actual source fuse, preserving all other actors.
                                if centers.iter().any(|other| {
                                    other != center
                                        && (other.x - goal.x).abs().max((other.y - goal.y).abs())
                                            <= 1
                                }) {
                                    continue;
                                }
                                if let Ok(route) = super::srx3_primal_terminal::source_route(
                                    view.effective_movement_cells(),
                                    start,
                                    goal,
                                    &occupied,
                                    false,
                                ) {
                                    choices.push((*center, route));
                                }
                            }
                        }
                    }
                    choices
                        .into_iter()
                        .min_by_key(|(_, route)| route.len())
                        .ok_or("actual captured-map route beside an isolated Mushroom")?
                };
                for direction in route {
                    owner.clock_origin = owner
                        .clock_origin
                        .checked_sub(std::time::Duration::from_millis(500))
                        .ok_or("Mushroom victim pace clock")?;
                    let view = owner.runtime_view(&registered);
                    let (result, _) = view
                        .ordinary_paced_step(actor, admitted.game_session_id, direction, None, None)
                        .await;
                    assert_eq!(
                        result.disposition,
                        oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved,
                        "actual source-map movement beside Mushroom"
                    );
                }
                let view = owner.runtime_view(&registered);
                let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(
                    bundle,
                ) = view.effective_movement_cells().index()
                else {
                    return Err("actual Mushroom Bundle facts".into());
                };
                let facts = &bundle.map().facts;
                let cursor = facts
                    .creature_visuals_since(0)
                    .last()
                    .map_or(0, |(n, _)| *n);
                drop(view);
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(2))
                    .ok_or("Mushroom recipient first-think clock")?;
                owner.runtime_view(&registered).drain_monster_melee().await;
                let view = owner.runtime_view(&registered);
                let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(
                    bundle,
                ) = view.effective_movement_cells().index()
                else {
                    return Err("actual Mushroom Bundle facts".into());
                };
                let wire_center = crate::gameplay_transport::soul_war_scene::wire_position(center)
                    .ok_or("Mushroom source floor")?;
                let primed = bundle.map().facts.creature_visuals_since(cursor);
                assert_eq!(
                    primed
                        .iter()
                        .filter(|(_, cue)| matches!(cue,
                    crate::map::facts::CreatureVisualKind::Effect { position, effect: 15 }
                        if *position == wire_center))
                        .count(),
                    2,
                    "literal paired MagicGreen priming cues at actual Mushroom"
                );
                drop(view);
                if mushroom_cutoff {
                    // The adjacent actual source timer is1s; other original
                    // children retain their authentic18s deadlines. Death is
                    // enqueued after the first deadline, before the others.
                    owner.clock_origin = owner
                        .clock_origin
                        .checked_sub(std::time::Duration::from_millis(1100))
                        .ok_or("actual death cutoff clock")?;
                    let view = owner.runtime_view(&registered);
                    let cutoff = view.owner_now();
                    let mut runtime = view.runtime.lock().await;
                    retain_murcion_native_lethal(
                        &mut runtime,
                        actor,
                        admitted.game_session_id,
                        *fence.character_id.as_bytes(),
                        fence.character_lease_generation,
                        cutoff,
                    )?;
                    let selected = original_mushrooms
                        .iter()
                        .copied()
                        .find(|child| {
                            runtime
                                .read_actor_position(*child)
                                .is_ok_and(|p| p.position() == center)
                        })
                        .ok_or("actual primed source Mushroom")?;
                    let later = original_mushrooms
                        .iter()
                        .copied()
                        .filter(|child| *child != selected)
                        .collect::<Vec<_>>();
                    assert!(!later.is_empty(), "real later source timers");
                    drop(runtime);
                    drop(view);
                    owner.clock_origin = owner
                        .clock_origin
                        .checked_sub(std::time::Duration::from_secs(20))
                        .ok_or("real late observation clock")?;
                    eprintln!(
                        "SRX3_MUSHROOM_ACTUAL_SOURCE_DEATH_CUTOFF_REACHED cutoff_us={}",
                        cutoff.get()
                    );
                    owner.runtime_view(&registered).drain_monster_melee().await;
                    let view = owner.runtime_view(&registered);
                    let runtime = view.runtime.lock().await;
                    assert!(
                        !runtime.contains_live_creature(selected),
                        "older original1s source fuse must finish before deferred death"
                    );
                    assert!(
                        later
                            .iter()
                            .all(|child| runtime.contains_live_creature(*child)),
                        "future-at-death18s source fuses must not run merely because real now is late"
                    );
                    drop(runtime);
                    drop(view);
                    drop(registered);
                    drop(owner);
                    drop(authority);
                    drop(seal);
                    return h.cleanup().await;
                }
                if mushroom_retry {
                    owner
                        .runtime_view(&registered)
                        .states
                        .spell_states
                        .lock()
                        .await
                        .rotten_blood_callbacks
                        .pause_mushroom_after_player_commit_for_test();
                }
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_millis(1001))
                    .ok_or("Mushroom normal-damage fuse clock")?;
                owner.runtime_view(&registered).drain_monster_melee().await;
                let view = owner.runtime_view(&registered);
                let runtime = view.runtime.lock().await;
                let states = view.states.spell_states.lock().await;
                let receipts =
                    states.mushroom_damage_receipts_for_test(actor, admitted.game_session_id);
                assert!(
                    receipts
                        .iter()
                        .any(|(raw, receipt)| *raw > 0 && receipt.applied > 0),
                    "normal native Mushroom LifeDrain must commit through canonical Player vitals: {receipts:?}"
                );
                let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(
                    bundle,
                ) = view.effective_movement_cells().index()
                else {
                    return Err("actual Mushroom Bundle facts".into());
                };
                let cues = bundle.map().facts.creature_visuals_since(cursor);
                let retained = if mushroom_retry {
                    let progress = states
                        .rotten_blood_callbacks
                        .mushroom_progress_for_test()
                        .ok_or("actual Mushroom continuation after committed Player HP")?;
                    assert!(progress.cursor > 0 && progress.cursor <= progress.total);
                    assert_eq!(progress.players, vec![(actor, admitted.game_session_id)]);
                    assert!(progress.original_due <= view.owner_now().get());
                    assert_eq!(
                        runtime
                            .read_actor_position(progress.actor)
                            .map_err(debug)?
                            .position(),
                        center,
                        "the retained callback still names the original Mushroom"
                    );
                    assert!(runtime.contains_live_creature(progress.actor));
                    Some((progress.actor, receipts.clone()))
                } else {
                    None
                };
                for dy in -2_i32..=2 {
                    let width = if dy.abs() == 2 { 1 } else { 2 };
                    for dx in -width..=width {
                        let at = crate::gameplay_transport::soul_war_scene::wire_position(
                            MovementLocalPosition {
                                x: center.x + dx,
                                y: center.y + dy,
                                floor: center.floor,
                            },
                        )
                        .ok_or("source green-ring position")?;
                        assert!(
                            cues.iter().any(|(_, cue)| matches!(cue,
                            crate::map::facts::CreatureVisualKind::Effect { position, effect: 9 }
                                if *position == at)),
                            "source21-cell GreenRings mask at {at:?}"
                        );
                    }
                }
                drop(states);
                drop(runtime);
                drop(view);
                if let Some((original, receipts)) = retained {
                    owner.clock_origin = owner
                        .clock_origin
                        .checked_sub(std::time::Duration::from_millis(1))
                        .ok_or("Mushroom later retry clock")?;
                    owner.runtime_view(&registered).drain_monster_melee().await;
                    let view = owner.runtime_view(&registered);
                    let runtime = view.runtime.lock().await;
                    let states = view.states.spell_states.lock().await;
                    assert!(
                        states
                            .rotten_blood_callbacks
                            .mushroom_progress_for_test()
                            .is_none()
                    );
                    assert!(!runtime.contains_live_creature(original));
                    assert_eq!(
                        states.mushroom_damage_receipts_for_test(actor, admitted.game_session_id),
                        receipts,
                        "later retry keeps the exact canonical HP receipt of the captured Player"
                    );
                    let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(
                        bundle,
                    ) = view.effective_movement_cells().index()
                    else {
                        return Err("actual Mushroom retry Bundle facts".into());
                    };
                    let blast_only = crate::gameplay_transport::soul_war_scene::wire_position(
                        MovementLocalPosition {
                            x: center.x + 2,
                            y: center.y + 1,
                            floor: center.floor,
                        },
                    )
                    .ok_or("Mushroom blast-only cell")?;
                    assert_eq!(
                        bundle
                            .map()
                            .facts
                            .creature_visuals_since(cursor)
                            .iter()
                            .filter(|(_, cue)| matches!(cue,
                            crate::map::facts::CreatureVisualKind::Effect { position, effect: 9 }
                            if *position == blast_only))
                            .count(),
                        1,
                        "the retained21-cell blast is published once across owner retries"
                    );
                }
            }
            if mushroom_fuse {
                // Give the real spawned source actors their first reserved think,
                // then cross the literal18s fuse without injecting a callback.
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(2))
                    .ok_or("mushroom first-think clock")?;
                owner.runtime_view(&registered).drain_monster_melee().await;
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(20))
                    .ok_or("mushroom fuse clock")?;
                owner.runtime_view(&registered).drain_monster_melee().await;
                let runtime = registered.runtime.lock().await;
                for child in original_mushrooms {
                    assert!(
                        !runtime.matches_live_creature_identity(child, b"oteryn:creature.mushroom"),
                        "actual native Mushroom fuse must remove its original actor after executing source effects"
                    );
                }
            }
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    });
    eprintln!(
        "SRX3_MURCION_NATIVE_POSTGRES_CASE_EXECUTED duration={:?} passed={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn srx3_rotten_native_corpse_free_children_keep_normal_experience_postgres() -> TestResult {
    run_rotten_native_lethal_cases(&[
        // Current native source precedence selects Canary rotten_blood/mushroom.lua:5
        // (0XP), rather than Crystal20600XP. Pillar is Crystal-only20600XP.
        (65002, "oteryn:creature.pillar_of_dark_energy", 20600, 0),
        (65005, "oteryn:creature.spiteful_spitter", 24960, 0),
        (65001, "oteryn:creature.mushroom", 0, 0),
    ])
}

#[test]
fn srx3_rotten_actual_native_hunter_death_materializes_source_corpse_postgres() -> TestResult {
    run_rotten_native_lethal_cases(&[(65003, "oteryn:creature.bloated_man_maggot", 21570, 1)])
}

#[test]
fn srx3_soul_war_actual_ordinary_capacity48_death_postgres() -> TestResult {
    // Bony Sea Devil: exact native17328XP and pinned XML corpse33797 capacity48.
    run_rotten_native_lethal_cases(&[(65008, "oteryn:creature.bony_sea_devil", 17328, 1)])
}

#[test]
fn srx3_soul_war_actual_plain_child_corpse_materialization_postgres() -> TestResult {
    run_rotten_native_lethal_cases(&[
        (65004, "oteryn:creature.weeping_soul", 0, 1),
        (65005, "oteryn:creature.poor_soul", 0, 2),
        (65006, "oteryn:creature.aspect_of_power", 0, 3),
        (65007, "oteryn:creature.necromantic_focus", 0, 4),
    ])
}

#[test]
fn srx3_plain_corpse_retirement_uses_actual_source_lifetime_postgres() -> TestResult {
    run_rotten_native_lethal_cases_with_timer(
        &[(65004, "oteryn:creature.weeping_soul", 0, 1)],
        Some(Some(300_000)),
    )
}

#[test]
fn srx3_hateful_soul_plain_corpse_has_no_invented_expiry_postgres() -> TestResult {
    run_rotten_native_lethal_cases_with_timer(
        &[(65009, "oteryn:creature.hateful_soul", 0, 1)],
        Some(None),
    )
}

#[test]
fn srx3_symbol_of_hatred_actual_plain_corpse_postgres() -> TestResult {
    run_rotten_native_lethal_cases(&[(65010, "oteryn:creature.symbol_of_hatred", 0, 1)])
}

#[test]
fn srx3_mortal_essence_plain_corpse_enters_native_ground_custody_postgres() -> TestResult {
    run_rotten_native_lethal_case_checks(
        &[(65005, "oteryn:creature.poor_soul", 0, 1)],
        None,
        true,
        false,
    )
}

#[test]
fn srx3_symbol_native_corpse_actual_step_fuses_remorse_and_ground_removal_postgres() -> TestResult {
    run_rotten_native_lethal_case_checks(
        &[(65010, "oteryn:creature.symbol_of_hatred", 0, 1)],
        None,
        false,
        true,
    )
}

fn run_rotten_native_lethal_cases(cases: &[(u16, &'static str, u64, i64)]) -> TestResult {
    run_rotten_native_lethal_cases_with_timer(cases, None)
}

fn run_rotten_native_lethal_cases_with_timer(
    cases: &[(u16, &'static str, u64, i64)],
    check_timer: Option<Option<i64>>,
) -> TestResult {
    run_rotten_native_lethal_case_checks(cases, check_timer, false, false)
}

fn run_rotten_native_lethal_case_checks(
    cases: &[(u16, &'static str, u64, i64)],
    check_timer: Option<Option<i64>>,
    check_plain_ground: bool,
    check_source_step: bool,
) -> TestResult {
    // The existing qualification fixture registers one spawn source per scope.
    // Preserve that production resource guard: every species gets a fresh,
    // independently admitted scope/Character instead of raising its ceiling.
    if cases.len() > 1 {
        for &(source, key, xp, corpses) in cases {
            run_rotten_native_lethal_case_checks(
                &[(source, key, xp, i64::from(corpses > 0))],
                check_timer,
                check_plain_ground,
                check_source_step,
            )?;
        }
        return Ok(());
    }
    let cases = cases.to_vec();
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("srx3-rotten-child-xp", move || {
        run(async move |admin| {
            let scene = SourceScene::new(
                PinnedActivitySourceRoom::Murcion,
                MovementLocalPosition {
                    x: 32974,
                    y: 32365,
                    floor: 15,
                },
            )?;
            let deps = scene.dependencies()?;
            let h =
                Harness::create_with_admission_seed(admin, "srx3_rotten_child_xp", false, false)
                    .await?;
            let evidence = super::house_transfer::RecoverySource::start(&h).await?;
            super::house_transfer::prepare_owned_session(&h).await?;
            if check_plain_ground {
                // Imported historical equipped source backpack, before the
                // fresh recovery seal; every new pickup guard stays active.
                super::item_look::seed_main_backpack(&h, &[]).await?;
            }
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.evidence = &evidence.evidence;
            install_scene(&mut owner, &scene, PinnedActivitySourceRoom::Murcion)?;
            super::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
            if check_source_step {
                let catalogue =
                    oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                        .map_err(debug)?;
                owner.quest_catalogue = Some(std::sync::Arc::new(catalogue.catalogue().clone()));
            }
            let mut admitted = deps.admitted;
            prepare_player(&h, &owner, &mut admitted, 1999)
                .await
                .map_err(|error| format!("native lethal Character preparation: {error}"))?;
            // The source Channel fixture has a Player but has not run the
            // Instance runtime factory which installs native Creature policies.
            let native = owner
                .active_generation
                .and_then(|g| g.native_gameplay())
                .ok_or("native companion policy source")?;
            native
                .install_companion_policies(&mut *owner.runtime.lock().await)
                .map_err(|error| format!("native companion policy installation: {error:?}"))?;
            for &(source, key, expected_experience, expected_corpses) in &cases {
                let before: i64 = sqlx::query_scalar(
                    "SELECT total_experience FROM game_character_progression_state",
                )
                .fetch_one(&h.pool)
                .await?;
                let actor = admitted.runtime_actor.ok_or("actual native participant")?;
                let fence = admitted.item_fence.ok_or("current Character fence")?;
                let now = owner.owner_now();
                let command = crate::foundation::CommandRef::new(
                    admitted.game_session_id,
                    crate::foundation::CommandId::new(u64::from(source)).map_err(debug)?,
                );
                publish_native_fixture_player_source(&owner, fence, actor, command, now.get())
                    .await?;
                let child = {
                    use crate::foundation::runtime_actor_spell_types::{
                        OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect,
                        SpellOccurrenceBinding,
                    };
                    let mut runtime = owner.runtime.lock().await;
                    let pin = runtime.content_pin().clone();
                    // This qualification fixture supplies an actual source-map
                    // walkable tile; actor HP, identity and flags are native.
                    // It does not claim the encounter summon callback was run.
                    let position = [
                        (32973, 32365),
                        (32975, 32365),
                        (32974, 32364),
                        (32974, 32366),
                    ]
                    .into_iter()
                    .map(|(x, y)| MovementLocalPosition { x, y, floor: 15 })
                    .find(|at| {
                        scene.room.movement_cells().index().lookup(
                            scene.room.movement_cells().scope(),
                            crate::content::LogicalCell {
                                x: at.x,
                                y: at.y,
                                z: i32::from(at.floor),
                            },
                        ) == Ok(crate::content::CollisionClass::Walkable)
                    })
                    .ok_or("actual source adjacent walkable child tile")?;
                    let child = runtime
                        .realize_native_qualification_spawn_for_source(source, pin, key, position)
                        .map_err(|error| format!("native qualification child {key}: {error:?}"))?;
                    let native = owner
                        .active_generation
                        .and_then(|g| g.native_gameplay())
                        .ok_or("native reward owner")?;
                    assert_eq!(
                        native
                            .reward_table()
                            .encounter_facts(key)
                            .and_then(|f| f.experience),
                        Some(expected_experience)
                    );
                    let batch = OwnerCombatBatch {
                        caster: actor,
                        attacker: crate::foundation::CharacterId::decode(
                            fence.character_id.as_bytes(),
                        )
                        .map_err(debug)?,
                        current_lease_generation: fence.character_lease_generation,
                        command,
                        occurrence: SpellOccurrenceBinding {
                            id: format!("srx3-native-child:{source}"),
                            revisions: ["r1", "r1", "r1", "r1", "r1"].map(str::to_owned),
                        },
                        binding: format!("srx3-native-child:{source}").into_bytes(),
                        anchor: None,
                        now_us: Some(now),
                        now_ms: now.get() / 1000,
                        effects: vec![OwnerCombatEffect {
                            target: child.actor,
                            sub_ordinal: 0,
                            change: OwnerCombatChange::Damage {
                                target_atom: runtime
                                    .creature_spell_target_atom(child.actor)
                                    .map_err(debug)?,
                                magnitude: child.maximum_health,
                            },
                        }],
                        deferred: None,
                    };
                    let staged = runtime.stage_spell_batch(&batch).map_err(debug)?;
                    let receipt = runtime.commit_spell_batch(staged).map_err(debug)?;
                    assert!(
                        receipt
                            .effects
                            .iter()
                            .any(|e| e.health.is_some_and(|h| h.health_after == 0)),
                        "actual native child lethal HP"
                    );
                    let attack = owner.states.attack.lock().await;
                    crate::gameplay_transport::kill_reward::record_batch_kills(
                        &mut runtime,
                        crate::gameplay_transport::kill_reward::KillSink {
                            prey: None,
                            activity: &[],
                            queue: attack.kills(),
                            rewards: native.reward_table(),
                            charms: None,
                        },
                        &receipt,
                        now.get() / 1000,
                    );
                    child.actor
                };
                super::srx3_native_source_cast::drain_source_death(&owner, child, now).await?;
                assert_eq!(
                    owner.drain_kill_rewards(admitted.game_session_id).await,
                    crate::gameplay_transport::kill_reward::DrainOutcome::Drained
                );
                let after: i64 = sqlx::query_scalar(
                    "SELECT total_experience FROM game_character_progression_state",
                )
                .fetch_one(&h.pool)
                .await?;
                if expected_experience > 0 {
                    assert!(
                        after > before,
                        "actual native child {key} must commit its normalXP through the production reward owner"
                    );
                } else {
                    assert_eq!(after, before, "source child {key} has zeroXP");
                }
                assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_mint_receipts WHERE loot_purpose_key='CORPSE_MATERIALIZATION'").fetch_one(&h.pool).await?,expected_corpses,"source physical corpse count");
                if check_plain_ground {
                    let scope = owner.runtime.lock().await.binding().runtime_scope();
                    let ground = owner.root.read_item_loop_ground_for_scope(scope).await?;
                    assert!(
                        ground.iter().any(|item| item.item.definition.production_key
                            == "oteryn:item.tibia.i33891"),
                        "actual plain Mortal Essence corpse must be visible through the canonical native Ground owner for pickup/use"
                    );
                    assert!(
                        owner.runtime.lock().await.visible_corpses().is_empty(),
                        "plain Item body must not be mislabeled as an openable corpse container"
                    );
                    let actual = ground
                        .iter()
                        .find(|item| {
                            item.item.definition.production_key == "oteryn:item.tibia.i33891"
                        })
                        .ok_or("actual Mortal Essence Ground item")?;
                    assert!(
                        actual.source_plain_corpse && actual.field_collision.is_none(),
                        "plain corpse provenance is independent of spell-field collision"
                    );
                    let base = owner
                        .bundle_map()
                        .ok_or("actual source native map")?
                        .overlay
                        .base()
                        .clone();
                    let overlay = crate::gameplay_transport::loop_move::loop_ground(
                        &owner,
                        &base,
                        native.item_index().map(|index| index.as_ref()),
                    )
                    .await
                    .ok_or("actual native plain-body Ground overlay")?;
                    let (at, _) = overlay
                        .ground_entry(&actual.item.item_instance_id)
                        .ok_or("actual plain corpse must project into native map coordinates")?;
                    let map = owner.bundle_map().ok_or("current source native map")?;
                    let facts = overlay.facts(&map.facts);
                    let tile = crate::map::view::compose_tile(overlay.overlay(), &facts, at)
                        .map_err(debug)?
                        .ok_or("native plain-body tile")?;
                    assert!(tile.entries.iter().any(|entry|matches!(entry.source,crate::map::view::EntrySource::Ground(id) if id==actual.item.item_instance_id)&&entry.facts.pickupable),
                        "native map must expose the same physical Mortal Essence as a source-proved pickupable Item");
                    let picked = crate::gameplay_transport::loop_move::move_loop_item(
                        &owner,
                        actor,
                        crate::gameplay_transport::connection::UseCommand {
                            game_session_id: admitted.game_session_id,
                            command_id: 1,
                            item_fence: Some(fence),
                        },
                        crate::gameplay_transport::item_move::LoopMove {
                            item: crate::gameplay_transport::item_view::ItemKey::Instance(
                                actual.item.item_instance_id,
                            ),
                            count: None,
                            target: crate::gameplay_transport::item_move::LoopTarget::MainBackpack,
                        },
                    )
                    .await;
                    assert_eq!(
                        picked,
                        crate::gameplay_transport::item_move::LoopMoveResult::Committed,
                        "actual native command9 pickup must normalize the source runtime coordinates under the current owner"
                    );
                    let after_view = crate::gameplay_transport::loop_move::loop_ground(
                        &owner,
                        &base,
                        native.item_index().map(|index| index.as_ref()),
                    )
                    .await
                    .ok_or("native Ground view after actual pickup")?;
                    assert!(
                        after_view
                            .ground_entry(&actual.item.item_instance_id)
                            .is_none(),
                        "same native Item leaves Ground after known pickup"
                    );
                    assert_eq!(sqlx::query_scalar::<_, i64>(
                        "SELECT count(*) FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid")
                        .bind(actual.item.item_instance_id.as_slice()).bind(fence.character_id.as_bytes().as_slice())
                        .fetch_one(&h.pool).await?, 1, "same physical Item enters actual Character custody");
                    assert!(
                        !sqlx::query_scalar::<_, bool>(
                            "SELECT game_item_plain_corpse_source(encode($1,'hex')::uuid)"
                        )
                        .bind(actual.item.item_instance_id.as_slice())
                        .fetch_one(&h.pool)
                        .await?,
                        "pickup cannot retain original Ground provenance"
                    );
                }
                if check_source_step {
                    // Both Items below come from real native Symbol deaths. The
                    // current source Player starts with an explicit Torment
                    // predecessor through the existing live state owner.
                    {
                        let runtime = owner.runtime.lock().await;
                        let mut states = owner.states.spell_states.lock().await;
                        let next = states
                            .get(&runtime, actor, admitted.game_session_id)
                            .ok_or("current source Player")?
                            .stage_soul_war_torment_counter(24)
                            .ok_or("Torment predecessor24")?;
                        assert!(states.commit(&runtime, actor, admitted.game_session_id, next));
                    }
                    exercise_remorse_step(&h, &mut owner, &admitted, &scene, 1, 24).await?;
                    create_second_symbol_corpse(&owner, &admitted, &scene).await?;
                    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_mint_receipts WHERE loot_purpose_key='CORPSE_MATERIALIZATION'").fetch_one(&h.pool).await?,2,"both Remorse Items have actual native death/MINT predecessors");
                    exercise_remorse_step(&h, &mut owner, &admitted, &scene, 0, 0).await?;
                }
                if let Some(expected_lifetime) = check_timer {
                    let duration = sqlx::query_scalar::<_, Option<i64>>("SELECT game_corpse_retirement_deadline(item_instance_id)-materialized_at FROM game_item_mint_receipts WHERE loot_purpose_key='CORPSE_MATERIALIZATION'").fetch_one(&h.pool).await?;
                    assert_eq!(
                        duration, expected_lifetime,
                        "actual source {key} corpse lifetime; no invented60s fallback"
                    );
                }
            }
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    });
    eprintln!(
        "SRX3_ROTTEN_NATIVE_CHILD_XP_POSTGRES_CASE_EXECUTED duration={:?} passed={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

/// This test drives the ordinary Player movement producer. The old current
/// Ground read is retained solely to replay the exact already committed writer
/// after the physical Item has gone; it never grants a fresh mutation.
async fn exercise_remorse_step(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &crate::gameplay_transport::connection::AdmittedSession,
    scene: &SourceScene,
    expected_remorse: i64,
    expected_torment: u8,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native source")?;
    let actor = admitted.runtime_actor.ok_or("actual Player")?;
    let scope = owner.runtime.lock().await.binding().runtime_scope();
    let ground = owner.root.read_item_loop_ground_for_scope(scope).await?;
    let actual = ground
        .iter()
        .find(|item| item.item.definition.production_key == "oteryn:item.tibia.i33792")
        .ok_or("actual Symbol plain corpse33792 must exist before source STEP")?;
    assert!(actual.source_plain_corpse);
    let raw = &actual.item.ground.spatial_position;
    if raw.len() != 10 {
        return Err("actual source corpse coordinate envelope".into());
    }
    let target_position = MovementLocalPosition {
        x: i32::from_be_bytes(raw[0..4].try_into()?),
        y: i32::from_be_bytes(raw[4..8].try_into()?),
        floor: i16::from_be_bytes(raw[8..10].try_into()?),
    };
    let (before, target) = {
        let runtime = owner.runtime.lock().await;
        (
            runtime.read_actor_position(actor).map_err(debug)?,
            crate::durability::spell_items_abi::SpellGroundTarget::for_native_tile_read(
                &scene.room,
                &runtime,
                target_position,
            )
            .map_err(debug)?,
        )
    };
    use oteryn_protocol_oteryn::world_spatial::StepDirection;
    let direction = match (
        target_position.x - before.position().x,
        target_position.y - before.position().y,
    ) {
        (0, -1) => StepDirection::North,
        (1, 0) => StepDirection::East,
        (0, 1) => StepDirection::South,
        (-1, 0) => StepDirection::West,
        _ => return Err("real corpse must be on actual adjacent source tile".into()),
    };
    let permit = owner
        .spell_lane_permit()
        .await
        .ok_or("actual source Step lane")?;
    let original = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "original Step Character fence")?
        .ok_or("live Step session")?;
    let read = owner
        .root
        .read_standing_player_tile(
            &permit,
            owner.character,
            owner.holder,
            original.item_fence(),
            native.source_digest(),
            target,
        )
        .await
        .map_err(debug)?;
    drop(permit);
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_millis(500))
        .ok_or("actual Remorse step pace")?;
    let (step, _) = owner
        .ordinary_paced_step(actor, admitted.game_session_id, direction, None, None)
        .await;
    assert_eq!(
        step.disposition,
        oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved
    );
    let (movement, selected, state_after) = {
        let runtime = owner.runtime.lock().await;
        let movement = runtime.read_actor_position(actor).map_err(debug)?;
        assert_eq!(movement.position(), target_position);
        assert!(movement.last_translation_revision() > before.last_translation_revision());
        let selected=crate::durability::quest_state::soul_war_source::step::SoulWarSourceStep::from_standing_read(
            &runtime,native,&scene.room,actor,admitted.game_session_id,movement,owner.reset_epoch,&read)
            .ok_or("retained original SQL Ground witness")?.into_iter()
            .find(|input|input.item_instance_id()==actual.item.item_instance_id).ok_or("same original33792 proof")?;
        let states = owner.states.spell_states.lock().await;
        let state = states
            .get(&runtime, actor, admitted.game_session_id)
            .ok_or("current Player after actual STEP")?
            .clone();
        assert_eq!(state.soul_war_torment_count(), expected_torment);
        assert_eq!(
            state.soul_war_torment_icon_count(),
            (expected_torment != 0).then_some(u16::from(expected_torment))
        );
        (movement, selected, state)
    };
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(original.character_id.as_bytes().as_slice()).bind(oteryn_gameplay_quests::quest::soul_war::source_items::REMORSE).fetch_one(&h.pool).await?,expected_remorse);
    assert!(
        !owner
            .root
            .read_item_loop_ground_for_scope(scope)
            .await?
            .iter()
            .any(|item| item.item.item_instance_id == actual.item.item_instance_id),
        "ordinary STEP already consumed the exact actual Remorse Item"
    );
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_quest_callback_consumptions WHERE item_instance_id=encode($1,'hex')::uuid AND transition_key='oteryn:quest-transition/soul-war/source-items/remorse'").bind(actual.item.item_instance_id.as_slice()).fetch_one(&h.pool).await?,1);
    let permit = owner
        .spell_lane_permit()
        .await
        .ok_or("source replay lane")?;
    let context = selected.context();
    let mut slot = owner
        .revision_sequencer
        .acquire(original.character_id)
        .await;
    let replay = slot
        .commit_soul_war_source_action(
            owner.root,
            owner.character,
            owner.holder,
            original,
            crate::durability::quest_state::soul_war_source::SoulWarSourceAction::Step(selected),
            owner
                .quest_catalogue
                .as_ref()
                .ok_or("source catalogue")?
                .clone(),
            Some((&permit, context)),
        )
        .await?;
    assert!(
        matches!(
            replay,
            crate::durability::quest_state::QuestTransitionOutcome::AlreadyCommitted(_)
        ),
        "same immutable source receipt reconciles before looking for the removed Item: {replay:?}"
    );
    drop(slot);
    {
        let mut runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        crate::gameplay_transport::spite_step::stage(
            owner,
            &mut runtime,
            &mut states,
            actor,
            admitted.game_session_id,
            movement,
        )
        .map_err(|_| "completed raw movement replay")?;
    }
    assert!(owner.resolve_spite_steps(&permit).await);
    let runtime = owner.runtime.lock().await;
    assert_eq!(
        owner
            .states
            .spell_states
            .lock()
            .await
            .get(&runtime, actor, admitted.game_session_id),
        Some(&state_after),
        "writer/raw movement replay does not repeat the counter/icon effect"
    );
    eprintln!("SRX3_ACTUAL_REMORSE_STEP_PAIRED_WRITER_REACHED remorse={expected_remorse}");
    Ok(())
}

async fn create_second_symbol_corpse(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &crate::gameplay_transport::connection::AdmittedSession,
    scene: &SourceScene,
) -> TestResult {
    use crate::foundation::runtime_actor_spell_types::{
        OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
    };
    let actor = admitted.runtime_actor.ok_or("actual Player")?;
    let fence = admitted.item_fence.ok_or("actual Item fence")?;
    let now = owner.owner_now();
    let command = crate::foundation::CommandRef::new(
        admitted.game_session_id,
        crate::foundation::CommandId::new(65011).map_err(debug)?,
    );
    publish_native_fixture_player_source(owner, fence, actor, command, now.get()).await?;
    let child = {
        let mut runtime = owner.runtime.lock().await;
        let origin = runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        let position = [(-1, 0), (1, 0), (0, -1), (0, 1)]
            .into_iter()
            .map(|(x, y)| MovementLocalPosition {
                x: origin.x + x,
                y: origin.y + y,
                floor: origin.floor,
            })
            .find(|at| {
                runtime.native_summon_cell_free(*at)
                    && scene.room.movement_cells().index().lookup(
                        scene.room.movement_cells().scope(),
                        crate::content::LogicalCell {
                            x: at.x,
                            y: at.y,
                            z: i32::from(at.floor),
                        },
                    ) == Ok(crate::content::CollisionClass::Walkable)
            })
            .ok_or("second actual adjacent native tile")?;
        // Reuse the existing native encounter owner; do not install a second
        // qualification-spawn source or alter its resource ceiling.
        let child = runtime
            .spawn_arena_opponent(
                "oteryn:creature.symbol_of_hatred",
                position,
                |current, at| {
                    Some(
                        current.native_summon_cell_free(at)
                            && scene.room.movement_cells().index().lookup(
                                scene.room.movement_cells().scope(),
                                crate::content::LogicalCell {
                                    x: at.x,
                                    y: at.y,
                                    z: i32::from(at.floor),
                                },
                            ) == Ok(crate::content::CollisionClass::Walkable),
                    )
                },
            )
            .map_err(debug)?;
        let maximum = runtime
            .companion_snapshot(child)
            .map_err(debug)?
            .maximum_health;
        let batch = OwnerCombatBatch {
            caster: actor,
            attacker: crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                .map_err(debug)?,
            current_lease_generation: fence.character_lease_generation,
            command,
            occurrence: SpellOccurrenceBinding {
                id: "srx3-native-remorse-second".into(),
                revisions: ["r1", "r1", "r1", "r1", "r1"].map(str::to_owned),
            },
            binding: b"srx3-native-remorse-second".to_vec(),
            anchor: None,
            now_us: Some(now),
            now_ms: now.get() / 1000,
            effects: vec![OwnerCombatEffect {
                target: child,
                sub_ordinal: 0,
                change: OwnerCombatChange::Damage {
                    target_atom: runtime.creature_spell_target_atom(child).map_err(debug)?,
                    magnitude: maximum,
                },
            }],
            deferred: None,
        };
        let staged = runtime.stage_spell_batch(&batch).map_err(debug)?;
        let receipt = runtime.commit_spell_batch(staged).map_err(debug)?;
        assert!(
            receipt
                .effects
                .iter()
                .any(|effect| effect.health.is_some_and(|health| health.health_after == 0))
        );
        let attack = owner.states.attack.lock().await;
        crate::gameplay_transport::kill_reward::record_batch_kills(
            &mut runtime,
            crate::gameplay_transport::kill_reward::KillSink {
                prey: None,
                activity: &[],
                queue: attack.kills(),
                rewards: owner
                    .active_generation
                    .and_then(|g| g.native_gameplay())
                    .ok_or("native rewards")?
                    .reward_table(),
                charms: None,
            },
            &receipt,
            now.get() / 1000,
        );
        child
    };
    super::srx3_native_source_cast::drain_source_death(owner, child, now).await?;
    assert_eq!(
        owner.drain_kill_rewards(admitted.game_session_id).await,
        crate::gameplay_transport::kill_reward::DrainOutcome::Drained
    );
    Ok(())
}

#[test]
fn srx3_primal_actual_instance_publishes_current_hazard_source_icon_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("srx3-primal-source-icon", || {
        run(async |admin| {
            reset();
            let scene = SourceScene::new(
                PinnedActivitySourceRoom::Primal,
                MovementLocalPosition {
                    x: 33548,
                    y: 32752,
                    floor: 14,
                },
            )?;
            let deps = scene.dependencies()?;
            let h =
                Harness::create_with_admission_seed(admin, "srx3_primal_source_icon", false, false)
                    .await?;
            let evidence = super::house_transfer::RecoverySource::start(&h).await?;
            super::house_transfer::prepare_owned_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.evidence = &evidence.evidence;
            install_scene(&mut owner, &scene, PinnedActivitySourceRoom::Primal)?;
            super::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
            let mut admitted = deps.admitted;
            admitted.continuity.selected_capabilities=SelectedCapabilities::select(
                BUNDLE_WORLD_OFFERED_CAPABILITIES,
                &[4,6,7,10,11,15,18,23,92,wire::CAPABILITY_BOSS_ENCOUNTER_V1,
                  oteryn_protocol_oteryn::world_spatial_entities::CAPABILITY_NATIVE_CREATURE_ICONS_V1],
            ).ok_or("actual native creature icon capabilities")?;
            prepare_player(&h, &owner, &mut admitted, 1999).await?;
            let instance = super::activity_runtime::allocate_ready_instance(&h, 215).await?;
            let mut frames = Vec::new();
            let (_, reply, barrier) = super::boss_activity::exchange_capture(
                &owner,
                &mut admitted,
                "oteryn:creature.the_primal_menace",
                true,
                &mut frames,
            )
            .await?;
            assert!(
                matches!(reply, wire::BossActivityReply::Accepted { .. }),
                "actual Primal entry: {reply:?}"
            );
            assert!(barrier.is_some());
            let fence = admitted.item_fence.ok_or("actual Primal fence")?;
            assert_eq!(fence.runtime_scope, instance);
            let registered = owner
                .activity_runtimes
                .get(instance, fence.scope_ownership_generation.get())
                .ok_or("actual Primal runtime")?;
            let view = owner.runtime_view(&registered);
            let actor = admitted.runtime_actor.ok_or("actual Primal Player")?;
            let (selected, frame, position, effective) = {
                let runtime = view.runtime.lock().await;
                (
                    runtime.hazard_level(actor, admitted.game_session_id),
                    runtime.hazard_world_frame(actor, admitted.game_session_id),
                    runtime
                        .read_actor_position(actor)
                        .map_err(debug)?
                        .position(),
                    oteryn_gameplay_combat::hazard::effective_level(
                        &runtime,
                        actor,
                        admitted.game_session_id,
                    ),
                )
            };
            let selected = selected.ok_or("actual re-admitted Primal Hazard selection")?;
            let observed = view
                .observe_channel_entities(actor)
                .await
                .ok_or("actual Primal entity projection")?;
            let player = observed
                .entities
                .iter()
                .find(|entity| entity.identity == actor.placement_identity())
                .ok_or("actual visible Primal Player")?;
            assert!(
                player
                    .source_icons
                    .iter()
                    .any(|icon| icon.category == 0 && icon.id == 22 && icon.count == selected),
                "actual private Primal source icon: selected={selected}, frame={frame}, at={position:?}, effective={effective:?}, icons={:?}",
                player.source_icons
            );
            drop(view);
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    });
    eprintln!(
        "SRX3_PRIMAL_SOURCE_ICON_POSTGRES_CASE_EXECUTED duration={:?} passed={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

/// The normal owner batch must carry the same current Player DATA as a real
/// native cast; source health callbacks may never receive a fabricated vocation.
async fn publish_native_fixture_player_source(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    actor: crate::foundation::ExactActorRef,
    command: crate::foundation::CommandRef,
    now: u64,
) -> TestResult {
    let permit = owner
        .spell_lane_permit()
        .await
        .ok_or("actual source lane")?;
    let active = owner
        .active_generation
        .ok_or("actual source active generation")?;
    let native = active
        .native_gameplay()
        .ok_or("actual source native policy")?;
    let room = owner
        .effective_rooms()
        .gate_field_step_ingress()
        .ok_or("actual source map")?;
    let mut runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let mut context = (&mut *runtime, &*states, &permit);
    owner
        .root
        .try_issue_semantic_pass()?
        .run_with_context(&mut context, move |holder, deadline, context| {
            Box::pin(async move {
                use crate::durability::spell_item_transaction::{
                    assert_spell_item_authority_in_transaction, begin_spell_owner_transaction,
                };
                let (runtime, states, permit) = context;
                let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
                let authority = assert_spell_item_authority_in_transaction(
                    &mut tx,
                    permit,
                    owner.root,
                    owner.character,
                    owner.holder,
                    &fence,
                    command,
                    native.source_digest(),
                )
                .await
                .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                crate::gameplay_transport::soul_war_callbacks::publish_command_source(
                    &mut tx, &authority, active, room, runtime, states, actor, now,
                )
                .await?;
                tx.rollback().await?;
                Ok(())
            })
        })
        .await?;
    Ok(())
}
