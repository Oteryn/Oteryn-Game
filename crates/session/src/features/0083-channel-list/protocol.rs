//! Served Game Channel, distinct from chat rooms.
//! Accepted difference: GAME-CHANNEL-01_CHANNEL_PRODUCT_POLICY_CONTRACT.md:588-596,615-618.
use crate::{GatedSnapshot, SessionError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServedChannel {
    pub world_id: [u8; 16],
    pub channel_id: [u8; 16],
    pub premium_only: bool,
}

pub fn decode(bytes: &[u8]) -> Result<ServedChannel, SessionError> {
    // oteryn-choice: accept the bounded one-entry contract and reject ambiguous fields.
    if bytes.len() != 40 || bytes[..2] != [10, 38] {
        return Err(SessionError::SessionUnusable);
    }
    let mut at = 2;
    let (mut world_id, mut channel_id, mut premium_only) = (None, None, None);
    while at < bytes.len() {
        let key = bytes[at];
        at += 1;
        match key {
            10 | 18 => {
                if bytes.get(at) != Some(&16) {
                    return Err(SessionError::SessionUnusable);
                }
                at += 1;
                let id: [u8; 16] = bytes
                    .get(at..at + 16)
                    .ok_or(SessionError::SessionUnusable)?
                    .try_into()
                    .map_err(|_| SessionError::SessionUnusable)?;
                at += 16;
                let slot = if key == 10 {
                    &mut world_id
                } else {
                    &mut channel_id
                };
                if slot.replace(id).is_some() {
                    return Err(SessionError::SessionUnusable);
                }
            }
            24 => {
                let value = match bytes.get(at) {
                    Some(0) => false,
                    Some(1) => true,
                    _ => return Err(SessionError::SessionUnusable),
                };
                at += 1;
                if premium_only.replace(value).is_some() {
                    return Err(SessionError::SessionUnusable);
                }
            }
            _ => return Err(SessionError::SessionUnusable),
        }
    }
    Ok(ServedChannel {
        world_id: world_id.ok_or(SessionError::SessionUnusable)?,
        channel_id: channel_id.ok_or(SessionError::SessionUnusable)?,
        premium_only: premium_only.ok_or(SessionError::SessionUnusable)?,
    })
}

pub(crate) fn validate_join(
    selected: &[u32],
    snapshots: &[GatedSnapshot],
) -> Result<(), SessionError> {
    if selected.contains(&83) {
        let row = snapshots
            .iter()
            .find(|row| row.domain_id == 48)
            .ok_or(SessionError::MissingDomain(48))?;
        if row.snapshot_type != 1 {
            return Err(SessionError::SessionUnusable);
        }
        decode(&row.payload)?;
    }
    Ok(())
}
