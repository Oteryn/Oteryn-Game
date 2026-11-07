//! P22d: domain 98 pushes a tracker after a settled kill of a creature on it, on the
//! visibility cadence (Crystal iobestiary.cpp:504-507, io_bosstiary.cpp:194).
use super::*;
use oteryn_protocol_oteryn::cyclopedia_tracker as wire;

pub(super) async fn refresh<S: AsyncRead + AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
) -> Result<(), ServeEnd> {
    let selected = session
        .selected_capabilities
        .domain_selected(wire::STATE_DOMAIN_CYCLOPEDIA_TRACKER);
    for tracker in authority
        .cyclopedia_tracker_refresh(session.admitted, selected)
        .await
    {
        let payload = wire::encode_snapshot(&tracker).ok_or(ServeEnd::Disconnected)?;
        let sequence = session
            .sequence
            .checked_add(1)
            .ok_or(ServeEnd::Disconnected)?;
        let base = session.cyclopedia_tracker_revision;
        let revision = base.checked_add(1).ok_or(ServeEnd::Disconnected)?;
        let frame = oteryn_protocol_oteryn::encode_state_delta(
            session.generation,
            sequence,
            wire::STATE_DOMAIN_CYCLOPEDIA_TRACKER,
            base,
            revision,
            wire::DELTA_TYPE_CYCLOPEDIA_TRACKER_V1,
            &payload,
        )
        .map_err(|_| ServeEnd::Disconnected)?;
        write_frame(stream, &frame)
            .await
            .map_err(|_| ServeEnd::Disconnected)?;
        session.sequence = sequence;
        session.admitted.continuity.server_sequence = sequence;
        session.cyclopedia_tracker_revision = revision;
    }
    Ok(())
}
