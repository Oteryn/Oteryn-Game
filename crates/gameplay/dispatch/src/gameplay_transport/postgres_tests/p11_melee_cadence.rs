use super::full_world_support::*;
use oteryn_protocol_oteryn::attack::{FightMode, FightModes};

#[test]
fn p11_due_melee_swing_lands_with_busy_bounty_holder() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("p11-melee-cadence", || scenario(false))
}

#[test]
fn p11_step_onto_the_tile_of_the_killed_creature() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("p11-step-after-kill", || scenario(true))
}

fn scenario(step_after_kill: bool) -> TestResult {
    world_scenario!(
        "p11_melee_cadence",
        |h, request, seal, authority, world_id, native, world, cells, room, pin, deps,
         actor, session, owner, fence, admitted| {
            let _ = (&request, &admitted);
            let destination = if step_after_kill {
                MovementLocalPosition { x: ENTRY.0 + 1, y: ENTRY.1, floor: 7 }
            } else {
                MovementLocalPosition { x: ENTRY.0 - 1, y: ENTRY.1 + 1, floor: 7 }
            };
            let (target, entity, before) = {
                let mut runtime = owner.runtime.lock().await;
                runtime
                    .realize_native_qualification_spawn(
                        pin.clone(),
                        "oteryn:creature.rat",
                        destination,
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
                    runtime.live_creature_health(target.actor).ok_or("initial health")?,
                )
            };
            assert_eq!(
                owner.set_fight_modes(actor, session, FightModes {
                    fight_mode: FightMode::Offensive,
                    ..FightModes::DEFAULT
                }).await,
                AttackIntentDisposition::Ok
            );
            assert_eq!(
                owner.select_attack_target(actor, session, 1, Some(entity)).await,
                AttackIntentDisposition::Ok
            );
            if step_after_kill {
                assert_eq!(
                    owner.paced_step(actor, session, StepDirection::East).await.0.disposition,
                    StepDisposition::Blocked,
                    "the living rat occupies the next tile"
                );
                for _ in 0..32 {
                    owner.refresh_attack_weapon(actor, session).await;
                    owner.drain_auto_attacks().await;
                    if !owner.runtime.lock().await.contains_live_creature(target) {
                        break;
                    }
                    owner.clock_origin = owner.clock_origin.checked_sub(Duration::from_secs(3))
                        .ok_or("owner clock")?;
                }
                assert!(!owner.runtime.lock().await.contains_live_creature(target));
                assert!(matches!(owner.drain_kill_rewards(session).await, DrainOutcome::Drained));
                assert!(owner.runtime.lock().await.visible_corpses().iter().any(|corpse| {
                    (corpse.position.x, corpse.position.y, corpse.position.floor)
                        == (destination.x, destination.y, destination.floor)
                }));
                let (reply, _) = owner.paced_step(actor, session, StepDirection::East).await;
                assert_eq!(reply.disposition, StepDisposition::Moved, "{reply:?}");
                assert_eq!(position(&owner, actor).await?, destination);
            } else {
            owner.refresh_attack_weapon(actor, session).await;
            h.root.await_ready().await?;
            let holder = h.root.try_issue_semantic_pass()?;
            owner.drain_auto_attacks_at(owner.owner_now()).await;
            let after = owner.runtime.lock().await.live_creature_health(target).unwrap_or(0);
            assert!(after < before, "the due swing missed its tick: {before} HP to {after} HP");
            drop(holder);
            }
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        }
    )
}
