//! oteryn-choice: typed facts use the existing selected-domain revision store; no history is replayed.
use super::*;
pub use oteryn_protocol_oteryn::analyser::*;

pub(super) fn validate_join(
    selected: &[u32],
    snapshots: &[GatedSnapshot],
) -> Result<(), SessionError> {
    if !selected.contains(&CAPABILITY_ANALYSER_V1) {
        return Ok(());
    }
    let row = snapshots
        .iter()
        .find(|row| row.domain_id == STATE_DOMAIN_ACTOR_ANALYSER)
        .ok_or(SessionError::MissingDomain(STATE_DOMAIN_ACTOR_ANALYSER))?;
    validate_snapshot(row, None)
}

// oteryn-choice: recovery may skip revisions but cannot roll back this session's fact stream.
pub(super) fn validate_snapshot(
    row: &GatedSnapshot,
    previous: Option<u64>,
) -> Result<(), SessionError> {
    if row.snapshot_type != SNAPSHOT_TYPE_ANALYSER_V1
        || row.revision == 0
        || !row.payload.is_empty()
        || previous.is_some_and(|revision| row.revision < revision)
    {
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
    if !session.is_selected(CAPABILITY_ANALYSER_V1) {
        return Err(SessionError::UnselectedDomain {
            domain_id: STATE_DOMAIN_ACTOR_ANALYSER,
        });
    }
    let delta = decode_state_delta(payload)?;
    let row = session
        .gated_revisions
        .iter_mut()
        .find(|(id, _)| *id == STATE_DOMAIN_ACTOR_ANALYSER)
        .ok_or(SessionError::MissingDomain(STATE_DOMAIN_ACTOR_ANALYSER))?;
    check_delta(&delta, DELTA_TYPE_ANALYSER_FACTS_V1, row.1)?;
    let malformed = |_| SessionError::Protocol(FoundationProtocolError::MalformedFrame);
    if session
        .selected_capabilities
        .contains(&CAPABILITY_ANALYSER_HIT_ATTRIBUTION_V1)
    {
        let value = decode_attributed_analyser_facts(delta.payload).map_err(malformed)?;
        row.1 = delta.new_revision;
        return Ok(SessionEvent::AnalyserAttributedFacts(AppliedDelta {
            server_sequence: sequence,
            base_revision: delta.base_revision,
            new_revision: delta.new_revision,
            value,
        }));
    }
    let value = decode_analyser_facts(delta.payload).map_err(malformed)?;
    row.1 = delta.new_revision;
    Ok(SessionEvent::AnalyserFacts(AppliedDelta {
        server_sequence: sequence,
        base_revision: delta.base_revision,
        new_revision: delta.new_revision,
        value,
    }))
}
