//! Server-owned world-event announcement and seasonal calendar. World/session come from admission.
//! Query is empty; unknown fields, duplicate fields and oversized results fail closed.

pub use crate::charm_wire::CyclopediaWireError as WorldEventWireError;
use crate::charm_wire::{
    WireResult, push_message_field, push_nonzero_varint_field, read_bytes, read_varint, set_once,
};

pub const CAPABILITY_WORLD_EVENT_V1: u32 = 53;
pub const CAPABILITY_SEASONAL_EVENTS_V1: u32 = 371;
pub const COMMAND_TYPE_WORLD_EVENT_QUERY: u32 = 64;
pub const MAX_ANNOUNCEMENT_BYTES: usize = 128;
// The prior 256-byte bound gains the bounded goal field (3 bytes) and reserves
// one byte for a possible nested-message length-prefix increase.
pub const MAX_WORLD_EVENT_RESULT_BYTES: usize = 260;
const _: () = assert!(MAX_WORLD_EVENT_RESULT_BYTES <= crate::MAX_COMMAND_RESULT_PAYLOAD_BYTES);

/// Seasonal gameplay event identifiers. The server owns windows and activation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SeasonalEventKind {
    Halloween = 1,
    Christmas = 2,
    BirthdayCake = 3,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeasonalEventState {
    pub event: SeasonalEventKind,
    pub active: bool,
    pub start_unix: u64,
    pub end_unix: u64,
}

fn validate_seasonal(event: &SeasonalEventState) -> WireResult<()> {
    if event.start_unix == 0
        || event.end_unix <= event.start_unix
        || event.end_unix > 253_402_300_799
    {
        return Err(WorldEventWireError::Malformed);
    }
    Ok(())
}

fn encode_seasonal(event: &SeasonalEventState) -> Vec<u8> {
    let mut output = Vec::new();
    push_nonzero_varint_field(&mut output, 1, event.event as u64);
    push_nonzero_varint_field(&mut output, 2, u64::from(event.active));
    push_nonzero_varint_field(&mut output, 3, event.start_unix);
    push_nonzero_varint_field(&mut output, 4, event.end_unix);
    output
}

fn decode_seasonal(payload: &[u8]) -> WireResult<SeasonalEventState> {
    let (mut event, mut active, mut start, mut end) = (None, None, None, None);
    let mut cursor = 0;
    while cursor < payload.len() {
        match read_varint(payload, &mut cursor)? {
            0x08 => set_once(
                &mut event,
                match read_varint(payload, &mut cursor)? {
                    1 => SeasonalEventKind::Halloween,
                    2 => SeasonalEventKind::Christmas,
                    3 => SeasonalEventKind::BirthdayCake,
                    _ => return Err(WorldEventWireError::Malformed),
                },
            )?,
            0x10 => {
                let value = read_varint(payload, &mut cursor)?;
                if value > 1 {
                    return Err(WorldEventWireError::Malformed);
                }
                set_once(&mut active, value == 1)?;
            }
            0x18 => set_once(&mut start, read_varint(payload, &mut cursor)?)?,
            0x20 => set_once(&mut end, read_varint(payload, &mut cursor)?)?,
            _ => return Err(WorldEventWireError::Malformed),
        }
    }
    let value = SeasonalEventState {
        event: event.ok_or(WorldEventWireError::Malformed)?,
        active: active.unwrap_or(false),
        start_unix: start.unwrap_or(0),
        end_unix: end.unwrap_or(0),
    };
    validate_seasonal(&value)?;
    Ok(value)
}

/// Current A Piece of Cake progress, observed by the authoritative server clock.
/// Optional under SEASONAL_EVENTS_V1; retained earned bonuses outlive the event calendar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CakeProgress {
    pub stage: u8,
    pub substances: u16,
    pub golems: u16,
    pub layer: u8,
    pub layer_progress: u16,
    pub layer_goal: u16,
    pub occurrence_start: u64,
    pub occurrence_end: u64,
    pub window_end: Option<u64>,
    pub bonus_until: Option<u64>,
    pub observed_unix: u64,
}

fn validate_cake(value: &CakeProgress) -> WireResult<()> {
    if value.stage > 3
        || value.substances > 200
        || value.golems > 200
        || !(1..=4).contains(&value.layer)
        || !(1..=1116).contains(&value.layer_goal)
        || value.layer_progress > value.layer_goal
        || value.observed_unix == 0
        || value.observed_unix > 253_402_300_799
        || value.occurrence_start == 0
        || value.occurrence_end <= value.occurrence_start
        || value.occurrence_end > 253_402_300_799
        || value
            .window_end
            .is_some_and(|end| !(1800..=253_402_300_799).contains(&end))
        || value
            .bonus_until
            .is_some_and(|end| !(604800..=253_402_300_799).contains(&end))
    {
        return Err(WorldEventWireError::Malformed);
    }
    Ok(())
}

fn encode_cake(value: &CakeProgress) -> Vec<u8> {
    let mut bytes = Vec::new();
    for (field, value) in [
        (1, u64::from(value.stage)),
        (2, u64::from(value.substances)),
        (3, u64::from(value.golems)),
        (4, u64::from(value.layer)),
        (5, u64::from(value.layer_progress)),
        (6, value.occurrence_start),
        (7, value.occurrence_end),
        (8, value.window_end.unwrap_or(0)),
        (9, value.bonus_until.unwrap_or(0)),
        (10, value.observed_unix),
        (11, u64::from(value.layer_goal)),
    ] {
        push_nonzero_varint_field(&mut bytes, field, value);
    }
    bytes
}

fn decode_cake(payload: &[u8]) -> WireResult<CakeProgress> {
    let mut values = [None; 11];
    let mut cursor = 0;
    while cursor < payload.len() {
        let tag = read_varint(payload, &mut cursor)?;
        if tag & 7 != 0 || !(1..=11).contains(&(tag >> 3)) {
            return Err(WorldEventWireError::Malformed);
        }
        set_once(
            &mut values[(tag >> 3) as usize - 1],
            read_varint(payload, &mut cursor)?,
        )?;
    }
    let value = CakeProgress {
        stage: u8::try_from(values[0].unwrap_or(0)).map_err(|_| WorldEventWireError::Malformed)?,
        substances: u16::try_from(values[1].unwrap_or(0))
            .map_err(|_| WorldEventWireError::Malformed)?,
        golems: u16::try_from(values[2].unwrap_or(0))
            .map_err(|_| WorldEventWireError::Malformed)?,
        layer: u8::try_from(values[3].unwrap_or(0)).map_err(|_| WorldEventWireError::Malformed)?,
        layer_progress: u16::try_from(values[4].unwrap_or(0))
            .map_err(|_| WorldEventWireError::Malformed)?,
        layer_goal: u16::try_from(values[10].unwrap_or(0))
            .map_err(|_| WorldEventWireError::Malformed)?,
        occurrence_start: values[5].unwrap_or(0),
        occurrence_end: values[6].unwrap_or(0),
        window_end: values[7],
        bonus_until: values[8],
        observed_unix: values[9].unwrap_or(0),
    };
    validate_cake(&value)?;
    Ok(value)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldEventState {
    pub reset_epoch: u64,
    pub active: bool,
    pub announcement: String,
    pub seasonal: Vec<SeasonalEventState>,
    pub cake: Option<CakeProgress>,
}

fn validate(state: &WorldEventState) -> WireResult<()> {
    if let Some(cake) = &state.cake {
        validate_cake(cake)?;
    }
    if state.seasonal.len() > 3 {
        return Err(WorldEventWireError::LimitExceeded);
    }
    for (index, event) in state.seasonal.iter().enumerate() {
        validate_seasonal(event)?;
        if state.seasonal[..index]
            .iter()
            .any(|previous| previous.event == event.event)
        {
            return Err(WorldEventWireError::Malformed);
        }
    }
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
    for event in &state.seasonal {
        push_message_field(&mut output, 4, &encode_seasonal(event));
    }
    if let Some(cake) = &state.cake {
        push_message_field(&mut output, 5, &encode_cake(cake));
    }
    if output.len() > MAX_WORLD_EVENT_RESULT_BYTES {
        return Err(WorldEventWireError::LimitExceeded);
    }
    Ok(output)
}

pub fn decode_world_event_state(payload: &[u8]) -> WireResult<WorldEventState> {
    if payload.len() > MAX_WORLD_EVENT_RESULT_BYTES {
        return Err(WorldEventWireError::LimitExceeded);
    }
    let (mut epoch, mut active, mut announcement) = (None, None, None);
    let mut seasonal = Vec::new();
    let mut cake = None;
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
            0x22 => {
                if seasonal.len() == 3 {
                    return Err(WorldEventWireError::LimitExceeded);
                }
                seasonal.push(decode_seasonal(read_bytes(payload, &mut cursor)?)?);
            }
            0x2a => set_once(&mut cake, decode_cake(read_bytes(payload, &mut cursor)?)?)?,
            _ => return Err(WorldEventWireError::Malformed),
        }
    }
    let state = WorldEventState {
        reset_epoch: epoch.unwrap_or(0),
        active: active.unwrap_or(false),
        announcement: announcement.unwrap_or_default(),
        seasonal,
        cake,
    };
    validate(&state)?;
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn w05d_seasonal_world_event_wire_is_decoded() -> WireResult<()> {
        // Field 4 is a server-authored Halloween window, independent of reset epoch.
        let bytes = [34, 8, 8, 1, 16, 1, 24, 1, 32, 2];
        let state = decode_world_event_state(&bytes)?;
        assert!(!state.active);
        assert_eq!(encode_world_event_state(&state)?, bytes);
        Ok(())
    }

    #[test]
    fn w05d_seasonal_world_event_wire_rejects_bad_windows_and_duplicates() -> WireResult<()> {
        for bytes in [
            vec![34, 6, 8, 0, 24, 1, 32, 2],
            vec![34, 6, 8, 4, 24, 1, 32, 2],
            vec![34, 8, 8, 1, 16, 2, 24, 1, 32, 2],
            vec![34, 8, 8, 1, 8, 1, 24, 1, 32, 2],
            vec![34, 6, 8, 1, 24, 2, 32, 1],
            vec![34, 4, 8, 1, 32, 2],
            vec![34, 6, 8, 1, 24, 1, 32, 1],
            vec![34, 6, 8, 1, 24, 1, 32, 2, 34, 6, 8, 1, 24, 1, 32, 2],
        ] {
            assert!(decode_world_event_state(&bytes).is_err(), "{bytes:?}");
        }
        assert_eq!(
            decode_world_event_state(&[0; MAX_WORLD_EVENT_RESULT_BYTES + 1]),
            Err(WorldEventWireError::LimitExceeded),
        );
        let mut state = WorldEventState {
            reset_epoch: 0,
            active: false,
            announcement: String::new(),
            seasonal: [
                SeasonalEventKind::Halloween,
                SeasonalEventKind::Christmas,
                SeasonalEventKind::BirthdayCake,
            ]
            .into_iter()
            .map(|event| SeasonalEventState {
                event,
                active: true,
                start_unix: 1,
                end_unix: 253_402_300_799,
            })
            .collect(),
            cake: None,
        };
        let bytes = encode_world_event_state(&state)?;
        assert_eq!(decode_world_event_state(&bytes)?, state);
        state.seasonal.push(state.seasonal[0].clone());
        assert!(encode_world_event_state(&state).is_err());
        state.seasonal.truncate(1);
        state.seasonal[0].end_unix = 253_402_300_800;
        assert!(encode_world_event_state(&state).is_err());
        Ok(())
    }

    #[test]
    fn world_event_codec_roundtrips_and_bounds() -> WireResult<()> {
        for state in [
            WorldEventState {
                reset_epoch: 0,
                active: false,
                announcement: String::new(),
                seasonal: Vec::new(),
                cake: None,
            },
            WorldEventState {
                reset_epoch: u64::MAX,
                active: true,
                announcement: "a".repeat(128),
                seasonal: Vec::new(),
                cake: None,
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
                seasonal: Vec::new(),
                cake: None,
            })
            .is_err()
        );
        assert!(
            encode_world_event_state(&WorldEventState {
                reset_epoch: 1,
                active: true,
                announcement: "\n".into(),
                seasonal: Vec::new(),
                cake: None,
            })
            .is_err()
        );
        Ok(())
    }
    #[test]
    fn cake_progress_retains_earned_bonus_after_calendar_cleanup() -> Result<(), String> {
        let mut state = WorldEventState {
            reset_epoch: 0,
            active: false,
            announcement: String::new(),
            seasonal: Vec::new(),
            cake: None,
        };
        state.cake = Some(CakeProgress {
            stage: 0,
            substances: 200,
            golems: 200,
            layer: 4,
            layer_progress: 1116,
            layer_goal: 1116,
            occurrence_start: 1_799_000_000,
            occurrence_end: 1_799_518_400,
            window_end: None,
            bonus_until: Some(1_800_604_800),
            observed_unix: 1_800_100_000,
        });
        for event in &mut state.seasonal {
            event.active = false;
        }
        let wire = encode_world_event_state(&state)
            .map_err(|error| format!("valid retained bonus: {error:?}"))?;
        assert_eq!(
            decode_world_event_state(&wire)
                .map_err(|error| format!("retained state decode: {error:?}"))?,
            state
        );
        for malformed in [
            vec![8, 4],
            vec![32, 5],
            vec![80, 1, 80, 1],
            vec![48, 1, 80, 1],
        ] {
            assert!(decode_cake(&malformed).is_err());
        }
        Ok(())
    }

    #[test]
    fn seasonal_calendar_and_maximum_cake_result_fit_260_byte_bound() -> Result<(), String> {
        let maximum = 253_402_300_799;
        let state = WorldEventState {
            reset_epoch: u64::MAX,
            active: true,
            announcement: "a".repeat(128),
            seasonal: [
                SeasonalEventKind::Halloween,
                SeasonalEventKind::Christmas,
                SeasonalEventKind::BirthdayCake,
            ]
            .into_iter()
            .map(|event| SeasonalEventState {
                event,
                active: true,
                start_unix: maximum - 1,
                end_unix: maximum,
            })
            .collect(),
            cake: Some(CakeProgress {
                stage: 3,
                substances: 200,
                golems: 200,
                layer: 4,
                layer_progress: 1116,
                layer_goal: 1116,
                occurrence_start: maximum - 518400,
                occurrence_end: maximum,
                window_end: Some(maximum),
                bonus_until: Some(maximum),
                observed_unix: maximum,
            }),
        };
        let bytes = encode_world_event_state(&state)
            .map_err(|error| format!("maximum legal state: {error:?}"))?;
        assert_eq!(bytes.len(), 257);
        assert_eq!(
            decode_world_event_state(&bytes)
                .map_err(|error| format!("maximum state decode: {error:?}"))?,
            state
        );
        Ok(())
    }

    #[test]
    fn cake_progress_wire_is_decoded() {
        // Server progress field5: stage0, layer1, occurrence[1,2), observed1.
        assert!(decode_world_event_state(&[42, 10, 32, 1, 48, 1, 56, 2, 80, 1, 88, 1]).is_ok());
    }

    #[test]
    fn cake_progress_rejects_invalid_counters_deadlines_and_repeated_fields() {
        let valid = CakeProgress {
            stage: 0,
            substances: 200,
            golems: 200,
            layer: 4,
            layer_progress: 1116,
            layer_goal: 1116,
            occurrence_start: 1,
            occurrence_end: 2,
            window_end: Some(1800),
            bonus_until: Some(604800),
            observed_unix: 1,
        };
        for edit in 0..12 {
            let mut bad = valid.clone();
            match edit {
                0 => bad.stage = 4,
                1 => bad.substances = 201,
                2 => bad.golems = 201,
                3 => bad.layer = 0,
                4 => bad.layer_progress = 1117,
                5 => bad.occurrence_start = 2,
                6 => bad.occurrence_end = 253_402_300_800,
                7 => bad.window_end = Some(1799),
                8 => bad.bonus_until = Some(604799),
                9 => bad.observed_unix = 0,
                10 => bad.layer_goal = 0,
                _ => bad.layer_goal = 1117,
            }
            assert!(validate_cake(&bad).is_err(), "invalid case {edit}");
        }
        let mut nested = encode_cake(&valid);
        nested.extend([32, 4]);
        assert!(decode_cake(&nested).is_err());
        let mut outer = Vec::new();
        push_message_field(&mut outer, 5, &encode_cake(&valid));
        push_message_field(&mut outer, 5, &encode_cake(&valid));
        assert!(decode_world_event_state(&outer).is_err());
    }
}
