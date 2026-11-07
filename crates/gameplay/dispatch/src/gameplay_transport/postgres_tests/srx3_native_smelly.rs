//! Real selected native callback and its500ms physical Player HP consumer.
use super::srx3_activity_source_fixture::{SourceScene, install_scene, prepare_player};
use super::{
    TestResult,
    fixture::{Harness, debug, run},
};
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::foundation::MovementLocalPosition;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

#[test]
fn srx3_smelly_actual_native_callback_damages_top_player_after500ms_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("srx3-native-smelly", || {
        run(async |admin| {
            super::reset();
            // Relative source area uses an actual authenticated source map; no
            // fabricated Frame or retagged native catalogue is used for the caster.
            let kind = PinnedActivitySourceRoom::BrainHead;
            let scene = SourceScene::new(
                kind,
                MovementLocalPosition {
                    x: 31955,
                    y: 32325,
                    floor: 10,
                },
            )?;
            let deps = scene.dependencies()?;
            let h = Harness::create_with_admission_seed(admin, "srx3_native_smelly", false, false)
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
            install_scene(&mut owner, &scene, kind)?;
            super::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
            let mut admitted = deps.admitted;
            prepare_player(&h, &owner, &mut admitted, 1999).await?;
            let player = admitted.runtime_actor.ok_or("real native Player")?;
            let caster = {
                let mut runtime = owner.runtime.lock().await;
                scene
                    .controller
                    .active()
                    .and_then(|active| active.native_gameplay())
                    .ok_or("actual native Smelly profile registry")?
                    .install_companion_policies(&mut runtime)
                    .map_err(debug)?;
                let at = MovementLocalPosition {
                    x: 31954,
                    y: 32325,
                    floor: 10,
                };
                runtime
                    .spawn_arena_opponent("oteryn:creature.smelly_cheese", at, |_, at| {
                        Some(
                            scene.room.movement_cells().index().lookup(
                                scene.room.movement_cells().scope(),
                                crate::content::LogicalCell {
                                    x: at.x,
                                    y: at.y,
                                    z: i32::from(at.floor),
                                },
                            ) == Ok(crate::content::CollisionClass::Walkable),
                        )
                    })
                    .map_err(debug)?
            };
            let mut selected = false;
            for _ in 0..64 {
                owner.drain_monster_melee().await;
                {
                    let states = owner.states.spell_states.lock().await;
                    if let Some(report) = states
                        .monster_melee
                        .creature_ai
                        .last_report_for_test(caster)
                    {
                        selected = report.proposals.iter().any(|p| {
                            p.ability.key == "oteryn:ability.creature.smelly_cheese.berserk_bridge"
                        });
                    }
                }
                if selected {
                    break;
                }
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(2))
                    .ok_or("native schedule clock")?;
            }
            assert!(
                selected,
                "actual native schedule must select the source parent callback"
            );
            // Real physical position-owner fixture: the caster moves west after
            // casting. Hero remains in the retained original3x3 area but is outside
            // the caster's new3x3, so current-centre implementations fail this case.
            {
                let mut runtime = owner.runtime.lock().await;
                let original = runtime.read_actor_position(caster).map_err(debug)?;
                let next = MovementLocalPosition {
                    x: 31953,
                    y: 32325,
                    floor: 10,
                };
                assert_eq!(
                    scene.room.movement_cells().index().lookup(
                        scene.room.movement_cells().scope(),
                        crate::content::LogicalCell {
                            x: next.x,
                            y: next.y,
                            z: i32::from(next.floor)
                        }
                    ),
                    Ok(crate::content::CollisionClass::Walkable)
                );
                runtime
                    .borrow_movement_position()
                    .commit_creature_relocation(caster, original, next)
                    .map_err(debug)?;
            }
            let before = {
                let runtime = owner.runtime.lock().await;
                let states = owner.states.spell_states.lock().await;
                states
                    .get(&runtime, player, admitted.game_session_id)
                    .ok_or("actual Player HP beforepulse")?
                    .vitals()
                    .health
            };
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_millis(501))
                .ok_or("native500ms callback clock")?;
            owner.drain_monster_melee().await;
            let after = {
                let runtime = owner.runtime.lock().await;
                let states = owner.states.spell_states.lock().await;
                states
                    .get(&runtime, player, admitted.game_session_id)
                    .ok_or("actual Player HP afterpulse")?
                    .vitals()
                    .health
            };
            assert!(
                after < before,
                "selected source Smelly500ms callback must commit actual Player HP, before={before} after={after}"
            );
            assert!(
                before - after <= 800,
                "source400..800 Earth draw uses canonical mitigation and HP owner"
            );
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    });
    eprintln!(
        "SRX3_SMELLY_NATIVE_POSTGRES_CASE_EXECUTED duration={:?} passed={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
