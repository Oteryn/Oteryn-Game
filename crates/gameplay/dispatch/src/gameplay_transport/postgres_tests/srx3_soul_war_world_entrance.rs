//! Current published World hunt STEP must consume the actual Quest access flag.
use super::fixture::{Harness, debug, run};
use super::{TestResult, reset};
use crate::foundation::MovementLocalPosition;

#[test]
fn srx3_soul_war_actual_world_hunt_entry_rejects_missing_quest_access_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("srx3-soul-war-world-entry", || {
        run(async |admin| {
            reset();
            // Crystal201189f moveevent-soul_war_entrances.lua3/24–30.
            let scene = super::srx3_soul_war_world_fixture::source_world(MovementLocalPosition {
                x: 33615,
                y: 31421,
                floor: 10,
            })?;
            let deps = scene.dependencies()?;
            let h = Harness::create_with_admission_seed(
                admin,
                "srx3_soul_war_world_entry",
                false,
                false,
            )
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
            super::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
            let mut admitted = deps.admitted;
            super::srx3_activity_source_fixture::prepare_player(&h, &owner, &mut admitted, 1999)
                .await?;
            let actor = admitted.runtime_actor.ok_or("actual World player")?;
            let before = owner
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?;
            let context = scene
                .controller
                .active()
                .ok_or("actual World active")?
                .qualify_published_world_context(&scene.room, &scene.pin)
                .ok_or("actual current World frame")?;
            let frame = context.source_frame();
            let position = before.position();
            assert_eq!(
                frame.source_position((position.x, position.y, position.floor)),
                Some([33615, 31421, 10])
            );
            // Keep the actual published collision predecessor visible when a
            // map/Item projection refuses entry before the source MoveEvent.
            let cells = scene.room.movement_cells();
            for source in [[33615, 31421, 10], [33615, 31422, 10]] {
                let (x, y, floor) = frame
                    .physical_position(source)
                    .ok_or("entry diagnostic frame")?;
                eprintln!(
                    "SRX3_HUNT_ENTRY_CURRENT_CELL source={source:?} collision={:?}",
                    cells.index().lookup(
                        cells.scope(),
                        crate::content::LogicalCell {
                            x,
                            y,
                            z: i32::from(floor),
                        }
                    )
                );
            }
            let (moved, _) = owner
                .ordinary_paced_step(
                    actor,
                    admitted.game_session_id,
                    oteryn_protocol_oteryn::world_spatial::StepDirection::South,
                    Some(before),
                    None,
                )
                .await;
            assert_eq!(
                moved.disposition,
                oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved,
                "actual current World movement must reach the source fog portal"
            );
            let now = owner
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?;
            let position = now.position();
            assert_eq!(
                frame.source_position((position.x, position.y, position.floor)),
                Some([33615, 31421, 10]),
                "actual hunt entrance returns an access-less current Player to the captured from-position"
            );
            assert_eq!(
                now.facing(),
                Some(crate::foundation::MovementFacing::South),
                "source teleportTo(fromPosition,true) preserves the committed South STEP facing"
            );
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })?;
    eprintln!(
        "SRX3_SOUL_WAR_WORLD_ENTRY_POSTGRES_CASE_EXECUTED duration={:?}",
        started.elapsed()
    );
    Ok(())
}
