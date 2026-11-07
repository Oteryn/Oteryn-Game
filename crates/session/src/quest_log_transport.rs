//! Selected quest domain validation; interpretation stays in client-runtime.
use super::*;

pub(super) fn validate_join(
    selected: &[u32],
    snapshots: &[GatedSnapshot],
) -> Result<(), SessionError> {
    if !selected.contains(&quest_log::CAPABILITY_QUEST_LOG_V1) {
        return Ok(());
    }
    let snapshot = snapshots
        .iter()
        .find(|s| s.domain_id == 16)
        .ok_or(SessionError::MissingDomain(16))?;
    if snapshot.snapshot_type != 1 {
        return Err(SessionError::UnregisteredSnapshotType {
            domain_id: 16,
            snapshot_type: snapshot.snapshot_type,
        });
    }
    quest_log::decode_quest_log(&snapshot.payload)
        .map_err(|_| SessionError::Protocol(FoundationProtocolError::MalformedFrame))?;
    Ok(())
}

pub(super) fn apply<S: SessionStream>(
    session: &mut Session<S>,
    sequence: u64,
    delta: oteryn_protocol_oteryn::StateDeltaView<'_>,
) -> Result<SessionEvent, SessionError> {
    if !session.is_selected(16) {
        return Err(SessionError::UnselectedDomain { domain_id: 16 });
    }
    let revision = session
        .gated_revisions
        .iter_mut()
        .find(|(id, _)| *id == 16)
        .ok_or(SessionError::MissingDomain(16))?;
    check_delta(&delta, 1, revision.1)?;
    let value = quest_log::decode_quest_log(delta.payload)
        .map_err(|_| SessionError::Protocol(FoundationProtocolError::MalformedFrame))?;
    revision.1 = delta.new_revision;
    Ok(SessionEvent::QuestLog(AppliedDelta {
        server_sequence: sequence,
        base_revision: delta.base_revision,
        new_revision: delta.new_revision,
        value,
    }))
}
