//! Soul War portal movement on the published World with native Quest state.
use super::fixture::{Harness, debug, id, run};
use super::{TestResult, reset};
use crate::foundation::MovementLocalPosition;
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
use oteryn_protocol_oteryn::zones_nologout::ServerMessage;

#[test]
fn soulwarmv_reward_portal_step_reads_megalomania_completion_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    portal_steps(
        "soulwarmv_reward_portal",
        [33621, 31417, 10],
        StepDirection::North,
        &[
            (
                250,
                31,
                false,
                [33621, 31417, 10],
                Some("Only warriors who defeated Goshnar's Megalomania can access this area."),
            ),
            (249, 0, true, [33621, 31411, 10], None),
        ],
    )
}

pub(super) type PortalTrial = (u32, i16, bool, [i32; 3], Option<&'static str>);

pub(super) fn portal_steps(
    label: &'static str,
    origin: [i32; 3],
    direction: StepDirection,
    scenarios: &'static [PortalTrial],
) -> TestResult {
    super::kill_reward_e2e::scenario_thread::run(label, move || {
        run(async |admin| {
            reset();
            let [x, y, floor] = origin;
            let scene = super::srx3_soul_war_world_fixture::source_world(MovementLocalPosition {
                x,
                y,
                floor: floor.try_into()?,
            })?;
            let active = scene.controller.active().ok_or("published World")?;
            let context = active
                .qualify_published_world_context(&scene.room, &scene.pin)
                .ok_or("published World frame")?;
            let frame = context.source_frame();
            for &(level, bosses, completed, destination, message) in scenarios {
                let deps = scene.dependencies()?;
                let h =
                    Harness::create_with_admission_seed(admin.clone(), label, false, false).await?;
                // Import existing native progression, then drive the live World STEP.
                let mut historical = h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica")
                    .execute(&mut *historical)
                    .await?;
                sqlx::query("INSERT INTO game_character_soul_war(character_id,teleport_access,defeated_bosses,first_taint_at,megalomania_completed,final_reward_claimed) VALUES(encode($1,'hex')::uuid,true,$2,CASE WHEN $2=0 THEN NULL ELSE floor(extract(epoch FROM clock_timestamp()))::bigint END,$3,false)")
                    .bind(id(41).as_slice())
                    .bind(bosses)
                    .bind(completed)
                    .execute(&mut *historical)
                    .await?;
                historical.commit().await?;
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
                owner.active_generation = Some(active);
                owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&scene.room));
                super::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
                let mut admitted = deps.admitted;
                super::srx3_activity_source_fixture::prepare_player(
                    &h,
                    &owner,
                    &mut admitted,
                    level,
                )
                .await?;
                let actor = admitted.runtime_actor.ok_or("actual World player")?;
                let before = owner
                    .runtime
                    .lock()
                    .await
                    .read_actor_position(actor)
                    .map_err(debug)?;
                let session = admitted.game_session_id;
                let generation = admitted.continuity.connection_generation;
                assert!(owner.states.server_messages.join(session, generation));
                let (moved, _) = owner
                    .ordinary_paced_step(
                        actor,
                        admitted.game_session_id,
                        direction,
                        Some(before),
                        None,
                    )
                    .await;
                let (at, observation) = {
                    let runtime = owner.runtime.lock().await;
                    let at = runtime
                        .read_actor_position(actor)
                        .map_err(debug)?
                        .position();
                    (at, owner.wire_observation(&runtime, at))
                };
                let notice = owner.states.server_messages.take(session, generation);
                drop(owner);
                drop(authority);
                drop(seal);
                h.cleanup().await?;
                assert_eq!(moved.disposition, StepDisposition::Moved);
                assert_eq!(
                    frame.source_position((at.x, at.y, at.floor)),
                    Some(destination),
                    "level={level}, miniboss flags={bosses:#07b}, completed={completed}"
                );
                assert!(observation.is_some());
                assert_eq!(moved.moved_to, observation);
                assert_eq!(
                    notice,
                    message.map(|text| ServerMessage::Advance(text.into()))
                );
            }
            Ok(())
        })
    })
}
