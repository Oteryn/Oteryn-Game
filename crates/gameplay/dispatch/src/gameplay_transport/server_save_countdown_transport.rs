//! PARITY-EVENT-CALLBACKS-1: capability-gated server-save countdown framing (domain 40).
use super::*;
use oteryn_protocol_oteryn::server_save_countdown::*;

pub(super) fn snapshot(admitted: &AdmittedSession) -> Option<DomainSnapshot<'static>> {
    admitted
        .continuity
        .selected_capabilities
        .contains(CAPABILITY_SERVER_SAVE_COUNTDOWN_V1)
        .then_some(DomainSnapshot {
            domain_id: STATE_DOMAIN_SERVER_SAVE_COUNTDOWN,
            revision: admitted.continuity.server_save_revision,
            snapshot_type: SNAPSHOT_TYPE_SERVER_SAVE_COUNTDOWN_V1,
            payload: SERVER_SAVE_COUNTDOWN_SNAPSHOT,
        })
}

/// Writes the Channel's due minute once per session; the receipt travels with continuity.
pub(super) async fn emit<S: AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
    stream: &mut S,
    authority: &A,
    admitted: &mut AdmittedSession,
    sequence: &mut u64,
) -> Option<()> {
    if !admitted
        .continuity
        .selected_capabilities
        .contains(CAPABILITY_SERVER_SAVE_COUNTDOWN_V1)
    {
        return Some(());
    }
    let Some(due) = authority
        .server_save_countdown()
        .filter(|due| due.follows(admitted.continuity.server_save_sent))
    else {
        return Some(());
    };
    let payload = encode_server_save_countdown(due.notice).ok()?;
    let next_sequence = sequence.checked_add(1)?;
    let revision = admitted.continuity.server_save_revision;
    let next_revision = revision.checked_add(1)?;
    let frame = encode_state_delta(
        admitted.continuity.connection_generation,
        next_sequence,
        STATE_DOMAIN_SERVER_SAVE_COUNTDOWN,
        revision,
        next_revision,
        DELTA_TYPE_SERVER_SAVE_COUNTDOWN_V1,
        &payload,
    )
    .ok()?;
    *sequence = next_sequence;
    admitted.continuity.server_sequence = next_sequence;
    admitted.continuity.server_save_revision = next_revision;
    admitted.continuity.server_save_sent = Some(due.receipt());
    write_frame(stream, &frame).await.ok()
}
