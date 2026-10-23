//! Frames published after a committed native movement step.
use super::*;

/// The frames that follow one moved step of `actor`: with capability 6 the interest set's change
/// (VIS-3, a resync ends the connection), without it the v1 own-actor delta; the open corpse a
/// step out of reach or to another floor closes (ITEM-VIEW-1b §4.3); and the tiles the step
/// brought into, kept in or took out of view (MAP-WIRE-2). A client step sends them after its
/// result, a chase step (CHASE-1) on its own. `None` ends the connection.
#[allow(clippy::too_many_arguments)]
pub(super) async fn moved_step_frames<S, A>(
    stream: &mut S,
    authority: &A,
    actor: ExactActorRef,
    admitted: &mut AdmittedSession,
    visibility: Option<&mut SessionVisibility>,
    mut item_view: Option<&mut SessionItemView>,
    world_map: Option<&mut SessionMapView>,
    generation: u64,
    sequence: &mut u64,
    revision: &mut u64,
    appearances: &mut SessionAppearance,
    observation: WorldSpatialObservation,
) -> Option<()>
where
    S: AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    let (frames_out, delta_sequence, new_revision) = if let Some(view) = visibility {
        let mut channel = authority.observe_visible_entities(actor).await?;
        // The step's committed position is the observer's own.
        let observer = channel.observer;
        for own in channel
            .entities
            .iter_mut()
            .filter(|entity| entity.identity == observer)
        {
            own.position = observation.actor_position;
        }
        let (next_sequence, next_revision) = appearances
            .refresh(
                stream,
                authority,
                actor,
                admitted,
                view,
                &channel,
                item_view.as_deref_mut(),
                *sequence,
                *revision,
            )
            .await?;
        (Vec::new(), next_sequence, next_revision)
    } else {
        let (Some(delta_sequence), Some(new_revision)) =
            (sequence.checked_add(1), revision.checked_add(1))
        else {
            return None;
        };
        let delta = encode_state_delta(
            generation,
            delta_sequence,
            STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            *revision,
            new_revision,
            DELTA_TYPE_WORLD_SPATIAL_V1,
            &encode_world_spatial(&observation),
        )
        .ok()?;
        (vec![delta], delta_sequence, new_revision)
    };
    *sequence = delta_sequence;
    admitted.continuity.server_sequence = *sequence;
    for frame in &frames_out {
        write_frame(stream, frame).await.ok()?;
    }
    // r4122508665: only record the new spatial revision once the delta that carries it has
    // actually been transmitted.
    *revision = new_revision;
    admitted.continuity.spatial_revision = *revision;
    if let Some(view) = item_view.as_deref_mut() {
        let closed = view.close(&CloseTrigger::Moved {
            to: observation.actor_position,
        });
        admitted.continuity.item_view = view.continuity();
        if let Some(delta) = closed.ok()? {
            let (delta_sequence, frame) = item_view_delta(generation, *sequence, &delta)?;
            *sequence = delta_sequence;
            admitted.continuity.server_sequence = *sequence;
            write_frame(stream, &frame).await.ok()?;
        }
    }
    if let (Some(view), Some(items)) = (world_map, item_view) {
        let update = world_map_delta(authority, actor, generation, *sequence, view, items).await;
        admitted.continuity.item_view = items.continuity();
        if let Some((delta_sequence, frame)) = update? {
            *sequence = delta_sequence;
            admitted.continuity.server_sequence = *sequence;
            admitted.continuity.world_map_revision = view.revision();
            write_frame(stream, &frame).await.ok()?;
        }
    }
    Some(())
}
