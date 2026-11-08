//! Real source Think → ordinary physical lethal → existing core-death prefix.
//! Observations never create callbacks or death notices. Only the nonlethal1HP
//! injury is a bounded fixture precondition, as in the full W10 terminal case.
use super::fixture::{Harness, debug, run};
use super::srx3_activity_source_fixture::{SourceScene, install_scene, prepare_player};
use super::{Dependencies, TestResult, reset};
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES;
use oteryn_protocol_oteryn::boss_encounter as wire;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

#[test]
fn srx3_primal_actual_source_deferred_tasks_precede_physical_death_cutoff_postgres() -> TestResult {
    run_actual_primal_core_cutoff(false, false)
}

#[test]
fn srx3_primal_actual_queued_pod_expiry_precedes_core_without_sweeping_future_postgres()
-> TestResult {
    run_actual_primal_core_cutoff(true, false)
}

#[test]
fn srx3_primal_actual_normal_map_sql_wait_releases_runtime_owner_postgres() -> TestResult {
    run_actual_primal_core_cutoff(true, true)
}

fn run_actual_primal_core_cutoff(pod_expiry_case: bool, lock_normal_read: bool) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run(
        "srx3-primal-death-cutoff",
        move || {
            run(async move |admin| {
                reset();
                // Expensive native qualification precedes short-lived admission proof.
                let scene = SourceScene::new(
                    PinnedActivitySourceRoom::Primal,
                    MovementLocalPosition {
                        x: 33548,
                        y: 32752,
                        floor: 14,
                    },
                )?;
                let deps: Dependencies = scene.dependencies()?;
                let h =
                    Harness::create_with_admission_seed(admin, "srx3_primal_cutoff", false, false)
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
                let (offered, scope, generation) = {
                    let runtime = owner.runtime.lock().await;
                    let offered = crate::gameplay_transport::world_required_capabilities::with_native_map_presentation_offer(
                    BUNDLE_WORLD_OFFERED_CAPABILITIES, owner.active_generation,
                    owner.effective_rooms().gate_wire_position(), runtime.content_pin(),
                );
                    (
                        offered,
                        runtime.binding().runtime_scope(),
                        runtime.binding().scope_generation().get(),
                    )
                };
                owner.required_capabilities = crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
                &[11,18,30], scope, generation, "policy-1", offered,
            ).map_err(debug)?;
                let supported = [4,6,7,10,11,15,18,23,30,92,wire::CAPABILITY_BOSS_ENCOUNTER_V1,385,
                oteryn_protocol_oteryn::world_spatial_entities::CAPABILITY_NATIVE_OBJECT_APPEARANCE_V1];
                admitted.continuity.selected_capabilities =
                    crate::gameplay_transport::connection::fresh_capability_selection(
                        &owner, &supported,
                    )
                    .await
                    .ok_or("actual source native capability selection")?;
                owner
                    .required_capabilities
                    .authenticate_resume_support(
                        &supported,
                        admitted.continuity.selected_capabilities,
                    )
                    .map_err(debug)?;
                prepare_player(&h, &owner, &mut admitted, 1999).await?;
                let instance = super::activity_runtime::allocate_ready_instance(&h, 219).await?;
                let (_, reply, barrier) = super::boss_activity::exchange_capture(
                    &owner,
                    &mut admitted,
                    "oteryn:creature.the_primal_menace",
                    true,
                    &mut Vec::new(),
                )
                .await?;
                assert!(matches!(reply, wire::BossActivityReply::Accepted { .. }));
                assert!(barrier.is_some());
                let fence = admitted.item_fence.ok_or("actual transferred fence")?;
                assert_eq!(fence.runtime_scope, instance);
                let actor = admitted.runtime_actor.ok_or("actual player")?;
                let registered = owner
                    .activity_runtimes
                    .get(instance, fence.scope_ownership_generation.get())
                    .ok_or("actual current Primal Instance")?;
                let (boss, target, route) = {
                    let view = owner.runtime_view(&registered);
                    let runtime = view.runtime.lock().await;
                    let creature = runtime
                        .visible_entities()
                        .creatures
                        .into_iter()
                        .find(|row| {
                            runtime.matches_live_creature_identity(
                                row.actor,
                                b"oteryn:creature.the_primal_menace",
                            )
                        })
                        .ok_or("actual source Primal")?;
                    let position = runtime
                        .read_actor_position(creature.actor)
                        .map_err(debug)?
                        .position();
                    let occupied = runtime
                        .visible_entities()
                        .creatures
                        .into_iter()
                        .map(|row| {
                            runtime
                                .read_actor_position(row.actor)
                                .map(|at| at.position())
                        })
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(debug)?;
                    let route = super::srx3_primal_terminal::source_route(
                        view.effective_movement_cells(),
                        runtime
                            .read_actor_position(actor)
                            .map_err(debug)?
                            .position(),
                        position,
                        &occupied,
                        true,
                    )?;
                    (
                        creature.actor,
                        EntityRef {
                            identity: creature.actor.placement_identity(),
                            generation: creature.generation,
                        },
                        route,
                    )
                };
                for direction in route {
                    owner.clock_origin = owner
                        .clock_origin
                        .checked_sub(std::time::Duration::from_secs(1))
                        .ok_or("paced owner clock")?;
                    let (step, _) = owner
                        .runtime_view(&registered)
                        .ordinary_paced_step(actor, admitted.game_session_id, direction, None, None)
                        .await;
                    assert_eq!(
                        step.disposition,
                        oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved
                    );
                }
                // The actual native CreatureCheck/Think starts its own clock. No
                // source task or wave state is seeded by this test.
                for _ in 0..5 {
                    let native_drain: std::pin::Pin<
                        Box<dyn std::future::Future<Output = ()> + '_>,
                    > = Box::pin(async {
                        owner.runtime_view(&registered).drain_monster_melee().await;
                    });
                    native_drain.await;
                    if owner
                        .runtime_view(&registered)
                        .states
                        .spell_states
                        .lock()
                        .await
                        .primal_callbacks
                        .source_started_for_test()
                    {
                        break;
                    }
                    owner.clock_origin = owner
                        .clock_origin
                        .checked_sub(std::time::Duration::from_secs(1))
                        .ok_or("native wake clock")?;
                }
                assert!(
                    owner
                        .runtime_view(&registered)
                        .states
                        .spell_states
                        .lock()
                        .await
                        .primal_callbacks
                        .source_started_for_test(),
                    "prerequisite: actual reserved native onThink initialized the source"
                );
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(if pod_expiry_case {
                        21
                    } else {
                        11
                    }))
                    .ok_or("source first wave clock")?;
                let native_drain: std::pin::Pin<Box<dyn std::future::Future<Output = ()> + '_>> =
                    Box::pin(async {
                        owner.runtime_view(&registered).drain_monster_melee().await;
                    });
                native_drain.await;
                let mut old_pod = None;
                let mut future_pod = None;
                if pod_expiry_case {
                    let pod_due = owner
                        .runtime_view(&registered)
                        .states
                        .spell_states
                        .lock()
                        .await
                        .primal_callbacks
                        .pods
                        .iter()
                        .map(|pod| pod.due_us())
                        .min()
                        .ok_or("prerequisite: actual source Think scheduled Pod creates")?;
                    let delta = pod_due
                        .checked_add(100_000)
                        .ok_or("pod due bounds")?
                        .checked_sub(owner.owner_now().get())
                        .ok_or("actual Pod create window")?;
                    owner.clock_origin = owner
                        .clock_origin
                        .checked_sub(std::time::Duration::from_micros(delta))
                        .ok_or("pod create clock")?;
                    owner
                        .runtime_view(&registered)
                        .drain_primal_pod_creates_through(pod_due)
                        .await
                        .map_err(|_| "actual source Pod callback owner")?;
                    let first = owner
                        .runtime_view(&registered)
                        .states
                        .spell_states
                        .lock()
                        .await
                        .primal_callbacks
                        .pod_execution_deadlines_for_test()
                        .into_iter()
                        .find(|row| row.3)
                        .ok_or("prerequisite: first actual Pod create committed")?;
                    assert_eq!(first.2, first.1.checked_add(4_000_000).ok_or("source TTL")?);
                    if lock_normal_read {
                        let mut blocker = h.pool.begin().await?;
                        let blocker_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
                            .fetch_one(&mut *blocker)
                            .await?;
                        sqlx::query("SELECT i.item_instance_id FROM game_item_instances i JOIN game_primal_pod_callbacks p USING(item_instance_id) WHERE p.source_transaction_id=encode($1,'hex')::uuid FOR UPDATE OF i")
                            .bind(first.0.as_slice()).fetch_one(&mut *blocker).await?;
                        let epoch_before = crate::gameplay_transport::primal_callbacks::normal_map_read_epoch_for_test();
                        let view = owner.runtime_view(&registered);
                        {
                            let runtime = view.runtime.lock().await;
                            let states = view.states.spell_states.lock().await;
                            eprintln!(
                                "SRX3_PRIMAL_NORMAL_MAP_READ_PREREQUISITES owner_now={} native_due={:?} next_pass={} boss_hp={:?} player_hp={:?} census={} epoch={epoch_before} blocker_pid={blocker_pid}",
                                view.owner_now().get(),
                                states.monster_melee.creature_ai.earliest_due(),
                                states.next_monster_ai_pass_us,
                                runtime
                                    .companion_snapshot_including_dead(boss)
                                    .ok()
                                    .map(|snapshot| snapshot.health),
                                states
                                    .get(&runtime, actor, admitted.game_session_id)
                                    .map(|player| player.health),
                                runtime.positioned_actor_census().map_err(debug)?.len()
                            );
                        }
                        // pg_stat_activity truncates at configured1kB. Match the
                        // exact visible TILE_ROWS prefix, not its FROM at byte1959.
                        let (_, observed) = tokio::join!(
                            Box::pin(view.drain_monster_melee())
                                as std::pin::Pin<Box<dyn std::future::Future<Output = ()> + '_>>,
                            Box::pin(async {
                                let observation: TestResult = async {
                                    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(8);
                                    loop {
                                        let blocked: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid)) AND wait_event_type='Lock' AND query LIKE 'SELECT i.item_instance_id::text,i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity,i.state_revision::text,%')")
                                            .bind(blocker_pid).fetch_one(&h.pool).await?;
                                        if blocked {
                                            eprintln!("SRX3_PRIMAL_MAP_SQL_BLOCK_OBSERVED blocker_pid={blocker_pid} epoch_before={epoch_before} epoch_now={}",crate::gameplay_transport::primal_callbacks::normal_map_read_epoch_for_test());
                                            if crate::gameplay_transport::primal_callbacks::normal_map_read_epoch_for_test() <= epoch_before {
                                                return Err("prerequisite: generic pre-Primal map read blocked before actual Primal map-read boundary".into());
                                            }
                                            break;
                                        }
                                        if tokio::time::Instant::now() >= deadline {
                                            eprintln!("SRX3_PRIMAL_MAP_SQL_WAIT_MISSING blocker_pid={blocker_pid} epoch_before={epoch_before} epoch_now={}",crate::gameplay_transport::primal_callbacks::normal_map_read_epoch_for_test());
                                            return Err("prerequisite: actual Primal TILE_ROWS did not enter exact external row-lock wait".into());
                                        }
                                        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                                    }
                                    eprintln!("SRX3_PRIMAL_NORMAL_MAP_SQL_WAIT_REACHED source={:?} blocker_pid={blocker_pid}", first.0);
                                    let acquired = tokio::time::timeout(std::time::Duration::from_millis(250), view.runtime.lock()).await;
                                    if acquired.is_err() {
                                        return Err("actual Primal map SQL wait must release the canonical runtime owner BEFORE external SQL unblock".into());
                                    }
                                    drop(acquired);
                                    let still_blocked: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid)) AND wait_event_type='Lock' AND query LIKE 'SELECT i.item_instance_id::text,i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity,i.state_revision::text,%')")
                                        .bind(blocker_pid).fetch_one(&h.pool).await?;
                                    assert!(still_blocked, "runtime acquisition must precede SQL unblock, not follow source read timeout");
                                    Ok(())
                                }.await;
                                // Release before returning a prerequisite/target failure,
                                // so the genuine producer always gets to unwind.
                                blocker.rollback().await?;
                                observation
                            })
                                as std::pin::Pin<
                                    Box<dyn std::future::Future<Output = TestResult> + '_>,
                                >
                        );
                        observed?;
                        drop(view);
                        drop(registered);
                        drop(owner);
                        drop(authority);
                        drop(seal);
                        return h.cleanup().await;
                    }
                    // Wait real SQL receipt+4s independently of native owner time.
                    wait_actual_pod_sql_due(&h.pool, first.0).await?;
                    owner
                        .runtime_view(&registered)
                        .drain_primal_pod_creates_through(pod_due)
                        .await
                        .map_err(|_| "actual source Pod callback owner")?;
                    let second = owner
                        .runtime_view(&registered)
                        .states
                        .spell_states
                        .lock()
                        .await
                        .primal_callbacks
                        .pod_execution_deadlines_for_test()
                        .into_iter()
                        .find(|row| row.0 != first.0 && row.3)
                        .ok_or("prerequisite: actual second Pod create at later execution")?;
                    assert!(second.2 > first.2);
                    owner
                        .runtime_view(&registered)
                        .drain_primal_pod_expiries()
                        .await;
                    assert!(
                        owner
                            .runtime_view(&registered)
                            .states
                            .spell_states
                            .lock()
                            .await
                            .primal_callbacks
                            .queued_pod_expiry_sources_for_test()
                            .contains(&first.0),
                        "prerequisite: actual SQL-due first Pod expiry queued before physical lethal"
                    );
                    old_pod = Some(first);
                    future_pod = Some(second);
                }
                let scheduled = owner
                    .runtime_view(&registered)
                    .states
                    .spell_states
                    .lock()
                    .await
                    .primal_callbacks
                    .deferred_original_deadlines_for_test()
                    .map_err(|_| "actual retained callback owner busy")?;
                let first_due_ms = scheduled
                    .iter()
                    .map(|(_, due)| *due)
                    .min()
                    .ok_or("prerequisite: real native Think scheduled callbacks")?;
                let target_us = if pod_expiry_case {
                    owner
                        .owner_now()
                        .get()
                        .checked_add(250_000)
                        .ok_or("actual expiry core window")?
                } else {
                    first_due_ms
                        .checked_mul(1000)
                        .and_then(|due| due.checked_add(250_000))
                        .ok_or("source cutoff window")?
                };
                let current_us = owner.owner_now().get();
                let until_window = target_us
                    .checked_sub(current_us)
                    .ok_or("prerequisite: callback window elapsed before physical attack")?;
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_micros(until_window))
                    .ok_or("actual owner cutoff clock")?;
                {
                    let view = owner.runtime_view(&registered);
                    let mut runtime = view.runtime.lock().await;
                    assert_eq!(
                        runtime
                            .companion_snapshot(boss)
                            .map_err(debug)?
                            .maximum_health,
                        400_000
                    );
                    assert!(
                        runtime
                            .borrow_combat_death()
                            .current_session_damage_shares(boss)
                            .map_err(debug)?
                            .is_empty()
                    );
                    runtime.set_test_creature_health(boss, 1).map_err(debug)?;
                    assert!(!runtime.retains_corpse_projection(boss));
                }
                let view = owner.runtime_view(&registered);
                assert_eq!(
                    view.select_attack_target(actor, admitted.game_session_id, 934, Some(target))
                        .await,
                    oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
                );
                // The normal physical owner creates the lethal core queue entry;
                // this test never constructs a death or an OwnerDamage grant.
                view.refresh_attack_weapon(actor, admitted.game_session_id)
                    .await;
                view.drain_auto_attacks().await;
                {
                    let runtime = view.runtime.lock().await;
                    assert_eq!(
                        runtime
                            .companion_snapshot_including_dead(boss)
                            .map_err(debug)?
                            .health,
                        0,
                        "prerequisite: actual ordinary physical hit must be lethal in the observed source window"
                    );
                    assert!(runtime.source_creature_is_registered(boss).map_err(debug)?);
                }
                let (core_actor, core_death, core_observed_at) = view
                    .states
                    .attack
                    .lock()
                    .await
                    .kills()
                    .core_head_for_test()
                    .ok_or("prerequisite: actual physical core-death queue head")?;
                assert_eq!(core_actor, boss);
                let cutoff = core_observed_at
                    .ok_or("prerequisite: actual physical core-death owner clock")?
                    .get();
                let core_counts_before = view.states.attack.lock().await.kills().counts();
                assert_eq!(
                    core_counts_before.0, 1,
                    "actual core prefix contains this one physical death"
                );
                assert_eq!(
                    core_counts_before.1, 0,
                    "no core settlement reserved before target drain"
                );
                let before = view
                    .states
                    .spell_states
                    .lock()
                    .await
                    .primal_callbacks
                    .deferred_original_deadlines_for_test()
                    .map_err(|_| "actual retained callbacks after lethal")?;
                let (older, future): (Vec<_>, Vec<_>) = before
                    .iter()
                    .copied()
                    .partition(|(_, due)| due.saturating_mul(1000) <= cutoff);
                assert!(
                    !older.is_empty() && (!future.is_empty() || pod_expiry_case),
                    "prerequisite: actual source callbacks must bracket the real physical notice; cutoff={cutoff},work={before:?}"
                );
                if !pod_expiry_case {
                    assert!(
                        future
                            .iter()
                            .any(|(_, due)| due.saturating_mul(1000) > cutoff)
                    );
                }
                if let (Some(old), Some(future)) = (old_pod, future_pod) {
                    assert!(
                        old.2 <= cutoff && future.2 > cutoff,
                        "actual expiry/core chronological split"
                    );
                    wait_actual_pod_sql_due(&h.pool, future.0).await?;
                }
                drop(view);
                let latest = future
                    .iter()
                    .map(|(_, due)| due.checked_mul(1000).ok_or("future callback clock"))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .chain(future_pod.map(|row| row.2))
                    .max()
                    .ok_or("future actual callback or expiry")?;
                let now = owner.owner_now().get();
                let late_target = latest
                    .checked_add(1_000_000)
                    .ok_or("late real callback drain")?;
                let overdue = if pod_expiry_case {
                    late_target.saturating_sub(now)
                } else {
                    late_target
                        .checked_sub(now)
                        .ok_or("prerequisite: future source deadline already passed")?
                };
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_micros(overdue))
                    .ok_or("late source owner clock")?;
                let view = owner.runtime_view(&registered);
                eprintln!(
                    "SRX3_PRIMAL_REAL_DEATH_CUTOFF_REACHED core_death={core_death:?} core_counts={core_counts_before:?} cutoff={cutoff} real_now={} older={older:?} future={future:?}",
                    view.owner_now().get()
                );
                let mut settled = false;
                for _ in 0..32 {
                    let _ = view.drain_kill_rewards(admitted.game_session_id).await;
                    let after = view
                        .states
                        .spell_states
                        .lock()
                        .await
                        .primal_callbacks
                        .deferred_original_deadlines_for_test()
                        .map_err(|_| "retained source callback observation")?;
                    assert!(
                        after.iter().all(|work| before.contains(work)),
                        "the due-only core-death bridge cannot create fresh Think callbacks: before={before:?},after={after:?}"
                    );
                    for expected in &future {
                        assert!(
                            after.contains(expected),
                            "the real death-head bridge must retain original future work despite late execution: {expected:?},after={after:?}"
                        );
                    }
                    let executions = view
                        .states
                        .spell_states
                        .lock()
                        .await
                        .primal_callbacks
                        .pod_execution_deadlines_for_test();
                    if let Some(expected) = future_pod {
                        assert!(
                            executions.contains(&expected),
                            "future expiry keeps exact first-invocation stamp across retry"
                        );
                        let (completion, sql_due): (i16,bool) = sqlx::query_as("SELECT completion_kind,due_unix_ms<=floor(extract(epoch FROM statement_timestamp())*1000)::bigint FROM game_primal_pod_callbacks WHERE source_transaction_id=encode($1,'hex')::uuid")
                        .bind(expected.0.as_slice()).fetch_one(&h.pool).await?;
                        assert!(sql_due, "prerequisite: future native expiry also SQL-ready");
                        assert_eq!(
                            completion, 0,
                            "future SQL-due expiry cannot overtake actual core cutoff"
                        );
                    }
                    if older
                        .iter()
                        .all(|old| !after.iter().any(|current| current.0 == old.0))
                        && old_pod.is_none_or(|old| !executions.iter().any(|row| row.0 == old.0))
                    {
                        settled = true;
                        break;
                    }
                    let attack = view.states.attack.lock().await;
                    let current = attack.kills().core_head_for_test();
                    assert_eq!(
                        current,
                        Some((core_actor, core_death, core_observed_at)),
                        "core settlement cannot overtake genuinely older Primal work; counts={:?}",
                        attack.kills().counts()
                    );
                }
                assert!(
                    settled,
                    "actual core-death caller must drain its genuinely older Primal tasks before settlement"
                );
                if let Some(old) = old_pod {
                    let (completion, acknowledged): (i16,bool) = sqlx::query_as("SELECT completion_kind,effect_acknowledged FROM game_primal_pod_callbacks WHERE source_transaction_id=encode($1,'hex')::uuid")
                    .bind(old.0.as_slice()).fetch_one(&h.pool).await?;
                    assert_eq!(
                        (completion, acknowledged),
                        (2, true),
                        "older queued real Pod removal/native attempt/ACK completes before core"
                    );
                }
                if pod_expiry_case {
                    let mut core_finished = false;
                    for _ in 0..32 {
                        let outcome = view.drain_kill_rewards(admitted.game_session_id).await;
                        let executions = view
                            .states
                            .spell_states
                            .lock()
                            .await
                            .primal_callbacks
                            .pod_execution_deadlines_for_test();
                        if let Some(expected) = future_pod {
                            assert!(
                                executions.contains(&expected),
                                "future expiry remains exact while actual core settles"
                            );
                            let completion: i16 = sqlx::query_scalar("SELECT completion_kind FROM game_primal_pod_callbacks WHERE source_transaction_id=encode($1,'hex')::uuid")
                                .bind(expected.0.as_slice()).fetch_one(&h.pool).await?;
                            assert_eq!(
                                completion, 0,
                                "settlement retry cannot sweep future expiry"
                            );
                        }
                        let attack = view.states.attack.lock().await;
                        match attack.kills().core_head_for_test() {
                            None => {
                                assert_eq!(
                                    attack.kills().counts(),
                                    (0, 0, 0),
                                    "actual core queue and reservations drained"
                                );
                                assert!(matches!(
                                    outcome,
                                    crate::gameplay_transport::kill_reward::DrainOutcome::Drained
                                ));
                                core_finished = true;
                                break;
                            }
                            Some(current) => assert_eq!(
                                current,
                                (core_actor, core_death, core_observed_at),
                                "retry retains exact original core prefix"
                            ),
                        }
                    }
                    assert!(
                        core_finished,
                        "actual core must progress after older Pod native effect and ACK; no permanent prefix deadlock"
                    );
                }
                if let Some((current_actor, current_death, current_clock)) =
                    view.states.attack.lock().await.kills().core_head_for_test()
                {
                    assert_eq!(current_actor, core_actor);
                    assert_eq!(
                        current_death, core_death,
                        "timer progress cannot replace the canonical core death head"
                    );
                    assert_eq!(
                        current_clock, core_observed_at,
                        "retry preserves actual lethal owner clock"
                    );
                }
                drop(view);
                drop(registered);
                drop(owner);
                drop(authority);
                drop(seal);
                h.cleanup().await
            })
        },
    );
    eprintln!(
        "SRX3_PRIMAL_REAL_CUTOFF_POSTGRES_CASE_EXECUTED duration={:?} passed={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

async fn wait_actual_pod_sql_due(pool: &sqlx::PgPool, source: [u8; 16]) -> TestResult {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let due: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_primal_pod_callbacks WHERE source_transaction_id=encode($1,'hex')::uuid AND due_unix_ms<=floor(extract(epoch FROM statement_timestamp())*1000)::bigint)")
            .bind(source.as_slice()).fetch_one(pool).await?;
        if due {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(
                "actual exact Pod receipt did not become SQL-due within bounded wait".into(),
            );
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}
