use super::fixture::{Harness, debug, run};
use super::{TestResult, prepare_current_session};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef, MovementLocalPosition};
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};

const fn cell(x: i32, y: i32) -> MovementLocalPosition {
    MovementLocalPosition { x, y, floor: 14 }
}

const ROUTES: [(
    &str,
    MovementLocalPosition,
    StepDirection,
    MovementLocalPosition,
    MovementLocalPosition,
); 3] = [
    (
        "dwarves",
        cell(33783, 32205),
        StepDirection::East,
        cell(33782, 32205),
        cell(33921, 32401),
    ),
    (
        "gnomes",
        cell(33828, 32186),
        StepDirection::SouthEast,
        cell(33828, 32185),
        cell(33982, 32236),
    ),
    (
        "scouts",
        cell(33831, 32139),
        StepDirection::North,
        cell(33831, 32140),
        cell(34023, 32037),
    ),
];

#[test]
fn ddepths_faction_step_uses_points_and_all_three_access_flags_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("ddepths-faction", move || {
        run(async |configured_admin| {
            assert!(admin == configured_admin);
            let scene = super::srx3_soul_war_world_fixture::source_world(cell(33828, 32186))?;
            let h = Harness::create_with_admission_seed(
                configured_admin,
                "ddepths_faction",
                true,
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
            let catalogue = std::sync::Arc::new(
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?
                    .catalogue()
                    .clone(),
            );
            let setup = owner.revision_sequencer.acquire(fence.character_id).await
                .commit_quest_transition(
                    owner.root, &authority, owner.holder, fence,
                    QuestTransitionRequest {
                        transition_key: "oteryn:quest-transition/quest/u11_50/dangerous_depths/gnomes/measurements/npc_2".into(),
                        cause: QuestCause::Command(CommandRef::new(session, CommandId::new(1).map_err(debug)?)),
                    },
                    catalogue,
                ).await.map_err(debug)?;
            assert!(
                matches!(setup, QuestTransitionOutcome::Committed(_)),
                "{setup:?}"
            );
            // Fixture the previously earned faction points and Bigfoot flags.
            // The portal itself must use its current fence and live PG values.
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
            for (faction, approach, direction, refusal, landing) in ROUTES {
                for (points, flags, allowed) in [
                    (9_i64, [1_i64, 1, 1], false),
                    (10, [1, 1, 0], false),
                    (14, [1, 1, 2], false),
                    (15, [0, 0, 0], true),
                    (10, [1, 1, 1], true),
                ] {
                    let mut tx = h.pool.begin().await?;
                    sqlx::query("SET LOCAL session_replication_role=replica")
                        .execute(&mut *tx)
                        .await?;
                    let faction_points = ["dwarves", "gnomes", "scouts"].map(|name| {
                        (
                            format!(
                                "oteryn:quest-progress/quest/u11_50/dangerous_depths/{name}/points"
                            ),
                            "oteryn:quest.dangerous_depths_quest",
                            if name == faction { points } else { 15 },
                        )
                    });
                    let bigfoot_flags = [
                        (
                            "oteryn:quest-progress/quest/u9_60/bigfoots_burden/warzone1_access"
                                .to_owned(),
                            "oteryn:quest.bigfoot_s_burden_quest",
                            flags[0],
                        ),
                        (
                            "oteryn:quest-progress/quest/u9_60/bigfoots_burden/warzone2_access"
                                .to_owned(),
                            "oteryn:quest.bigfoot_s_burden_quest",
                            flags[1],
                        ),
                        (
                            "oteryn:quest-progress/quest/u9_60/bigfoots_burden/warzone3_access"
                                .to_owned(),
                            "oteryn:quest.bigfoot_s_burden_quest",
                            flags[2],
                        ),
                    ];
                    for (track, quest, value) in faction_points.into_iter().chain(bigfoot_flags) {
                        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,$3,$4,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid ON CONFLICT (character_id,track_key) DO UPDATE SET value=excluded.value")
                            .bind(character.as_bytes().as_slice()).bind(track).bind(quest).bind(value).execute(&mut *tx).await?;
                    }
                    tx.commit().await?;
                    let actor = deps.actor;
                    let origin = {
                        let mut runtime = owner.runtime.lock().await;
                        let current = runtime.read_actor_position(actor).map_err(debug)?;
                        if current.position() == approach {
                            current
                        } else {
                            let map = owner.bundle_map().ok_or("faction portal map")?;
                            let doors = crate::map::floor::BundleDoors(map.facts.doors());
                            let proof = crate::map::floor::prepare_quest_callback_teleport(
                                &runtime,
                                &owner.movement_cells,
                                actor,
                                session,
                                current,
                                approach,
                                false,
                                &doors,
                            )
                            .map_err(debug)?;
                            runtime.commit_source_step(proof).map_err(debug)?
                        }
                    };
                    owner.clock_origin = owner
                        .clock_origin
                        .checked_sub(std::time::Duration::from_secs(3))
                        .ok_or("paced step clock")?;
                    let (outcome, _) = owner
                        .ordinary_paced_step(actor, session, direction, Some(origin), None)
                        .await;
                    assert_eq!(
                        outcome.disposition,
                        StepDisposition::Moved,
                        "{faction} entrance"
                    );
                    assert_eq!(
                        owner
                            .runtime
                            .lock()
                            .await
                            .read_actor_position(actor)
                            .map_err(debug)?
                            .position(),
                        if allowed { landing } else { refusal },
                        "{faction} portal with {points} points and flags {flags:?}",
                    );
                }
            }
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}
