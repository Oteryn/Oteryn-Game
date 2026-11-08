//! Real source Think → ordinary physical lethal → existing source-death arbiter.
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
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("srx3-primal-death-cutoff", || {
        run(async |admin| {
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
            let h = Harness::create_with_admission_seed(admin, "srx3_primal_cutoff", false, false)
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
                .authenticate_resume_support(&supported, admitted.continuity.selected_capabilities)
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
                owner.runtime_view(&registered).drain_monster_melee().await;
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
                .checked_sub(std::time::Duration::from_secs(11))
                .ok_or("source first wave clock")?;
            owner.runtime_view(&registered).drain_monster_melee().await;
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
            let target_us = first_due_ms
                .checked_mul(1000)
                .and_then(|due| due.checked_add(250_000))
                .ok_or("source cutoff window")?;
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
            // The normal physical owner creates the lethal receipt and FIFO
            // notice; this test never constructs a death or an OwnerDamage grant.
            view.refresh_attack_weapon(actor, admitted.game_session_id)
                .await;
            view.drain_auto_attacks().await;
            let notice = {
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
                let notice = runtime
                    .next_source_creature_death()
                    .map_err(debug)?
                    .ok_or("actual physical source-death notice")?;
                assert_eq!(notice.actor(), boss);
                notice
            };
            let cutoff = notice
                .enqueued_at_us()
                .ok_or("actual committed lethal owner clock")?;
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
                !older.is_empty() && !future.is_empty(),
                "prerequisite: actual source callbacks must bracket the real physical notice; cutoff={cutoff},work={before:?}"
            );
            assert!(
                future
                    .iter()
                    .any(|(_, due)| due.saturating_mul(1000) > cutoff)
            );
            drop(view);
            let latest = future
                .iter()
                .map(|(_, due)| *due)
                .max()
                .ok_or("future actual callback")?
                .checked_mul(1000)
                .ok_or("future callback clock")?;
            let now = owner.owner_now().get();
            let overdue = latest
                .checked_add(1_000_000)
                .ok_or("late real callback drain")?
                .checked_sub(now)
                .ok_or("prerequisite: future source deadline already passed")?;
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_micros(overdue))
                .ok_or("late source owner clock")?;
            let view = owner.runtime_view(&registered);
            eprintln!(
                "SRX3_PRIMAL_REAL_DEATH_CUTOFF_REACHED cutoff={cutoff} real_now={} older={older:?} future={future:?}",
                view.owner_now().get()
            );
            let mut settled = false;
            for _ in 0..32 {
                view.drain_monster_melee().await;
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
                    "the due-only source-death bridge cannot create fresh Think callbacks: before={before:?},after={after:?}"
                );
                for expected in &future {
                    assert!(
                        after.contains(expected),
                        "the real death-head bridge must retain original future work despite late execution: {expected:?},after={after:?}"
                    );
                }
                if older
                    .iter()
                    .all(|old| !after.iter().any(|current| current.0 == old.0))
                {
                    settled = true;
                    break;
                }
            }
            assert!(
                settled,
                "actual source-death caller must drain its genuinely older Primal tasks instead of blocking forever"
            );
            let runtime = view.runtime.lock().await;
            if let Some(current) = runtime.next_source_creature_death().map_err(debug)? {
                assert_eq!(
                    current.death_key(),
                    notice.death_key(),
                    "timer progress cannot replace the canonical source death head"
                );
            }
            drop(runtime);
            drop(view);
            drop(registered);
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    });
    eprintln!(
        "SRX3_PRIMAL_REAL_CUTOFF_POSTGRES_CASE_EXECUTED duration={:?} passed={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
