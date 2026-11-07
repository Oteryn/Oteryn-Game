//! Canary livestream.cpp:441,724: isolated live caster lines, independent of viewer delay.
use crate::cast::{
    CAPABILITY_CAST_CHAT_V1, CAPABILITY_CAST_CONTROL_V1, Message, STATE_DOMAIN_CAST_CASTER_CHAT,
};
use crate::*;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CasterLine {
    pub handle: [u8; 16],
    pub text: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Roster {
    pub revision: u64,
    pub subjects: Vec<[u8; 16]>,
}
fn invalid() -> SessionError {
    SessionError::SessionUnusable
}
pub fn decode_line(payload: &[u8]) -> Result<CasterLine, SessionError> {
    match Message::decode(payload).map_err(|_| invalid())? {
        Message::CasterLine { handle, text } => Ok(CasterLine { handle, text }),
        _ => Err(invalid()),
    }
}
pub(crate) fn validate_snapshot(
    snapshot_type: u32,
    revision: u64,
    payload: &[u8],
) -> Result<(), SessionError> {
    if snapshot_type != 1
        || revision == 0
        || Message::decode(payload).map_err(|_| invalid())? != Message::CasterChatEmpty
    {
        return Err(invalid());
    }
    Ok(())
}
pub(crate) fn validate_join(
    selected: &[u32],
    snapshots: &[GatedSnapshot],
) -> Result<(), SessionError> {
    if selected.contains(&CAPABILITY_CAST_CHAT_V1)
        && (!selected.contains(&CAPABILITY_CAST_CONTROL_V1)
            || !snapshots
                .iter()
                .any(|row| row.domain_id == STATE_DOMAIN_CAST_CASTER_CHAT))
    {
        return Err(SessionError::MissingDomain(STATE_DOMAIN_CAST_CASTER_CHAT));
    }
    Ok(())
}
pub(crate) fn apply<S: SessionStream>(
    session: &mut Session<S>,
    sequence: u64,
    payload: &[u8],
) -> Result<SessionEvent, SessionError> {
    let delta = decode_state_delta(payload)?;
    if !session.is_selected(CAPABILITY_CAST_CHAT_V1)
        || !session.is_selected(CAPABILITY_CAST_CONTROL_V1)
    {
        return Err(SessionError::UnselectedDomain {
            domain_id: STATE_DOMAIN_CAST_CASTER_CHAT,
        });
    }
    let revision = session
        .gated_revisions
        .iter()
        .find(|(id, _)| *id == STATE_DOMAIN_CAST_CASTER_CHAT)
        .map(|(_, revision)| *revision)
        .ok_or(SessionError::MissingDomain(STATE_DOMAIN_CAST_CASTER_CHAT))?;
    let message = Message::decode(delta.payload).map_err(|_| invalid())?;
    let expected = if matches!(message, Message::CasterRoster { .. }) {
        cast::DELTA_TYPE_CAST_ROSTER
    } else {
        1
    };
    check_delta(&delta, expected, revision)?;
    if revision.checked_add(1) != Some(delta.new_revision) {
        return Err(invalid());
    }
    let event = match message {
        Message::CasterLine { handle, text } => SessionEvent::CastCasterLine(AppliedDelta {
            server_sequence: sequence,
            base_revision: delta.base_revision,
            new_revision: delta.new_revision,
            value: CasterLine { handle, text },
        }),
        Message::CasterRoster { revision, subjects } => SessionEvent::CastRoster(AppliedDelta {
            server_sequence: sequence,
            base_revision: delta.base_revision,
            new_revision: delta.new_revision,
            value: Roster { revision, subjects },
        }),
        _ => return Err(invalid()),
    };
    let (_, stored) = session
        .gated_revisions
        .iter_mut()
        .find(|(id, _)| *id == STATE_DOMAIN_CAST_CASTER_CHAT)
        .ok_or(invalid())?;
    *stored = delta.new_revision;
    Ok(event)
}
