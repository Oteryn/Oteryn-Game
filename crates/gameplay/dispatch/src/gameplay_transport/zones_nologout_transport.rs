//! Capability 92 framing; only a completed login snapshot consumes the MOTD receipt.
use super::*;
use oteryn_protocol_oteryn::tutorial_notices::{TutorialNotice, encode_tutorial_notice};
use oteryn_protocol_oteryn::zones_nologout::*;

pub(super) fn snapshot<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
) -> Option<Result<crate::gameplay_transport::item_view::ItemViewSnapshotDomain, ()>> {
    if !selected(admitted) {
        return None;
    }
    let text = authority.server_motd();
    let payload = if admitted.continuity.server_motd_sent || text.is_empty() {
        vec![8, 1]
    } else {
        match encode_tutorial_notice(&TutorialNotice::StatusText(text.into())) {
            Ok(bytes) => bytes,
            Err(_) => return Some(Err(())),
        }
    };
    Some(Ok(
        crate::gameplay_transport::item_view::ItemViewSnapshotDomain {
            domain_id: STATE_DOMAIN_SERVER_MESSAGES,
            revision: admitted.continuity.server_message_revision,
            snapshot_type: 1,
            payload,
        },
    ))
}

fn selected(admitted: &AdmittedSession) -> bool {
    admitted
        .continuity
        .selected_capabilities
        .contains(CAPABILITY_ZONES_NOLOGOUT_V1)
}

pub(super) fn delivered(admitted: &mut AdmittedSession) {
    if selected(admitted) {
        admitted.continuity.server_motd_sent = true;
    }
}

pub(super) async fn emit<S: AsyncWrite + Unpin>(
    stream: &mut S,
    admitted: &mut AdmittedSession,
    sequence: &mut u64,
    text: &str,
) -> Option<()> {
    if !selected(admitted) {
        return Some(());
    }
    let payload = encode_tutorial_notice(&TutorialNotice::StatusText(text.into())).ok()?;
    let next_sequence = sequence.checked_add(1)?;
    let revision = admitted.continuity.server_message_revision;
    let next_revision = revision.checked_add(1)?;
    let frame = encode_state_delta(
        admitted.continuity.connection_generation,
        next_sequence,
        STATE_DOMAIN_SERVER_MESSAGES,
        revision,
        next_revision,
        1,
        &payload,
    )
    .ok()?;
    write_frame(stream, &frame).await.ok()?;
    *sequence = next_sequence;
    admitted.continuity.server_sequence = next_sequence;
    admitted.continuity.server_message_revision = next_revision;
    Some(())
}
