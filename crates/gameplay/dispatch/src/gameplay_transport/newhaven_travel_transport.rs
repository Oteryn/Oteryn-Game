//! A completed NPC relocation retires a walk buffered at the old position.
use super::*;

pub(super) async fn retire_buffer<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    before: Option<WorldSpatialObservation>,
    buffered: &mut Option<BufferedStep>,
) -> Option<WorldSpatialObservation> {
    let after = authority.observe(actor).await;
    if before
        .zip(after)
        .is_some_and(|(before, after)| before.actor_position != after.actor_position)
    {
        *buffered = None;
        return after;
    }
    None
}

/// Reuse the step owner's spatial, map and open-container updates after NPC relocation.
#[allow(
    clippy::too_many_arguments,
    reason = "the existing moved-step view boundary"
)]
pub(super) async fn emit_relocation<S: AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
    stream: &mut S,
    authority: &A,
    actor: ExactActorRef,
    admitted: &mut AdmittedSession,
    visibility: Option<&mut SessionVisibility>,
    item_view: Option<&mut SessionItemView>,
    world_map: Option<&mut SessionMapView>,
    generation: u64,
    sequence: &mut u64,
    revision: &mut u64,
    relocation: Option<WorldSpatialObservation>,
) -> Option<()> {
    if let Some(observation) = relocation {
        moved_step_frames(
            stream,
            authority,
            actor,
            admitted,
            visibility,
            item_view,
            world_map,
            generation,
            sequence,
            revision,
            observation,
        )
        .await?;
    }
    Some(())
}
