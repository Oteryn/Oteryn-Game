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
