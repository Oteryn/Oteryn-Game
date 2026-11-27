use super::full_world_support::*;
use crate::combat_attack::AttackConstants;
use oteryn_protocol_oteryn::attack::{ChaseMode, FightMode, FightModes};
use std::future::{Future, poll_fn};
use std::sync::atomic::Ordering;
use std::task::Poll;

#[test]
fn adjacent_chase_knight_swings_at_every_attack_interval() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("w13-melee-cadence", || scenario(0))
}

#[test]
fn ordinary_combat_tick_does_no_durable_work_with_many_held_targets() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("w13-melee-budget", || scenario(64))
}

fn scenario(extra_actors: usize) -> TestResult {
    world_scenario!(
        "w13_melee_cadence",
        extra_actors + 4,
        crate::spell::Vocation::Knight,
        200,
        |h,
         request,
         seal,
         authority,
         world_id,
         native,
         world,
         cells,
         room,
         pin,
         deps,
         actor,
         session,
         owner,
         fence,
         admitted| {
            let _ = (request, world_id, world, &mut admitted);
            let target_cell = MovementLocalPosition {
                x: ENTRY.0 - 1,
                y: ENTRY.1 + 1,
                floor: 7,
            };
            let (target, entity) = {
                let mut runtime = owner.runtime.lock().await;
                runtime
                    .realize_native_qualification_spawn(
                        pin.clone(),
                        "oteryn:creature.braindeath",
                        target_cell,
                    )
                    .map_err(debug)?;
                let visible = runtime.visible_entities();
                let target = visible.creatures.first().ok_or("stationary target")?;
                (
                    target.actor,
                    EntityRef {
                        identity: target.actor.placement_identity(),
                        generation: target.generation,
                    },
                )
            };
            let modes = FightModes {
                chase: ChaseMode::Chase,
                fight_mode: FightMode::Offensive,
                ..FightModes::DEFAULT
            };
            assert_eq!(
                owner.set_fight_modes(actor, session, modes).await,
                AttackIntentDisposition::Ok
            );
            assert_eq!(
                owner
                    .select_attack_target(actor, session, 1, Some(entity))
                    .await,
                AttackIntentDisposition::Ok
            );
            // Other admitted placements have a held target but have yet to
            // load a weapon. Their missing enrichment must not gate this attacker.
            {
                let mut runtime = owner.runtime.lock().await;
                let mut attack = owner.states.attack.lock().await;
                for slot in 0..extra_actors {
                    let peer =
                        GameSessionId::decode(&id(120 + u8::try_from(slot)?)).map_err(debug)?;
                    let reserved = runtime.reserve_fresh_session(peer).map_err(debug)?;
                    let placed = runtime.commit_fresh_session(reserved).map_err(debug)?;
                    runtime
                        .initialize_first_entry_position(placed)
                        .map_err(debug)?;
                    assert_eq!(
                        attack.set_target(
                            &runtime,
                            Some(&room),
                            placed,
                            peer,
                            owner.owner_now(),
                            Some(entity),
                            1
                        ),
                        AttackIntentDisposition::Ok
                    );
                }
            }

            owner.refresh_attack_weapon(actor, session).await;
            h.root.await_ready().await?;
            let holder = h.root.try_issue_semantic_pass()?;
            let interval = AttackConstants::checked_in()
                .ok_or("attack constants")?
                .attack_interval_micros();
            let start_us = owner.owner_now().get();
            let mut health = owner
                .runtime
                .lock()
                .await
                .companion_snapshot(target)
                .map_err(debug)?
                .health;
            tokio::time::pause();
            let start = tokio::time::Instant::now();
            for turn in 0..if extra_actors == 0 { 6 } else { 2 } {
                let now = start_us + u64::try_from(start.elapsed().as_micros())?;
                owner
                    .states
                    .test_owner_time
                    .store(now + 1, Ordering::Relaxed);
                tokio::task::yield_now().await;
                let mut tick = std::pin::pin!(owner.tick_actor_combat(actor, session));
                // Work budget is one poll with zero external waits, even with
                // every ready holder checked out. Check that a real swing ran.
                let result = poll_fn(|cx| Poll::Ready(tick.as_mut().poll(cx))).await;
                let Poll::Ready(Some(state)) = result else {
                    return Err(format!(
                        "combat tick yielded before completing turn {turn} for {} actors",
                        extra_actors + 1
                    )
                    .into());
                };
                assert_eq!(state.target, Some(entity));
                assert_eq!(state.modes, modes);
                let attack = owner.states.attack.lock().await;
                let held = attack
                    .party_attack_state(actor, session)
                    .ok_or("held attack")?;
                assert_eq!(
                    held.next_due(interval).map(|due| due.get()),
                    Some(now + interval),
                    "skipped due swing on turn {turn}"
                );
                drop(attack);
                let snapshot = owner
                    .runtime
                    .lock()
                    .await
                    .companion_snapshot(target)
                    .map_err(debug)?;
                assert!(
                    snapshot.health < health,
                    "turn {turn}: stationary target HP stayed at {health}"
                );
                health = snapshot.health;
                tokio::time::advance(Duration::from_micros(interval)).await;
            }
            tokio::time::resume();
            drop(holder);
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        }
    )
}
