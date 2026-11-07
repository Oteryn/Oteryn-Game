//! Transient player typing command and negotiated CHAT-domain delta (S19).
//!
//! Crystal 201189f8: protocolgame.cpp:2818-2820 and :11314-11323. The
//! Oteryn capability replaces the donor's OTCR-only output gate.
use std::num::NonZeroU64;

pub use crate::charm_wire::CyclopediaWireError as Error;
use crate::charm_wire::{push_message_field, push_varint_field, read_bytes, read_varint, set_once};
use crate::chat::ChatSpeaker;

pub const CAPABILITY_PLAYER_TYPING_V1: u32 = 233;
pub const COMMAND_TYPE_PLAYER_TYPING: u32 = 447;
/// A negotiated extension of CHAT domain 12; existing deltas 1..=3 are unchanged.
pub const DELTA_TYPE_PLAYER_TYPING_V1: u32 = 4;
pub const MAX_INTENT_BYTES: usize = 2;
pub const MAX_UPDATE_BYTES: usize = 31;

/// The actor's current incarnation, never a character/account identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypingUpdate {
    pub speaker: ChatSpeaker,
    pub typing: bool,
}

pub fn encode_intent(typing: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(MAX_INTENT_BYTES);
    push_varint_field(&mut out, 1, u64::from(typing));
    out
}

/// Exactly one explicit Boolean; omission, duplicates and unknown fields are refused.
pub fn decode_intent(payload: &[u8]) -> Result<bool, Error> {
    match payload {
        [8, 0] => Ok(false),
        [8, 1] => Ok(true),
        _ if payload.len() > MAX_INTENT_BYTES => Err(Error::LimitExceeded),
        _ => Err(Error::Malformed),
    }
}

pub fn encode_update(update: TypingUpdate) -> Vec<u8> {
    let mut out = Vec::with_capacity(MAX_UPDATE_BYTES);
    push_message_field(&mut out, 1, &update.speaker.identity);
    push_varint_field(&mut out, 2, update.speaker.generation.get());
    push_varint_field(&mut out, 3, u64::from(update.typing));
    out
}

pub fn decode_update(payload: &[u8]) -> Result<TypingUpdate, Error> {
    if payload.len() > MAX_UPDATE_BYTES {
        return Err(Error::LimitExceeded);
    }
    let (mut identity, mut generation, mut typing) = (None, None, None);
    let mut cursor = 0;
    while cursor < payload.len() {
        match read_varint(payload, &mut cursor)? {
            10 => set_once(
                &mut identity,
                read_bytes(payload, &mut cursor)?
                    .try_into()
                    .map_err(|_| Error::Malformed)?,
            )?,
            16 => set_once(
                &mut generation,
                NonZeroU64::new(read_varint(payload, &mut cursor)?).ok_or(Error::Malformed)?,
            )?,
            24 => set_once(
                &mut typing,
                match read_varint(payload, &mut cursor)? {
                    0 => false,
                    1 => true,
                    _ => return Err(Error::Malformed),
                },
            )?,
            _ => return Err(Error::Malformed),
        }
    }
    Ok(TypingUpdate {
        speaker: ChatSpeaker {
            identity: identity.ok_or(Error::Malformed)?,
            generation: generation.ok_or(Error::Malformed)?,
        },
        typing: typing.ok_or(Error::Malformed)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_typing_intent_requires_one_boolean() {
        for typing in [false, true] {
            assert_eq!(decode_intent(&encode_intent(typing)), Ok(typing));
        }
        for payload in [
            Vec::new(),
            vec![0],
            vec![8],
            vec![8, 2],
            vec![16, 1],
            vec![8, 0, 8, 1],
            vec![8, 1, 16, 0],
            vec![8, 0x80, 0],
        ] {
            assert!(decode_intent(&payload).is_err(), "{payload:?}");
        }
    }

    #[test]
    fn player_typing_update_round_trips_both_states() -> Result<(), Error> {
        for typing in [false, true] {
            let update = TypingUpdate {
                speaker: ChatSpeaker {
                    identity: [7; 16],
                    generation: NonZeroU64::new(u64::MAX).ok_or(Error::Malformed)?,
                },
                typing,
            };
            let bytes = encode_update(update);
            assert_eq!(bytes.len(), MAX_UPDATE_BYTES);
            assert_eq!(decode_update(&bytes)?, update);
        }
        Ok(())
    }

    #[test]
    fn player_typing_update_rejects_missing_duplicate_and_unknown_fields() -> Result<(), Error> {
        let update = TypingUpdate {
            speaker: ChatSpeaker {
                identity: [7; 16],
                generation: NonZeroU64::new(1).ok_or(Error::Malformed)?,
            },
            typing: true,
        };
        let good = encode_update(update);
        for end in 0..good.len() {
            assert!(decode_update(&good[..end]).is_err());
        }
        let mut zero_generation = good.clone();
        zero_generation[19] = 0;
        assert!(decode_update(&zero_generation).is_err());
        let mut not_boolean = good.clone();
        not_boolean[21] = 2;
        assert!(decode_update(&not_boolean).is_err());
        let mut repeated = good.clone();
        repeated.extend_from_slice(&[24, 0]);
        assert!(decode_update(&repeated).is_err());
        let mut unknown = good.clone();
        unknown.extend_from_slice(&[32, 0]);
        assert!(decode_update(&unknown).is_err());
        let mut wrong_identity = good.clone();
        wrong_identity[1] = 15;
        wrong_identity.remove(17);
        assert!(decode_update(&wrong_identity).is_err());
        assert!(decode_update(&[0; MAX_UPDATE_BYTES + 1]).is_err());
        Ok(())
    }
}
