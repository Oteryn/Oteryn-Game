use super::fixture::{Harness, debug, run};
use super::{TestResult, prepare_current_session};
use crate::foundation::MovementLocalPosition;
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};

#[test]
fn ddepths_faction_step_uses_points_and_all_three_access_flags_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("ddepths-faction", move || {
        run(async |configured_admin| {
            assert!(admin == configured_admin);
            let scene = super::srx3_soul_war_world_fixture::source_world(MovementLocalPosition {
                x: 33830,
                y: 32187,
                floor: 14,
            })?;
            let h = Harness::create_with_admission_seed(
                configured_admin,
                "ddepths_faction",
                false,
                false,
            )
            .await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let deps = scene.dependencies()?;
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = scene.controller.active();
            owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&scene.room));
            let session = deps.admitted.game_session_id;
            let fence = owner
                .current_quest_fence(session)
                .await
                .map_err(|()| "Character fence")?
                .ok_or("current Character fence")?;
            let character = fence.character_id;
            {
                let mut runtime = owner.runtime.lock().await;
                runtime
                    .bind_attacker_lease(
                        deps.actor,
                        session,
                        crate::foundation::CharacterLease::new(
                            crate::foundation::CharacterId::decode(character.as_bytes())
                                .map_err(debug)?,
                            fence.character_lease_generation,
                        )
                        .map_err(debug)?,
                    )
                    .map_err(debug)?;
                runtime
                    .initialize_first_entry_position(deps.actor)
                    .map_err(debug)?;
            }
            deps.initialize(&owner).await;
            for (points, flags, x, y) in [
                (9_i64, [1_i64, 1, 1], 33830, 32186),
                (10, [1, 1, 0], 33830, 32186),
                (14, [1, 1, 2], 33830, 32186),
                (15, [0, 0, 0], 33982, 32236),
                (10, [1, 1, 1], 33982, 32236),
            ] {
                let mut tx = h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica")
                    .execute(&mut *tx)
                    .await?;
                for (track, quest, value) in [
                    (
                        "oteryn:quest-progress/quest/u11_50/dangerous_depths/gnomes/points",
                        "oteryn:quest.dangerous_depths_quest",
                        points,
                    ),
                    (
                        "oteryn:quest-progress/quest/u9_60/bigfoots_burden/warzone1_access",
                        "oteryn:quest.bigfoot_s_burden_quest",
                        flags[0],
                    ),
                    (
                        "oteryn:quest-progress/quest/u9_60/bigfoots_burden/warzone2_access",
                        "oteryn:quest.bigfoot_s_burden_quest",
                        flags[1],
                    ),
                    (
                        "oteryn:quest-progress/quest/u9_60/bigfoots_burden/warzone3_access",
                        "oteryn:quest.bigfoot_s_burden_quest",
                        flags[2],
                    ),
                ] {
                    sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,$3,$4,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid ON CONFLICT (character_id,track_key) DO UPDATE SET value=excluded.value")
                        .bind(character.as_bytes().as_slice()).bind(track).bind(quest).bind(value).execute(&mut *tx).await?;
                }
                tx.commit().await?;
                let actor = deps.actor;
                let origin = {
                    let mut runtime = owner.runtime.lock().await;
                    let current = runtime.read_actor_position(actor).map_err(debug)?;
                    let start = MovementLocalPosition {
                        x: 33830,
                        y: 32187,
                        floor: 14,
                    };
                    if current.position() == start {
                        current
                    } else {
                        let map = owner.bundle_map().ok_or("faction portal map")?;
                        let proof = crate::map::floor::prepare_quest_callback_teleport(
                            &runtime,
                            &owner.movement_cells,
                            actor,
                            session,
                            current,
                            start,
                            false,
                            &crate::map::floor::BundleDoors(map.facts.doors()),
                        )
                        .map_err(debug)?;
                        runtime.commit_source_step(proof).map_err(debug)?
                    }
                };
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(1))
                    .ok_or("paced step clock")?;
                let (outcome, _) = owner
                    .ordinary_paced_step(actor, session, StepDirection::West, Some(origin), None)
                    .await;
                assert_eq!(outcome.disposition, StepDisposition::Moved);
                assert_eq!(
                    owner
                        .runtime
                        .lock()
                        .await
                        .read_actor_position(actor)
                        .map_err(debug)?
                        .position(),
                    MovementLocalPosition { x, y, floor: 14 },
                    "faction portal with {points} points and flags {flags:?}",
                );
            }
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}
