//! Capability-8 achievement notice routing through the selected-domain revision store.
use super::*;
use oteryn_protocol_oteryn::achievement_notices::*;

pub(super) fn validate_join(
    selected: &[u32],
    snapshots: &[GatedSnapshot],
) -> Result<(), SessionError> {
    if !selected.contains(&CAPABILITY_ACHIEVEMENT_NOTICES_V1) {
        return Ok(());
    }
    let snapshot = snapshots
        .iter()
        .find(|row| row.domain_id == STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES)
        .ok_or(SessionError::MissingDomain(
            STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES,
        ))?;
    if snapshot.snapshot_type != 1 {
        return Err(SessionError::UnregisteredSnapshotType {
            domain_id: snapshot.domain_id,
            snapshot_type: snapshot.snapshot_type,
        });
    }
    decode_achievement_notices_snapshot(&snapshot.payload)
        .map(|_| ())
        .map_err(|_| SessionError::Protocol(FoundationProtocolError::MalformedFrame))
}

pub(super) fn apply<S: SessionStream>(
    session: &mut Session<S>,
    sequence: u64,
    payload: &[u8],
) -> Result<SessionEvent, SessionError> {
    if !session.is_selected(CAPABILITY_ACHIEVEMENT_NOTICES_V1) {
        return Err(SessionError::UnselectedDomain {
            domain_id: STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES,
        });
    }
    let delta = decode_state_delta(payload)?;
    let entry = session
        .gated_revisions
        .iter_mut()
        .find(|(id, _)| *id == STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES)
        .ok_or(SessionError::MissingDomain(
            STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES,
        ))?;
    check_delta(&delta, 1, entry.1)?;
    let value = decode_achievement_earned(delta.payload)
        .map_err(|_| SessionError::Protocol(FoundationProtocolError::MalformedFrame))?;
    entry.1 = delta.new_revision;
    Ok(SessionEvent::AchievementEarned(AppliedDelta {
        server_sequence: sequence,
        base_revision: delta.base_revision,
        new_revision: delta.new_revision,
        value,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_join_requires_and_decodes_domain_13() {
        let selected = [CAPABILITY_ACHIEVEMENT_NOTICES_V1];
        assert!(matches!(
            validate_join(&selected, &[]),
            Err(SessionError::MissingDomain(
                STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES
            ))
        ));
        let snapshot = GatedSnapshot {
            domain_id: STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES,
            snapshot_type: 1,
            revision: 0,
            payload: encode_achievement_notices_snapshot(AchievementWatermark::default()),
        };
        assert!(validate_join(&selected, &[snapshot]).is_ok());
    }
}
