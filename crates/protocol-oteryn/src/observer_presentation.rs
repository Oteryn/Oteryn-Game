//! PAR-M10 observer presentation (capability 262): a transient world magic effect or monster
//! speech line for the players who see it, as CHAT domain 12 delta type 6.
//!
//! Crystal 201189f8 `game.cpp:8950-8961` sends a magic effect to every player spectator of the
//! position (multifloor, `map/spectators.cpp:98-190`); `game.cpp:6941-6960` sends a say-range
//! line (`TALKTYPE_MONSTER_SAY`, `utils/utils_definitions.hpp:390`) to the spectators on the
//! speaker's floor. Neither is stored or replayed.
use std::num::NonZeroU64;

pub use crate::charm_wire::CyclopediaWireError as Error;
use crate::charm_wire::{push_message_field, push_varint_field, read_bytes, read_varint, set_once};
use crate::chat::ChatSpeaker;
use crate::world_spatial::{ActorPosition, decode_position, encode_position};

pub const CAPABILITY_OBSERVER_PRESENTATION_V1: u32 = 262;
/// A negotiated extension of CHAT domain 12; it shares the CHAT revision stream.
pub const DELTA_TYPE_OBSERVER_PRESENTATION_V1: u32 = 6;
/// Crystal `MagicEffectClasses` is a `uint16_t`; 0 (`CONST_ME_NONE`) sends nothing.
pub const MAX_MAGIC_EFFECT_ID: u32 = 65_535;
/// Crystal `utils/utils_definitions.hpp:58,87,99,109`.
pub const MAGIC_EFFECT_POFF: u32 = 3;
pub const MAGIC_EFFECT_STUN: u32 = 32;
pub const MAGIC_EFFECT_ICEATTACK: u32 = 44;
pub const MAGIC_EFFECT_WATERSPLASH: u32 = 54;
/// The CHAT line text bound.
pub const MAX_SAY_TEXT_BYTES: usize = 255;
pub const MAX_FLOOR: i16 = 15;
pub const MAX_COORDINATE: i32 = 65_535;
/// A monster say at the coordinate, generation and text bounds: tag and two-byte length (3),
/// identity (18), generation (11), position (12), text (258).
pub const MAX_PRESENTATION_BYTES: usize = 302;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObserverPresentation {
    MagicEffect {
        position: ActorPosition,
        effect_id: u32,
    },
    MonsterSay {
        speaker: ChatSpeaker,
        position: ActorPosition,
        text: String,
    },
}

fn check_position(position: &ActorPosition) -> Result<(), Error> {
    if (0..=MAX_FLOOR).contains(&position.floor)
        && (0..=MAX_COORDINATE).contains(&position.x)
        && (0..=MAX_COORDINATE).contains(&position.y)
    {
        Ok(())
    } else {
        Err(Error::Malformed)
    }
}

fn check(presentation: &ObserverPresentation) -> Result<(), Error> {
    match presentation {
        ObserverPresentation::MagicEffect {
            position,
            effect_id,
        } => {
            check_position(position)?;
            if !(1..=MAX_MAGIC_EFFECT_ID).contains(effect_id) {
                return Err(Error::Malformed);
            }
        }
        ObserverPresentation::MonsterSay { position, text, .. } => {
            check_position(position)?;
            if text.is_empty() {
                return Err(Error::Malformed);
            }
            if text.len() > MAX_SAY_TEXT_BYTES {
                return Err(Error::LimitExceeded);
            }
        }
    }
    Ok(())
}

pub fn encode(presentation: &ObserverPresentation) -> Result<Vec<u8>, Error> {
    check(presentation)?;
    let mut inner = Vec::with_capacity(MAX_PRESENTATION_BYTES);
    let field = match presentation {
        ObserverPresentation::MagicEffect {
            position,
            effect_id,
        } => {
            push_message_field(&mut inner, 1, &encode_position(position));
            push_varint_field(&mut inner, 2, u64::from(*effect_id));
            1
        }
        ObserverPresentation::MonsterSay {
            speaker,
            position,
            text,
        } => {
            push_message_field(&mut inner, 1, &speaker.identity);
            push_varint_field(&mut inner, 2, speaker.generation.get());
            push_message_field(&mut inner, 3, &encode_position(position));
            push_message_field(&mut inner, 4, text.as_bytes());
            2
        }
    };
    let mut out = Vec::with_capacity(inner.len() + 3);
    push_message_field(&mut out, field, &inner);
    Ok(out)
}

fn read_position(input: &[u8], cursor: &mut usize) -> Result<ActorPosition, Error> {
    decode_position(read_bytes(input, cursor)?).map_err(|_| Error::Malformed)
}

fn decode_effect(input: &[u8]) -> Result<ObserverPresentation, Error> {
    let (mut position, mut effect_id) = (None, None);
    let mut cursor = 0;
    while cursor < input.len() {
        match read_varint(input, &mut cursor)? {
            10 => set_once(&mut position, read_position(input, &mut cursor)?)?,
            16 => set_once(
                &mut effect_id,
                u32::try_from(read_varint(input, &mut cursor)?).map_err(|_| Error::Malformed)?,
            )?,
            _ => return Err(Error::Malformed),
        }
    }
    Ok(ObserverPresentation::MagicEffect {
        position: position.ok_or(Error::Malformed)?,
        effect_id: effect_id.ok_or(Error::Malformed)?,
    })
}

fn decode_say(input: &[u8]) -> Result<ObserverPresentation, Error> {
    let (mut identity, mut generation, mut position, mut text) = (None, None, None, None);
    let mut cursor = 0;
    while cursor < input.len() {
        match read_varint(input, &mut cursor)? {
            10 => set_once(
                &mut identity,
                read_bytes(input, &mut cursor)?
                    .try_into()
                    .map_err(|_| Error::Malformed)?,
            )?,
            16 => set_once(
                &mut generation,
                NonZeroU64::new(read_varint(input, &mut cursor)?).ok_or(Error::Malformed)?,
            )?,
            26 => set_once(&mut position, read_position(input, &mut cursor)?)?,
            34 => set_once(
                &mut text,
                String::from_utf8(read_bytes(input, &mut cursor)?.to_vec())
                    .map_err(|_| Error::Malformed)?,
            )?,
            _ => return Err(Error::Malformed),
        }
    }
    Ok(ObserverPresentation::MonsterSay {
        speaker: ChatSpeaker {
            identity: identity.ok_or(Error::Malformed)?,
            generation: generation.ok_or(Error::Malformed)?,
        },
        position: position.ok_or(Error::Malformed)?,
        text: text.ok_or(Error::Malformed)?,
    })
}

/// Exactly one of the two events; unknown, repeated or out-of-bound fields are refused.
pub fn decode(payload: &[u8]) -> Result<ObserverPresentation, Error> {
    if payload.len() > MAX_PRESENTATION_BYTES {
        return Err(Error::LimitExceeded);
    }
    let mut cursor = 0;
    let presentation = match read_varint(payload, &mut cursor)? {
        10 => decode_effect(read_bytes(payload, &mut cursor)?)?,
        18 => decode_say(read_bytes(payload, &mut cursor)?)?,
        _ => return Err(Error::Malformed),
    };
    if cursor != payload.len() {
        return Err(Error::Malformed);
    }
    check(&presentation)?;
    Ok(presentation)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn position() -> ActorPosition {
        ActorPosition {
            x: MAX_COORDINATE,
            y: MAX_COORDINATE,
            floor: MAX_FLOOR,
        }
    }

    fn say(text: &str) -> Result<ObserverPresentation, Error> {
        Ok(ObserverPresentation::MonsterSay {
            speaker: ChatSpeaker {
                identity: [7; 16],
                generation: NonZeroU64::new(u64::MAX).ok_or(Error::Malformed)?,
            },
            position: position(),
            text: text.to_owned(),
        })
    }

    #[test]
    fn observer_presentation_round_trips_effects_and_monster_say_at_their_bounds()
    -> Result<(), Error> {
        for effect_id in [
            1,
            MAGIC_EFFECT_POFF,
            MAGIC_EFFECT_STUN,
            MAGIC_EFFECT_ICEATTACK,
            MAGIC_EFFECT_WATERSPLASH,
            MAX_MAGIC_EFFECT_ID,
        ] {
            let effect = ObserverPresentation::MagicEffect {
                position: position(),
                effect_id,
            };
            assert_eq!(decode(&encode(&effect)?)?, effect);
        }
        let longest = say(&"x".repeat(MAX_SAY_TEXT_BYTES))?;
        let bytes = encode(&longest)?;
        assert_eq!(bytes.len(), MAX_PRESENTATION_BYTES);
        assert_eq!(decode(&bytes)?, longest);
        assert_eq!(decode(&encode(&say("Faaart!")?)?)?, say("Faaart!")?);
        Ok(())
    }

    #[test]
    fn observer_presentation_refuses_out_of_bound_missing_repeated_and_unknown_fields()
    -> Result<(), Error> {
        for effect_id in [0, MAX_MAGIC_EFFECT_ID + 1] {
            assert!(
                encode(&ObserverPresentation::MagicEffect {
                    position: position(),
                    effect_id,
                })
                .is_err()
            );
        }
        for floor in [-1, MAX_FLOOR + 1] {
            let mut at = position();
            at.floor = floor;
            assert!(
                encode(&ObserverPresentation::MagicEffect {
                    position: at,
                    effect_id: 1,
                })
                .is_err()
            );
        }
        assert!(encode(&say("")?).is_err());
        assert_eq!(
            encode(&say(&"x".repeat(MAX_SAY_TEXT_BYTES + 1))?),
            Err(Error::LimitExceeded)
        );
        let good = encode(&say("Faaart!")?)?;
        for end in 0..good.len() {
            assert!(decode(&good[..end]).is_err());
        }
        let mut repeated = good.clone();
        repeated.extend_from_slice(&good);
        assert!(decode(&repeated).is_err());
        assert!(decode(&[26, 0]).is_err());
        let effect = encode(&ObserverPresentation::MagicEffect {
            position: position(),
            effect_id: 3,
        })?;
        let mut unknown = effect.clone();
        unknown[1] += 2;
        unknown.extend_from_slice(&[24, 0]);
        assert!(decode(&unknown).is_err());
        assert!(decode(&[0; MAX_PRESENTATION_BYTES + 1]).is_err());
        Ok(())
    }
}
