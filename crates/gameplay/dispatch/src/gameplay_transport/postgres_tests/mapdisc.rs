//! MAPDISC: entering a subarea discovers its points of interest through the production
//! movement hook (`cyclopedia_step`, called by `paced_movement`), once per Character; the
//! admitted command 6262 lists the discovery and the Measuring Tibia progress it fed.
use super::cyclopedia::support;
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::domain::cyclopedia::MapProgress;
use crate::foundation::{
    ChannelContentPin, ChannelRuntimeV1, ExactActorRef, MovementLocalPosition,
    MovementPositionSnapshot,
};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::cyclopedia_discovery as wire;
use oteryn_protocol_oteryn::tutorial_notices::TutorialNotice;

#[test]
fn mapdisc_entering_subareas_discovers_once_lists_them_and_feeds_measuring_tibia() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "MAPDISC PostgreSQL scenario panicked")??;
    Ok(())
}

/// Walk via row 0 (the only rows that span every x), then step the production hook.
async fn enter(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: crate::foundation::GameSessionId,
    x: i32,
    y: i32,
) -> TestResult<MovementPositionSnapshot> {
    let entered = {
        let mut runtime = owner.runtime.lock().await;
        let mut movement = runtime.borrow_movement_position();
        let mut current = movement.read(actor).map_err(debug)?;
        let floor = current.position().floor;
        for target in [
            MovementLocalPosition {
                y: 0,
                ..current.position()
            },
            MovementLocalPosition { x, y: 0, floor },
            MovementLocalPosition { x, y, floor },
        ] {
            while current.position() != target {
                let at = current.position();
                let next = MovementLocalPosition {
                    x: at.x + (target.x - at.x).signum(),
                    y: if at.x == target.x {
                        at.y + (target.y - at.y).signum()
                    } else {
                        at.y
                    },
                    ..at
                };
                current = movement.commit_cardinal(current, next).map_err(debug)?;
            }
        }
        current
    };
    owner.cyclopedia_step(actor, session, entered).await;
    Ok(entered)
}

async fn points(h: &Harness, subarea: u32) -> TestResult<Vec<(i32, i32)>> {
    let progress: String = sqlx::query_scalar(
        "SELECT (progress->'subareas'->($1::int::text))::text FROM game_character_cyclopedia",
    )
    .bind(subarea as i32)
    .fetch_one(&h.pool)
    .await?;
    let progress: MapProgress = serde_json::from_str(&progress)?;
    Ok(progress
        .exploration
        .ok_or("automatic exploration")?
        .points
        .iter()
        .map(|p| (p.x, p.y))
        .collect())
}

async fn revision(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT character_revision::bigint FROM game_character_roots")
            .fetch_one(&h.pool)
            .await?,
    )
}

async fn measuring(h: &Harness) -> TestResult<(i32, i32, i32, i16)> {
    Ok(sqlx::query_as(
        "SELECT discovered_subareas,completed_areas,main_areas,stage FROM game_character_measuring_tibia",
    )
    .fetch_one(&h.pool)
    .await?)
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "mapdisc", true, false).await?;
        eprintln!("POSTGRES_STARTED MAPDISC mapdisc_discovery");
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activation_pin) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("active generation")?;
        let world = support::world_with_subareas(&deps, generation, true)?;
        let cells = world
            .movement_cells(activation_pin.qualified_room().movement_cells())
            .map_err(debug)?;
        support::seed(&h, world.base().digest()).await?;
        let combat_room = activation_pin
            .qualified_room()
            .bundle_view(
                cells.clone(),
                activation_pin.frame_binding().digest(),
                world.map_revision_digest(),
                crate::content::NativeEntryStart {
                    x: 1,
                    y: 0,
                    floor: 7,
                },
            )
            .ok_or("MAPDISC room")?;
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activation_pin.frame_binding().digest(),
            world.map_revision_digest(),
            (1, 0, 7),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.movement_cells = (&cells).into();
        owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&combat_room));
        owner.active_generation = Some(generation);
        deps.initialize(&owner).await;
        assert!(owner.cyclopedia_available(), "authored areas are admitted");
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
        let offers = crate::gameplay_transport::cyclopedia_discovery::offered(
            &crate::gameplay_transport::cyclopedia::offered(
                owner.offered_capabilities(),
                owner.cyclopedia_available(),
            ),
            owner.cyclopedia_available(),
        );
        assert!(
            crate::gameplay_transport::cyclopedia_discovery::offered(&offers, false)
                .iter()
                .all(|c| c.id != wire::CAPABILITY_CYCLOPEDIA_DISCOVERY_V1),
            "no Cyclopedia content, no offer"
        );
        let session = admitted.game_session_id;
        let actor = deps.actor;

        // Entering subarea 2 starts its exploration; standing on each POI finds it.
        enter(&owner, actor, session, 1, 0).await?;
        let first = points(&h, 2).await?;
        for (x, y) in first.iter().copied() {
            enter(&owner, actor, session, x, y).await?;
        }
        assert_eq!(
            owner.states.tutorial_sessions.take(session),
            Some(TutorialNotice::StatusText(
                "Subarea discovered! All seven points of interest found.".into()
            ))
        );
        assert_eq!(measuring(&h).await?, (1, 0, 1, 0));
        // Re-entering the discovered subarea writes nothing and repeats no notice.
        let discovered_once = revision(&h).await?;
        let receipts = "SELECT count(*) FROM game_character_cyclopedia_receipts";
        let receipts_once: i64 = sqlx::query_scalar(receipts).fetch_one(&h.pool).await?;
        enter(&owner, actor, session, first[0].0, first[0].1).await?;
        enter(&owner, actor, session, 1, 0).await?;
        assert_eq!(revision(&h).await?, discovered_once, "discovered once");
        assert_eq!(
            sqlx::query_scalar::<_, i64>(receipts).fetch_one(&h.pool).await?,
            receipts_once
        );
        assert!(owner.states.tutorial_sessions.take(session).is_none());
        assert_eq!(measuring(&h).await?, (1, 0, 1, 0));

        // An unselected capability cannot read; the selected one lists the discovery.
        let mut unselected = admitted;
        unselected.continuity.selected_capabilities =
            SelectedCapabilities::select(&offers, &[4, 6, 57]).ok_or("selection")?;
        let (status, empty) = support::exchange(&owner, &mut unselected, 1, 6262, &[]).await?;
        assert_eq!((status, empty.len()), (CommandStatus::Rejected, 0));
        admitted.continuity = unselected.continuity;
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offers, &[4, 6, 57, 527]).ok_or("selection")?;
        let (status, bytes) = support::exchange(&owner, &mut admitted, 2, 6262, &[]).await?;
        assert_eq!(status, CommandStatus::Accepted);
        let view = wire::decode_reply(&bytes).map_err(debug)?;
        assert_eq!(
            view.mains,
            [wire::MainArea {
                id: 1,
                status: wire::STATUS_DISCOVERING,
                progress: 50
            }]
        );
        assert_eq!((view.discovered, view.discoverable), (vec![2], vec![3]));
        assert_eq!(
            view.quest,
            wire::QuestProgress {
                discovered_subareas: 1,
                completed_areas: 0,
                main_areas: 1,
                stage: 0
            }
        );

        // Subarea 3 holds exactly seven tiles: crossing it completes the only main area.
        enter(&owner, actor, session, 11, 8).await?;
        for (x, y) in points(&h, 3).await? {
            enter(&owner, actor, session, x, y).await?;
        }
        assert_eq!(
            owner.states.tutorial_sessions.take(session),
            Some(TutorialNotice::StatusText(
                "Subarea discovered! All seven points of interest found.".into()
            ))
        );
        assert_eq!(measuring(&h).await?, (2, 1, 1, 3), "Measuring Tibia completed");
        let next = admitted.continuity.next_command_id;
        let (status, bytes) = support::exchange(&owner, &mut admitted, next, 6262, &[]).await?;
        assert_eq!(status, CommandStatus::Accepted);
        let view = wire::decode_reply(&bytes).map_err(debug)?;
        assert_eq!(
            view,
            wire::Discovery {
                mains: vec![wire::MainArea {
                    id: 1,
                    status: wire::STATUS_COMPLETE,
                    progress: 100
                }],
                discovered: vec![2, 3],
                discoverable: vec![],
                quest: wire::QuestProgress {
                    discovered_subareas: 2,
                    completed_areas: 1,
                    main_areas: 1,
                    stage: 3
                },
            }
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        eprintln!("POSTGRES_CASE_COMPLETED MAPDISC mapdisc_discovery");
        Ok(())
    })
}
