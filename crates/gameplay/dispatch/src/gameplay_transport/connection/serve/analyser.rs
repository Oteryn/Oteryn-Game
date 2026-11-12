//! Capability-gated empty snapshots and bounded fact deltas on the real connection.
use super::*;
use oteryn_protocol_oteryn::analyser as wire;

pub(super) async fn snapshot<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &mut AdmittedSession,
    domains: &mut Vec<crate::gameplay_transport::item_view::ItemViewSnapshotDomain>,
) -> Result<(), ServeEnd> {
    if !admitted
        .continuity
        .selected_capabilities
        .contains(wire::CAPABILITY_ANALYSER_V1)
    {
        return Ok(());
    }
    let discarded = authority
        .analyser_snapshot(*admitted)
        .await
        .ok_or(ServeEnd::Disconnected)?;
    // Revision zero is an uninitialized stream, not a valid client snapshot.
    // Recovery retains the cursor and advances only when queued facts were discarded.
    if discarded || admitted.continuity.analyser_revision == 0 {
        admitted.continuity.analyser_revision = admitted
            .continuity
            .analyser_revision
            .checked_add(1)
            .ok_or(ServeEnd::Disconnected)?;
    }
    domains.push(
        crate::gameplay_transport::item_view::ItemViewSnapshotDomain {
            domain_id: wire::STATE_DOMAIN_ACTOR_ANALYSER,
            revision: admitted.continuity.analyser_revision,
            snapshot_type: wire::SNAPSHOT_TYPE_ANALYSER_V1,
            payload: Vec::new(),
        },
    );
    Ok(())
}

pub(super) async fn emit<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
) -> Result<(), ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    if !session
        .selected_capabilities
        .contains(wire::CAPABILITY_ANALYSER_V1)
    {
        return Ok(());
    }
    let Some(payload) = authority.analyser_next(session.admitted).await else {
        return Ok(());
    };
    let from = session.admitted.continuity.analyser_revision;
    let to = from.checked_add(1).ok_or(ServeEnd::Disconnected)?;
    let sequence = session
        .sequence
        .checked_add(1)
        .ok_or(ServeEnd::Disconnected)?;
    let frame = crate::foundation::encode_state_delta(
        session.generation,
        sequence,
        wire::STATE_DOMAIN_ACTOR_ANALYSER,
        from,
        to,
        wire::DELTA_TYPE_ANALYSER_FACTS_V1,
        &payload,
    )
    .map_err(|_| ServeEnd::Disconnected)?;
    // A failed write may have reached the peer. Never reuse that revision or sequence.
    session.admitted.continuity.analyser_revision = to;
    session.sequence = sequence;
    session.admitted.continuity.server_sequence = sequence;
    write_frame(stream, &frame)
        .await
        .map_err(|_| ServeEnd::Disconnected)
}

#[cfg(test)]
#[path = "../../analyser/transport_tests.rs"]
mod tests;
