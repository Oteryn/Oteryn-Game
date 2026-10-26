//! Channel placement consumers must reject Instance coordinates independently
//! of the shared session fence, before a future activity handoff can reach them.
use super::*;
use crate::durability::character_last_position::{
    FinalLastPosition, LastPositionCell, LastPositionRecord, LastPositionWrite,
};
use crate::durability::character_progression::CharacterProgressionError;
use crate::durability::character_tutorial::TutorialProgressOutcome;
use crate::durability::npc_travel::{PendingArrival, TravelOccurrence};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_content::content::newhaven_travel::NewhavenRoute;
use oteryn_gameplay_quests::tutorial::{ISLET_TEMPLE, TutorialProgress, TutorialStep};

async fn qualify(h: &fixture::Harness) -> TestResult {
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let channel = fixture::fence(1)?;
    let scope = RuntimeScopeRefV1::instance(channel.runtime_scope.world_id(), id(88))
        .map_err(fixture::debug)?;
    sqlx::query("INSERT INTO game_control_activity_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,1)")
        .bind(scope.world_id().as_bytes().as_slice()).bind(id(88).as_slice()).execute(&h.pool).await?;
    committed(
        h.writer
            .submit(&request(
                101,
                AssignmentCommand::Assign {
                    scope,
                    target: h.node.fact(),
                },
            )?)
            .await,
    )?;
    assert_eq!(
        h.root
            .publish_runtime_readiness(&h.node, &readiness(h, scope, 1, 1).await?)
            .await
            .map_err(fixture::debug)?,
        GuardPublicationDisposition::Applied
    );
    assert!(runtime_fence(h, scope, 1, &h.node).await?);
    let mut instance = channel;
    instance.runtime_scope = scope;
    let original = LastPositionRecord {
        world_id: scope.world_id(),
        cell: LastPositionCell {
            x: 100,
            y: 200,
            floor: 7,
        },
        coordinate_frame_digest: [1; 32],
        map_revision_digest: [2; 32],
        server_artifact_digest: [3; 32],
        vitals: None,
        rest: None,
    };
    assert_eq!(
        h.root
            .write_character_last_position(&authority, &h.node, channel, 1, original)
            .await?,
        LastPositionWrite::Written
    );
    h.root
        .settle_bed_regen(&authority, &h.node, channel)
        .await?;
    assert!(matches!(
        h.root.settle_bed_regen(&authority, &h.node, instance).await,
        Err(crate::durability::DurabilityError::Unavailable)
    ));
    let progress = TutorialProgress {
        step: TutorialStep::Islet,
        hunting: false,
        corruptor: false,
    };
    let crate::foundation::RuntimeScopeRefV1::Channel { channel_id, .. } = channel.runtime_scope
    else {
        return Err("Channel fixture required".into());
    };
    let mut same_uuid = channel;
    same_uuid.runtime_scope =
        RuntimeScopeRefV1::instance(channel.runtime_scope.world_id(), *channel_id.as_bytes())
            .map_err(fixture::debug)?;
    // Neither readiness nor identical UUID bytes confer tutorial placement
    // authority. Check the consumer's own scope gate, not a failed session
    // lookup: it must return InvalidInput before reaching the shared fence.
    for foreign in [instance, same_uuid] {
        for change in [None, Some((0, progress))] {
            assert!(matches!(
                h.root
                    .character_tutorial_progress(&authority, &h.node, foreign, change)
                    .await,
                Err(CharacterProgressionError::InvalidInput)
            ));
            assert_eq!(
                h.root
                    .character_tutorial_progress(&authority, &h.node, channel, None)
                    .await?,
                TutorialProgressOutcome::Read(None)
            );
        }
    }
    let pending = h
        .root
        .character_tutorial_progress(&authority, &h.node, channel, Some((0, progress)))
        .await?;
    let TutorialProgressOutcome::Written(pending) = pending else {
        return Err("missing islet obligation".into());
    };
    assert!(pending.pending_islet);
    for foreign in [instance, same_uuid] {
        // Read, stale-version conflict, equal-state replay and a real CAS
        // change all need the same scope gate; no outcome can bypass it.
        for change in [
            None,
            Some((0, progress)),
            Some((pending.version, progress)),
            Some((
                pending.version,
                TutorialProgress {
                    hunting: true,
                    ..progress
                },
            )),
        ] {
            assert!(matches!(
                h.root
                    .character_tutorial_progress(&authority, &h.node, foreign, change)
                    .await,
                Err(CharacterProgressionError::InvalidInput)
            ));
            assert_eq!(
                h.root
                    .character_tutorial_progress(&authority, &h.node, channel, None)
                    .await?,
                TutorialProgressOutcome::Read(Some(pending))
            );
        }
    }
    let islet = FinalLastPosition {
        write_sequence: 2,
        record: LastPositionRecord {
            cell: LastPositionCell {
                x: ISLET_TEMPLE.0,
                y: ISLET_TEMPLE.1,
                floor: ISLET_TEMPLE.2,
            },
            ..original
        },
    };
    assert!(matches!(
        h.root
            .complete_tutorial_islet_placement(&authority, &h.node, instance, islet)
            .await,
        Err(CharacterProgressionError::InvalidInput)
    ));
    assert_eq!(
        h.root
            .character_tutorial_progress(&authority, &h.node, channel, None)
            .await?,
        TutorialProgressOutcome::Read(Some(pending))
    );
    assert_eq!(
        h.root
            .read_character_last_position(&authority, &h.node, channel)
            .await?
            .ok_or("missing saved position")?
            .record,
        original
    );
    assert!(
        h.root
            .complete_tutorial_islet_placement(&authority, &h.node, channel, islet)
            .await?
    );
    assert!(
        !h.root
            .complete_tutorial_islet_placement(&authority, &h.node, channel, islet)
            .await?
    );
    let placed = h
        .root
        .read_character_last_position(&authority, &h.node, channel)
        .await?
        .ok_or("missing placed position")?;
    assert_eq!(placed.record, islet.record);
    assert_eq!(placed.key.write_sequence, 2);

    let route = NewhavenRoute::Seagull;
    let destination = route.destination();
    let arrival = PendingArrival {
        occurrence: TravelOccurrence::from_bytes(id(102)).map_err(fixture::debug)?,
        world_id: scope.world_id(),
        destination: LastPositionCell {
            x: destination.0,
            y: destination.1,
            floor: destination.2,
        },
    };
    let command = CommandRef::new(
        channel.game_session_id,
        CommandId::new(101).map_err(fixture::debug)?,
    );
    assert!(matches!(
        h.root
            .commit_newhaven_travel(&authority, &h.node, instance, command, arrival, route)
            .await,
        Err(CharacterProgressionError::InvalidInput)
    ));
    assert!(
        h.root
            .read_npc_pending_arrival(&authority, &h.node, channel)
            .await
            .map_err(fixture::debug)?
            .is_none()
    );
    assert_eq!(
        h.root
            .read_character_last_position(&authority, &h.node, channel)
            .await?
            .ok_or("missing saved islet")?,
        placed
    );
    let before = h
        .root
        .character_tutorial_progress(&authority, &h.node, channel, None)
        .await?;
    let TutorialProgressOutcome::Read(Some(before)) = before else {
        return Err("missing tutorial after placement".into());
    };
    assert_eq!(before.progress, progress);
    assert!(!before.pending_islet);
    assert_eq!(
        h.root
            .commit_newhaven_travel(&authority, &h.node, channel, command, arrival, route)
            .await?,
        Some(arrival)
    );
    assert!(matches!(
        h.root
            .commit_newhaven_travel(&authority, &h.node, instance, command, arrival, route)
            .await,
        Err(CharacterProgressionError::InvalidInput)
    ));
    assert_eq!(
        h.root
            .read_npc_pending_arrival(&authority, &h.node, channel)
            .await
            .map_err(fixture::debug)?,
        Some(arrival)
    );
    assert_eq!(
        h.root
            .commit_newhaven_travel(&authority, &h.node, channel, command, arrival, route)
            .await?,
        Some(arrival)
    );
    assert_eq!(
        h.root
            .read_character_last_position(&authority, &h.node, channel)
            .await?
            .ok_or("travel overwrote saved position")?,
        placed
    );
    Ok(())
}

#[test]
fn boss_activity_channel_consumers_postgres() -> TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    fixture::runtime()?.block_on(async move {
        let h = fixture::Harness::create(admin, "boss_channel_consumers", true).await?;
        eprintln!("Canonical PostgreSQL 18.6; Channel tutorial progress/placement/travel Instance refusal qualification");
        let result = qualify(&h).await;
        h.cleanup().await?;
        result
    })
}
