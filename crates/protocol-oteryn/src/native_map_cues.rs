//! Qualified native-map cue coordinates. The current epoch snapshot declares
//! the basis; legacy positive-floor codecs remain separate and unchanged.
use crate::charm_wire::WireResult;
use crate::{chat, presentation};
pub const CAPABILITY_NATIVE_MAP_CUES_V1: u32 = 385;
pub const CAPABILITY_NATIVE_MAP_CUES_V1_REQUIRES: [u32; 2] = [23, 30];
pub const SNAPSHOT_TYPE_NATIVE_MAP_CUE_BASIS_V1: u32 = 2;
pub const DELTA_TYPE_NATIVE_MAP_PRESENTATION_V1: u32 = 2;
pub const DELTA_TYPE_NATIVE_MAP_CHAT_LINE_V1: u32 = 6;

// The negotiated epoch basis selects strict native floors; legacy codecs stay positive.
pub fn encode_native_map_presentation_batch(
    events: &[presentation::PresentationEvent],
) -> WireResult<Vec<u8>> {
    presentation::encode_presentation_batch_with_basis(events, true)
}
pub fn decode_native_map_presentation_batch(
    payload: &[u8],
) -> WireResult<Vec<presentation::PresentationEvent>> {
    presentation::decode_presentation_batch_with_basis(payload, true)
}
pub fn encode_native_map_chat_line(line: &chat::ChatLine) -> WireResult<Vec<u8>> {
    if !matches!(line, chat::ChatLine::Local { .. }) {
        return Err(crate::charm_wire::CyclopediaWireError::Malformed);
    }
    chat::encode_chat_line_with_basis(line, true)
}
pub fn decode_native_map_chat_line(payload: &[u8]) -> WireResult<chat::ChatLine> {
    let line = chat::decode_chat_line_with_basis(payload, true)?;
    if !matches!(line, chat::ChatLine::Local { .. }) {
        return Err(crate::charm_wire::CyclopediaWireError::Malformed);
    }
    Ok(line)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_spatial::ActorPosition;
    fn event(floor: i16) -> presentation::PresentationEvent {
        presentation::PresentationEvent {
            decision_ordinal: 1,
            emission_ordinal: 1,
            kind: presentation::PresentationEventKind::MagicEffect {
                position: ActorPosition {
                    x: 33358,
                    y: 31187,
                    floor,
                },
                effect_id: 13,
                source: presentation::PresentationSource::Creatures,
            },
        }
    }
    fn speech(floor: i16, mode: chat::ChatSpeechMode) -> chat::ChatLine {
        chat::ChatLine::Local {
            speaker: chat::ChatSpeaker {
                identity: [1; 16],
                generation: std::num::NonZeroU64::MIN,
            },
            speaker_name: "Source Speaker".into(),
            mode,
            text: "The source cue.".into(),
            position: ActorPosition {
                x: 33358,
                y: 31187,
                floor,
            },
        }
    }
    #[test]
    fn srx3_native_map_presentation_codec_accepts_negative_floor_without_loosening_legacy()
    -> WireResult<()> {
        for floor in [-15, -14, 0] {
            let events = vec![event(floor)];
            let raw = encode_native_map_presentation_batch(&events)?;
            assert_eq!(decode_native_map_presentation_batch(&raw)?, events);
            if floor < 0 {
                assert!(presentation::encode_presentation_batch(&events).is_err());
                assert!(presentation::decode_presentation_batch(&raw).is_err());
            }
        }
        assert!(encode_native_map_presentation_batch(&[event(1)]).is_err());
        assert!(encode_native_map_presentation_batch(&[event(-16)]).is_err());
        Ok(())
    }
    #[test]
    fn srx3_native_map_chat_codec_covers_human_and_monster_local_speech() -> WireResult<()> {
        for mode in [
            chat::ChatSpeechMode::Say,
            chat::ChatSpeechMode::Whisper,
            chat::ChatSpeechMode::Yell,
            chat::ChatSpeechMode::MonsterSay,
        ] {
            let line = speech(-14, mode);
            let raw = encode_native_map_chat_line(&line)?;
            assert_eq!(decode_native_map_chat_line(&raw)?, line);
            assert!(chat::encode_chat_line(&line).is_err());
            assert!(chat::decode_chat_line(&raw).is_err());
        }
        assert!(encode_native_map_chat_line(&speech(1, chat::ChatSpeechMode::Say)).is_err());
        Ok(())
    }
    #[test]
    fn srx3_legacy_presentation_and_chat_bytes_remain_unchanged() -> WireResult<()> {
        let events = vec![event(14)];
        let raw = presentation::encode_presentation_batch(&events)?;
        assert_eq!(presentation::decode_presentation_batch(&raw)?, events);
        let line = speech(14, chat::ChatSpeechMode::Say);
        let bytes = chat::encode_chat_line(&line)?;
        assert_eq!(chat::decode_chat_line(&bytes)?, line);
        Ok(())
    }
    #[test]
    fn native_codec_preserves_structural_and_basis_refusals() -> WireResult<()> {
        let raw = presentation::encode_presentation_batch(&[event(14)])?;
        assert!(decode_native_map_presentation_batch(&raw).is_err());
        assert!(decode_native_map_presentation_batch(&[]).is_err());
        assert!(decode_native_map_presentation_batch(&[0]).is_err());
        assert!(encode_native_map_presentation_batch(&[event(-14), event(-14)]).is_err());
        let mut raw = encode_native_map_presentation_batch(&[event(-14)])?;
        raw.extend_from_slice(&[0]);
        assert!(decode_native_map_presentation_batch(&raw).is_err());
        let raw = chat::encode_chat_line(&speech(14, chat::ChatSpeechMode::Say))?;
        assert!(decode_native_map_chat_line(&raw).is_err());
        assert!(decode_native_map_chat_line(&[0]).is_err());
        assert!(encode_native_map_chat_line(&speech(-16, chat::ChatSpeechMode::Say)).is_err());
        // Floor zero has identical bytes in both explicitly selected coordinate bases.
        assert_eq!(
            encode_native_map_presentation_batch(&[event(0)])?,
            presentation::encode_presentation_batch(&[event(0)])?
        );
        let line = speech(0, chat::ChatSpeechMode::Say);
        assert_eq!(
            encode_native_map_chat_line(&line)?,
            chat::encode_chat_line(&line)?
        );
        Ok(())
    }
    #[test]
    fn native_codec_checks_every_position_bearing_presentation_variant() -> WireResult<()> {
        use presentation::{PresentationEventKind as K, PresentationSource as S};
        let position = ActorPosition {
            x: 1,
            y: 2,
            floor: -14,
        };
        let other = ActorPosition {
            x: 3,
            y: 4,
            floor: -13,
        };
        let actor = crate::world_spatial_entities::EntityRef {
            identity: [1; 16],
            generation: 1,
        };
        let kinds = [
            K::MagicEffect {
                position,
                effect_id: 1,
                source: S::Creatures,
            },
            K::Missile {
                from: position,
                to: other,
                missile_id: 1,
                source: S::Creatures,
            },
            K::SpellWords {
                caster: actor,
                position,
                spell: 1,
            },
            K::ValueText {
                kind: presentation::PresentationValueKind::Healed,
                position,
                target: actor,
                attacker: Some(actor),
                primary: presentation::PresentationValue {
                    value: 1,
                    color: presentation::PresentationColor::Red,
                },
                secondary: None,
            },
            K::Sound {
                position,
                cue_id: 1,
                source: S::Creatures,
                secondary_cue_id: 0,
            },
            K::ValueText {
                kind: presentation::PresentationValueKind::DamageOthers,
                position,
                target: actor,
                attacker: None,
                primary: presentation::PresentationValue {
                    value: 1,
                    color: presentation::PresentationColor::Red,
                },
                secondary: Some(presentation::PresentationValue {
                    value: 2,
                    color: presentation::PresentationColor::Blue,
                }),
            },
            K::Sound {
                position,
                cue_id: 1,
                source: S::Creatures,
                secondary_cue_id: 2,
            },
        ];
        for kind in kinds {
            let events = [presentation::PresentationEvent {
                decision_ordinal: 1,
                emission_ordinal: 1,
                kind,
            }];
            let bytes = encode_native_map_presentation_batch(&events)?;
            assert_eq!(decode_native_map_presentation_batch(&bytes)?, events);
            assert!(presentation::encode_presentation_batch(&events).is_err());
            assert!(presentation::decode_presentation_batch(&bytes).is_err());
        }
        let mixed = [presentation::PresentationEvent {
            decision_ordinal: 1,
            emission_ordinal: 1,
            kind: K::Missile {
                from: position,
                to: ActorPosition { floor: 1, ..other },
                missile_id: 1,
                source: S::Creatures,
            },
        }];
        assert!(encode_native_map_presentation_batch(&mixed).is_err());
        Ok(())
    }
}
