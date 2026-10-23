//! Capability-gated tutorial notice framing. Mounted once by the connection owner.
use super::*;
use oteryn_protocol_oteryn::tutorial_notices::*;

pub(super) fn snapshot(admitted: &AdmittedSession) -> Option<DomainSnapshot<'static>> {
    admitted
        .continuity
        .selected_capabilities
        .contains(CAPABILITY_TUTORIAL_NOTICES_V1)
        .then_some(DomainSnapshot {
            domain_id: STATE_DOMAIN_TUTORIAL_NOTICES,
            revision: admitted.continuity.tutorial_revision,
            snapshot_type: 1,
            payload: TUTORIAL_NOTICES_SNAPSHOT,
        })
}

pub(super) async fn emit<S: AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
    stream: &mut S,
    authority: &A,
    admitted: &mut AdmittedSession,
    sequence: &mut u64,
) -> Option<()> {
    if !admitted
        .continuity
        .selected_capabilities
        .contains(CAPABILITY_TUTORIAL_NOTICES_V1)
    {
        return Some(());
    }
    while let Some(notice) = authority.take_tutorial_notice(admitted.game_session_id) {
        let payload = encode_tutorial_notice(&notice).ok()?;
        let next_sequence = sequence.checked_add(1)?;
        let revision = admitted.continuity.tutorial_revision;
        let next_revision = revision.checked_add(1)?;
        let frame = encode_state_delta(
            admitted.continuity.connection_generation,
            next_sequence,
            STATE_DOMAIN_TUTORIAL_NOTICES,
            revision,
            next_revision,
            1,
            &payload,
        )
        .ok()?;
        *sequence = next_sequence;
        admitted.continuity.server_sequence = next_sequence;
        admitted.continuity.tutorial_revision = next_revision;
        write_frame(stream, &frame).await.ok()?;
    }
    Some(())
}
