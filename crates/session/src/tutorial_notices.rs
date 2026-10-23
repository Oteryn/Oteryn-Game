//! Tutorial notice application using the existing selected-domain revision store.
use super::*;
use oteryn_protocol_oteryn::tutorial_notices::*;

pub(super) fn validate_join(
    selected: &[u32],
    snapshots: &[GatedSnapshot],
) -> Result<(), SessionError> {
    if !selected.contains(&CAPABILITY_TUTORIAL_NOTICES_V1) {
        return Ok(());
    }
    let snapshot = snapshots
        .iter()
        .find(|row| row.domain_id == STATE_DOMAIN_TUTORIAL_NOTICES)
        .ok_or(SessionError::MissingDomain(STATE_DOMAIN_TUTORIAL_NOTICES))?;
    if snapshot.snapshot_type != 1 || snapshot.payload != TUTORIAL_NOTICES_SNAPSHOT {
        return Err(SessionError::Protocol(
            FoundationProtocolError::MalformedFrame,
        ));
    }
    Ok(())
}

pub(super) fn apply<S: SessionStream>(
    session: &mut Session<S>,
    sequence: u64,
    payload: &[u8],
) -> Result<SessionEvent, SessionError> {
    if !session.is_selected(CAPABILITY_TUTORIAL_NOTICES_V1) {
        return Err(SessionError::UnselectedDomain {
            domain_id: STATE_DOMAIN_TUTORIAL_NOTICES,
        });
    }
    let delta = decode_state_delta(payload)?;
    let entry = session
        .gated_revisions
        .iter_mut()
        .find(|(id, _)| *id == STATE_DOMAIN_TUTORIAL_NOTICES)
        .ok_or(SessionError::MissingDomain(STATE_DOMAIN_TUTORIAL_NOTICES))?;
    check_delta(&delta, 1, entry.1)?;
    let value = decode_tutorial_notice(delta.payload)
        .map_err(|_| SessionError::Protocol(FoundationProtocolError::MalformedFrame))?;
    entry.1 = delta.new_revision;
    Ok(SessionEvent::TutorialNotice(AppliedDelta {
        server_sequence: sequence,
        base_revision: delta.base_revision,
        new_revision: delta.new_revision,
        value,
    }))
}
