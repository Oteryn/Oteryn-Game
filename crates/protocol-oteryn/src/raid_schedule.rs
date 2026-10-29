//! Read-only raid progress observed by an admitted player. Raid execution remains
//! entirely server driven: this command cannot start, advance, or cancel a raid.
use crate::charm_wire::{
    CyclopediaWireError, WireResult, push_message_field, push_nonzero_varint_field, read_bytes,
    read_varint, set_once,
};
pub const CAPABILITY_RAID_SCHEDULE_V1: u32 = 149;
pub const COMMAND_TYPE_RAID_SCHEDULE_QUERY: u32 = 171;
pub const COMMAND_TYPE_RAID_SUBSCRIBE: u32 = 172;
pub const COMMAND_TYPE_RAID_NOTICE_ACK: u32 = 173;
pub const MAX_RAID_KEY_BYTES: usize = 128;
pub const MAX_RAID_QUERY_RESULT_BYTES: usize = 192;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaidScheduleState {
    pub raid_key: String,
    pub running: bool,
    pub committed_steps: u32,
    pub active_creatures: u32,
}

pub fn decode_raid_schedule_query(payload: &[u8]) -> WireResult<()> {
    if payload.is_empty() {
        Ok(())
    } else {
        Err(CyclopediaWireError::Malformed)
    }
}

pub fn encode_raid_schedule_state(state: &RaidScheduleState) -> WireResult<Vec<u8>> {
    if state.raid_key.len() > MAX_RAID_KEY_BYTES
        || state.raid_key.chars().any(char::is_control)
        || (state.raid_key.is_empty()
            && (state.running || state.committed_steps != 0 || state.active_creatures != 0))
    {
        return Err(CyclopediaWireError::Malformed);
    }
    let mut out = Vec::with_capacity(MAX_RAID_QUERY_RESULT_BYTES);
    if !state.raid_key.is_empty() {
        push_message_field(&mut out, 1, state.raid_key.as_bytes());
    }
    push_nonzero_varint_field(&mut out, 2, u64::from(state.running));
    push_nonzero_varint_field(&mut out, 3, u64::from(state.committed_steps));
    push_nonzero_varint_field(&mut out, 4, u64::from(state.active_creatures));
    Ok(out)
}

pub fn decode_raid_schedule_state(payload: &[u8]) -> WireResult<RaidScheduleState> {
    if payload.len() > MAX_RAID_QUERY_RESULT_BYTES {
        return Err(CyclopediaWireError::LimitExceeded);
    }
    let (mut key, mut running, mut steps, mut creatures) = (None, None, None, None);
    let mut cursor = 0;
    while cursor < payload.len() {
        match read_varint(payload, &mut cursor)? {
            0x0a => {
                let value = read_bytes(payload, &mut cursor)?;
                set_once(
                    &mut key,
                    std::str::from_utf8(value)
                        .map_err(|_| CyclopediaWireError::Malformed)?
                        .to_owned(),
                )?
            }
            0x10 => {
                let value = read_varint(payload, &mut cursor)?;
                if value > 1 {
                    return Err(CyclopediaWireError::Malformed);
                }
                set_once(&mut running, value == 1)?
            }
            0x18 => set_once(
                &mut steps,
                u32::try_from(read_varint(payload, &mut cursor)?)
                    .map_err(|_| CyclopediaWireError::LimitExceeded)?,
            )?,
            0x20 => set_once(
                &mut creatures,
                u32::try_from(read_varint(payload, &mut cursor)?)
                    .map_err(|_| CyclopediaWireError::LimitExceeded)?,
            )?,
            _ => return Err(CyclopediaWireError::Malformed),
        }
    }
    let state = RaidScheduleState {
        raid_key: key.unwrap_or_default(),
        running: running.unwrap_or(false),
        committed_steps: steps.unwrap_or(0),
        active_creatures: creatures.unwrap_or(0),
    };
    encode_raid_schedule_state(&state)?;
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn state_round_trip_and_query_is_empty() -> WireResult<()> {
        let state = RaidScheduleState {
            raid_key: "raid:badger".into(),
            running: true,
            committed_steps: 1,
            active_creatures: 100,
        };
        assert_eq!(
            decode_raid_schedule_state(&encode_raid_schedule_state(&state)?)?,
            state
        );
        assert!(decode_raid_schedule_query(&[]).is_ok());
        assert!(decode_raid_schedule_query(&[0]).is_err());
        Ok(())
    }
}
