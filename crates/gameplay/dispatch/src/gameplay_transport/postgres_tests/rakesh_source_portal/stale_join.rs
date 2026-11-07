//! A real shared-room participant must retain the independently current allocation revision.
use super::*;
use crate::durability::activity_handoff::{
    ActivityContentPin, ActivityEnterOutcome, ActivityEnterRequest, ActivityHandoffOwner,
    ActivityParticipant, ActivityRefusal,
};
use crate::durability::character_last_position::LastVitals;
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
pub(super) async fn reject(
    source: &ComposedFreshAdmission<'_, '_, '_>,
    h: &super::super::fixture::Harness,
    peer: AdmittedSession,
    first: AdmittedSession,
) -> TestResult {
    let scope = first
        .item_fence
        .ok_or("actual shared destination")?
        .runtime_scope;
    let assignment = h
        .root
        .read_runtime_scope_assignment(scope)
        .await
        .map_err(debug)?
        .ok_or("actual independent current assignment")?;
    let registered = source
        .activity_runtimes
        .get(scope, assignment.ownership_generation)
        .ok_or("actual shared native room")?;
    let key = registered
        .template
        .as_ref()
        .ok_or("actual qualified Source template key")?;
    let template = source
        .activity_runtimes
        .template(&key.key, &key.revision)
        .ok_or("actual qualified Source template")?;
    let fence = source
        .current_quest_fence(peer.game_session_id)
        .await
        .map_err(|_| "actual peer fence")?
        .ok_or("actual peer source")?
        .item_fence();
    let actor = peer.runtime_actor.ok_or("actual peer actor")?;
    let (saved, exit, occurrence) = {
        let mut runtime = source.runtime.lock().await;
        let snapshot = runtime
            .borrow_movement_position()
            .read(actor)
            .map_err(debug)?;
        assert_eq!(
            (
                snapshot.position().x,
                snapshot.position().y,
                snapshot.position().floor
            ),
            (33039, 32964, 8)
        );
        let mut saved = source.placed_record(&runtime, snapshot.position());
        let mut exit = source.placed_record(
            &runtime,
            MovementLocalPosition {
                x: 33040,
                y: 32962,
                floor: 8,
            },
        );
        let states = source.states.spell_states.lock().await;
        let state = states
            .get(&runtime, actor, peer.game_session_id)
            .ok_or("real peer vitals")?;
        let pools = state.vitals();
        saved.record.vitals = LastVitals::new(pools.health, pools.mana);
        saved.record.rest = Some(state.bed_rest_snapshot(source.owner_now().get()));
        exit.record.vitals = saved.record.vitals;
        exit.record.rest = saved.record.rest;
        (saved, exit, snapshot.revision())
    };
    let destination_content = {
        let runtime = registered.runtime.lock().await;
        ActivityContentPin {
            coordinate_frame_digest: runtime.content_pin().frame_binding_digest(),
            map_revision_digest: runtime.map_revision_digest(),
            server_artifact_digest: runtime.content_pin().server_artifact_digest(),
        }
    };
    let handoff: Vec<u8> =
        sqlx::query_scalar("SELECT uuid_send(handoff_id) FROM game_activity_handoff_groups")
            .fetch_one(&h.pool)
            .await?;
    let cell = *template
        .entry_positions()
        .first()
        .ok_or("actual Source landing")?;
    let request = ActivityEnterRequest {
        handoff_id: handoff.try_into().map_err(|_| "actual handoff UUID")?,
        command_id: occurrence,
        initiating_character_id: crate::foundation::CharacterId::decode(
            fence.character_id.as_bytes(),
        )
        .map_err(debug)?,
        activity_key: template.boss_key().into(),
        template_revision: template.template_revision().into(),
        content_revision: template.content_revision().into(),
        destination_scope: scope,
        destination_generation: crate::foundation::ScopeOwnershipGeneration::new(
            assignment.ownership_generation,
        )
        .map_err(debug)?,
        destination_assignment_source_revision: assignment
            .source_revision
            .checked_add(1)
            .ok_or("stale source revision bound")?,
        destination_content,
        time_limit_ms: u64::from(template.time_limit_seconds()) * 1000,
        participants: vec![ActivityParticipant {
            source: fence,
            destination_game_session_id: crate::foundation::GameSessionId::decode(&id(155))
                .map_err(debug)?,
            destination_transport: Some(
                crate::foundation::AuthenticatedTransportRefV1::decode(&id(156)).map_err(debug)?,
            ),
            selected_capabilities: peer.continuity.selected_capabilities.as_slice().to_vec(),
            source_position: saved.record,
            source_position_write_sequence: saved.write_sequence,
            destination_cell: crate::durability::character_last_position::LastPositionCell {
                x: cell.x,
                y: cell.y,
                floor: i16::try_from(cell.z)?,
            },
            exit_position: exit.record,
        }],
    };
    let result = ActivityHandoffOwner::new(h.root.clone())
        .enter_boss_room(source.character, source.holder, request)
        .await?;
    assert_eq!(
        result,
        ActivityEnterOutcome::Refused(ActivityRefusal::NoCapacity),
        "qualified join may bypass occupancy only, never the exact current assignment source revision"
    );
    let counts:(i64,i64)=sqlx::query_as("SELECT(SELECT count(*) FROM game_activity_handoff_participants),(SELECT count(*) FROM game_summer_portal_joins)")
        .fetch_one(&h.pool).await?;
    assert_eq!(
        counts,
        (1, 0),
        "refused stale join cannot issue a successor or source child"
    );
    Ok(())
}
