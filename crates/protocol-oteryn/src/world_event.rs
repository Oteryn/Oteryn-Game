//! Reset-selected world gathering announcement. World/session come from admission.
//! Query is empty; unknown fields, duplicate fields and oversized results fail closed.

pub use crate::charm_wire::CyclopediaWireError as WorldEventWireError;
use crate::charm_wire::{
    WireResult, push_message_field, push_nonzero_varint_field, read_bytes, read_varint, set_once,
};

pub const CAPABILITY_WORLD_EVENT_V1: u32 = 53;
pub const COMMAND_TYPE_WORLD_EVENT_QUERY: u32 = 64;
pub const MAX_ANNOUNCEMENT_BYTES: usize = 128;
pub const MAX_WORLD_EVENT_RESULT_BYTES: usize = 144;
const _: () = assert!(MAX_WORLD_EVENT_RESULT_BYTES <= crate::MAX_COMMAND_RESULT_PAYLOAD_BYTES);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldEventState {
    pub reset_epoch: u64,
    pub active: bool,
    pub announcement: String,
}

fn validate(state: &WorldEventState) -> WireResult<()> {
    if state.announcement.len() > MAX_ANNOUNCEMENT_BYTES {
        return Err(WorldEventWireError::LimitExceeded);
    }
    if state.active == state.announcement.is_empty()
        || (state.active && state.reset_epoch == 0)
        || state.announcement.chars().any(char::is_control)
    {
        return Err(WorldEventWireError::Malformed);
    }
    Ok(())
}

pub fn decode_world_event_query(payload: &[u8]) -> WireResult<()> {
    if payload.is_empty() {
        Ok(())
    } else {
        Err(WorldEventWireError::Malformed)
    }
}

pub fn encode_world_event_state(state: &WorldEventState) -> WireResult<Vec<u8>> {
    validate(state)?;
    let mut output = Vec::with_capacity(MAX_WORLD_EVENT_RESULT_BYTES);
    push_nonzero_varint_field(&mut output, 1, state.reset_epoch);
    push_nonzero_varint_field(&mut output, 2, u64::from(state.active));
    if !state.announcement.is_empty() {
        push_message_field(&mut output, 3, state.announcement.as_bytes());
    }
    Ok(output)
}

pub fn decode_world_event_state(payload: &[u8]) -> WireResult<WorldEventState> {
    if payload.len() > MAX_WORLD_EVENT_RESULT_BYTES {
        return Err(WorldEventWireError::LimitExceeded);
    }
    let (mut epoch, mut active, mut announcement) = (None, None, None);
    let mut cursor = 0;
    while cursor < payload.len() {
        match read_varint(payload, &mut cursor)? {
            0x08 => set_once(&mut epoch, read_varint(payload, &mut cursor)?)?,
            0x10 => {
                let value = read_varint(payload, &mut cursor)?;
                if value > 1 {
                    return Err(WorldEventWireError::Malformed);
                }
                set_once(&mut active, value == 1)?;
            }
            0x1a => {
                let bytes = read_bytes(payload, &mut cursor)?;
                if bytes.len() > MAX_ANNOUNCEMENT_BYTES {
                    return Err(WorldEventWireError::LimitExceeded);
                }
                let value =
                    std::str::from_utf8(bytes).map_err(|_| WorldEventWireError::Malformed)?;
                set_once(&mut announcement, value.to_owned())?;
            }
            _ => return Err(WorldEventWireError::Malformed),
        }
    }
    let state = WorldEventState {
        reset_epoch: epoch.unwrap_or(0),
        active: active.unwrap_or(false),
        announcement: announcement.unwrap_or_default(),
    };
    validate(&state)?;
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_event_codec_roundtrips_and_bounds() -> WireResult<()> {
        for state in [
            WorldEventState {
                reset_epoch: 0,
                active: false,
                announcement: String::new(),
            },
            WorldEventState {
                reset_epoch: u64::MAX,
                active: true,
                announcement: "a".repeat(128),
            },
        ] {
            let bytes = encode_world_event_state(&state)?;
            assert!(bytes.len() <= MAX_WORLD_EVENT_RESULT_BYTES);
            assert_eq!(decode_world_event_state(&bytes)?, state);
        }
        assert!(decode_world_event_query(&[]).is_ok());
        assert!(decode_world_event_query(&[0]).is_err());
        for bytes in [
            vec![8, 1, 8, 1],
            vec![16, 0, 16, 0],
            vec![26, 0, 26, 0],
            vec![16, 2],
            vec![8, 0x80],
            vec![32, 1],
            vec![26, 1, 0xff],
            vec![8, 1, 16, 1],
            vec![26, 1, b'a'],
            vec![0; 145],
        ] {
            assert!(decode_world_event_state(&bytes).is_err(), "{bytes:?}");
        }
        assert!(
            encode_world_event_state(&WorldEventState {
                reset_epoch: 1,
                active: true,
                announcement: "a".repeat(129),
            })
            .is_err()
        );
        assert!(
            encode_world_event_state(&WorldEventState {
                reset_epoch: 1,
                active: true,
                announcement: "\n".into(),
            })
            .is_err()
        );
        Ok(())
    }
}
