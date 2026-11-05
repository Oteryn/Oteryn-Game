//! Keep the existing container-view projection current for shop free-slot totals.
use crate::*;
pub(super) fn apply<S: SessionStream>(
    session: &mut Session<S>,
    sequence: u64,
    delta: oteryn_protocol_oteryn::StateDeltaView<'_>,
) -> Result<SessionEvent, SessionError> {
    let domain = container_tree::STATE_DOMAIN_CONTAINER_VIEWS;
    if !session.is_selected(container_tree::CAPABILITY_CONTAINER_TREE_V1) {
        return Err(SessionError::UnselectedDomain { domain_id: domain });
    }
    let revision = session
        .gated_revisions
        .iter()
        .find(|(id, _)| *id == domain)
        .map_or(0, |(_, revision)| *revision);
    check_delta(&delta, 1, revision)?;
    let value = container_tree::decode_container_views_for(delta.payload, session.is_selected(230))
        .map_err(SessionError::ItemView)?;
    match session
        .gated_revisions
        .iter_mut()
        .find(|(id, _)| *id == domain)
    {
        Some((_, revision)) => *revision = delta.new_revision,
        None => session.gated_revisions.push((domain, delta.new_revision)),
    }
    let snapshot = GatedSnapshot {
        domain_id: domain,
        snapshot_type: 1,
        revision: delta.new_revision,
        payload: delta.payload.to_vec(),
    };
    match session
        .gated_snapshots
        .iter_mut()
        .find(|snapshot| snapshot.domain_id == domain)
    {
        Some(stored) => *stored = snapshot,
        None => session.gated_snapshots.push(snapshot),
    }
    Ok(SessionEvent::ContainerViews(AppliedDelta {
        server_sequence: sequence,
        base_revision: delta.base_revision,
        new_revision: delta.new_revision,
        value,
    }))
}
