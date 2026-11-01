//! Every activity participant crosses the source stream's scope barrier before
//! any destination snapshot. Non-initiators have no fabricated command result.
use super::*;
use oteryn_protocol_oteryn::activity_scope::STATE_DOMAIN_ACTIVITY_SCOPE;

pub(super) async fn pending<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
) -> Result<Option<AdmittedSession>, ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    let Some(switch) = authority
        .pending_activity_switch(session.admitted)
        .await
        .map_err(|_| ServeEnd::Disconnected)?
    else {
        return Ok(None);
    };
    let source = session.admitted;
    let sequence = session
        .sequence
        .checked_add(1)
        .ok_or(ServeEnd::Disconnected)?;
    let frames = encode_single_chunk_snapshot(
        session.generation,
        sequence,
        sequence,
        &[DomainSnapshot {
            domain_id: STATE_DOMAIN_ACTIVITY_SCOPE,
            revision: 1,
            snapshot_type: 1,
            payload: &switch.payload,
        }],
    )
    .map_err(|_| ServeEnd::Disconnected)?;
    // SQL and native installation already changed the current session. A socket
    // failure while sending the barrier must lose the exact destination control,
    // rather than try to release the terminal source or reset a stale generation.
    session.admitted = switch.destination;
    for frame in frames {
        write_frame(stream, &frame)
            .await
            .map_err(|_| ServeEnd::Disconnected)?;
    }
    if !authority.finish_activity_switch(source, switch.destination) {
        return Err(ServeEnd::Disconnected);
    }
    Ok(Some(switch.destination))
}
