//! Display-only cast media. Map chunks contain no item handles, origins,
//! definitions or entity identities. Only visible effects and local speech fit.
use super::*;
use crate::{chat::ChatSpeechMode, presentation as effects, world_spatial::ActorPosition};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayTile {
    pub x: u16,
    pub y: u16,
    pub floor: i16,
    pub appearances: Vec<u16>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublicMedia {
    /// Display plane only; no world position or authority identity.
    Camera {
        floor: i16,
    },
    Map {
        reset: bool,
        last: bool,
        tiles: Vec<DisplayTile>,
    },
    Effects(Vec<effects::PresentationEvent>),
    Speech {
        position: ActorPosition,
        mode: ChatSpeechMode,
        text: String,
    },
}
impl PublicMedia {
    pub fn encode(&self) -> Result<Vec<u8>, WireError> {
        let mut b = Vec::new();
        match self {
            Self::Camera { floor } => {
                if !(-15..=0).contains(floor) {
                    return Err(WireError::Malformed);
                }
                // CLCAST lane: next free nested kind 4 + 30 = 34.
                push_varint_field(&mut b, 1, 34);
                push_varint_field(&mut b, 2, (*floor + 15) as u64);
            }
            Self::Map { reset, last, tiles } => {
                if tiles.len() > 32 {
                    return Err(WireError::LimitExceeded);
                }
                push_varint_field(&mut b, 1, 1);
                push_varint_field(&mut b, 2, u64::from(*reset));
                push_varint_field(&mut b, 3, u64::from(*last));
                for t in tiles {
                    if !(-15..=0).contains(&t.floor) || t.appearances.len() > 10 {
                        return Err(WireError::Malformed);
                    }
                    let mut v = Vec::new();
                    for (f, n) in [
                        (1, u64::from(t.x)),
                        (2, u64::from(t.y)),
                        (3, (t.floor + 15) as u64),
                    ] {
                        push_varint_field(&mut v, f, n);
                    }
                    for a in &t.appearances {
                        push_varint_field(&mut v, 4, u64::from(*a));
                    }
                    push_message_field(&mut b, 4, &v);
                }
            }
            Self::Effects(events) => {
                if events.is_empty()
                    || events.iter().any(|e| {
                        !matches!(
                            e.kind,
                            effects::PresentationEventKind::MagicEffect { .. }
                                | effects::PresentationEventKind::Missile { .. }
                        )
                    })
                {
                    return Err(WireError::Malformed);
                }
                push_varint_field(&mut b, 1, 2);
                let bytes =
                    effects::encode_presentation_batch(events).map_err(|_| WireError::Malformed)?;
                push_message_field(&mut b, 2, &bytes);
            }
            Self::Speech {
                position,
                mode,
                text,
            } => {
                if !(-15..=0).contains(&position.floor) {
                    return Err(WireError::Malformed);
                }
                let x = u16::try_from(position.x).map_err(|_| WireError::Malformed)?;
                let y = u16::try_from(position.y).map_err(|_| WireError::Malformed)?;
                push_varint_field(&mut b, 1, 3);
                for (f, n) in [
                    (2, u64::from(x)),
                    (3, u64::from(y)),
                    (4, (position.floor + 15) as u64),
                    (5, *mode as u64),
                ] {
                    push_varint_field(&mut b, f, n);
                }
                text_field(&mut b, 6, text)?;
            }
        }
        if b.len() > 8192 {
            return Err(WireError::LimitExceeded);
        }
        Ok(b)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, WireError> {
        let mut r = Reader { bytes, cursor: 0 };
        let value = match r.var(1)? {
            34 => Self::Camera {
                floor: i16::try_from(r.var(2)?).map_err(|_| WireError::Malformed)? - 15,
            },
            1 => {
                let reset = r.boolean(2)?;
                let last = r.boolean(3)?;
                let mut tiles = Vec::new();
                while r.bytes.get(r.cursor) == Some(&34) {
                    if tiles.len() >= 32 {
                        return Err(WireError::LimitExceeded);
                    }
                    let data = r.data(4, 128)?;
                    let mut t = Reader {
                        bytes: data,
                        cursor: 0,
                    };
                    let x = t.narrow(1)?;
                    let y = t.narrow(2)?;
                    let floor = i16::try_from(t.var(3)?).map_err(|_| WireError::Malformed)? - 15;
                    let mut appearances = Vec::new();
                    while t.cursor < data.len() {
                        if appearances.len() >= 10 {
                            return Err(WireError::LimitExceeded);
                        }
                        appearances.push(t.narrow(4)?);
                    }
                    tiles.push(DisplayTile {
                        x,
                        y,
                        floor,
                        appearances,
                    });
                }
                Self::Map { reset, last, tiles }
            }
            2 => Self::Effects(
                effects::decode_presentation_batch(r.data(2, 8192)?)
                    .map_err(|_| WireError::Malformed)?,
            ),
            3 => {
                let x = i32::from(r.narrow::<u16>(2)?);
                let y = i32::from(r.narrow::<u16>(3)?);
                let floor = i16::try_from(r.var(4)?).map_err(|_| WireError::Malformed)? - 15;
                let mode = match r.var(5)? {
                    1 => ChatSpeechMode::Say,
                    2 => ChatSpeechMode::Whisper,
                    3 => ChatSpeechMode::Yell,
                    4 => ChatSpeechMode::MonsterSay,
                    _ => return Err(WireError::Malformed),
                };
                Self::Speech {
                    position: ActorPosition { x, y, floor },
                    mode,
                    text: r.text(6)?,
                }
            }
            _ => return Err(WireError::Malformed),
        };
        if r.cursor != bytes.len() || value.encode()?.as_slice() != bytes {
            return Err(WireError::Malformed);
        }
        Ok(value)
    }
}
