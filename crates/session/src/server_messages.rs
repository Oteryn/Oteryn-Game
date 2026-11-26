//! Selected domain-57 messages; transient raid announcements are never replayed on join.
//! Capability 576 adds type 2 Party join and leave notices on the same domain revision.
use super::*;
use oteryn_protocol_oteryn::zones_nologout::{
    CAPABILITY_BOSS_DEFEATED_V1, CAPABILITY_ZONES_NOLOGOUT_V1, SERVER_MESSAGES_EMPTY_SNAPSHOT,
    STATE_DOMAIN_SERVER_MESSAGES, decode_server_message,
};

pub(super) fn validate_join(
    selected: &[u32],
    snapshots: &[GatedSnapshot],
    sequence: u64,
) -> Result<Vec<SessionEvent>, SessionError> {
    if !selected.contains(&CAPABILITY_ZONES_NOLOGOUT_V1) {
        return Ok(Vec::new());
    }
    let snapshot = snapshots
        .iter()
        .find(|row| row.domain_id == STATE_DOMAIN_SERVER_MESSAGES)
        .ok_or(SessionError::MissingDomain(STATE_DOMAIN_SERVER_MESSAGES))?;
    if snapshot.snapshot_type != 1 {
        return Err(SessionError::Protocol(
            FoundationProtocolError::MalformedFrame,
        ));
    }
    if snapshot.payload == SERVER_MESSAGES_EMPTY_SNAPSHOT {
        return Ok(Vec::new());
    }
    let value = decode_server_message(&snapshot.payload)
        .map_err(|_| SessionError::Protocol(FoundationProtocolError::MalformedFrame))?;
    if !matches!(value, ServerMessage::StatusText(_)) {
        return Err(SessionError::Protocol(
            FoundationProtocolError::MalformedFrame,
        ));
    }
    Ok(vec![SessionEvent::ServerMessage(AppliedDelta {
        server_sequence: sequence,
        base_revision: snapshot.revision,
        new_revision: snapshot.revision,
        value,
    })])
}

pub(super) fn apply<S: SessionStream>(
    session: &mut Session<S>,
    sequence: u64,
    payload: &[u8],
) -> Result<SessionEvent, SessionError> {
    if !session.is_selected(CAPABILITY_ZONES_NOLOGOUT_V1) {
        return Err(SessionError::UnselectedDomain {
            domain_id: STATE_DOMAIN_SERVER_MESSAGES,
        });
    }
    let boss_defeats = session.is_selected(CAPABILITY_BOSS_DEFEATED_V1);
    let party_notices = session.is_selected(party::CAPABILITY_PARTY_MEMBERSHIP_NOTICES_V1);
    let delta = decode_state_delta(payload)?;
    let entry = session
        .gated_revisions
        .iter_mut()
        .find(|(id, _)| *id == STATE_DOMAIN_SERVER_MESSAGES)
        .ok_or(SessionError::MissingDomain(STATE_DOMAIN_SERVER_MESSAGES))?;
    if delta.delta_type == party::DELTA_TYPE_PARTY_MEMBERSHIP_NOTICE_V1 {
        if !party_notices {
            return Err(SessionError::CapabilityNotSelected {
                capability: party::CAPABILITY_PARTY_MEMBERSHIP_NOTICES_V1,
            });
        }
        check_delta(
            &delta,
            party::DELTA_TYPE_PARTY_MEMBERSHIP_NOTICE_V1,
            entry.1,
        )?;
        let value = party::decode_party_membership_notice(delta.payload)
            .map_err(|_| SessionError::Protocol(FoundationProtocolError::MalformedFrame))?;
        entry.1 = delta.new_revision;
        return Ok(SessionEvent::PartyMembershipNotice(AppliedDelta {
            server_sequence: sequence,
            base_revision: delta.base_revision,
            new_revision: delta.new_revision,
            value,
        }));
    }
    check_delta(&delta, 1, entry.1)?;
    let value = decode_server_message(delta.payload)
        .map_err(|_| SessionError::Protocol(FoundationProtocolError::MalformedFrame))?;
    if matches!(value, ServerMessage::BossDefeated(_)) && !boss_defeats {
        return Err(SessionError::CapabilityNotSelected {
            capability: CAPABILITY_BOSS_DEFEATED_V1,
        });
    }
    entry.1 = delta.new_revision;
    Ok(SessionEvent::ServerMessage(AppliedDelta {
        server_sequence: sequence,
        base_revision: delta.base_revision,
        new_revision: delta.new_revision,
        value,
    }))
}
