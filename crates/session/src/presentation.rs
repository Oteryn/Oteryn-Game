//! Connection-local presentation revisions; transient cues are never replayed from snapshots.
use super::*;
use oteryn_protocol_oteryn::presentation::*;

pub(super) fn validate_join(
    selected: &[u32],
    snapshots: &[GatedSnapshot],
) -> Result<(), SessionError> {
    if selected.contains(&CAPABILITY_ITEM_EFFECTS_V1) {
        let row = snapshots
            .iter()
            .find(|r| r.domain_id == STATE_DOMAIN_ITEM_EFFECTS)
            .ok_or(SessionError::MissingDomain(STATE_DOMAIN_ITEM_EFFECTS))?;
        if row.snapshot_type != 1 || !row.payload.is_empty() {
            return Err(SessionError::SessionUnusable);
        }
    }
    use oteryn_protocol_oteryn::native_map_cues::{
        CAPABILITY_NATIVE_MAP_CUES_V1, SNAPSHOT_TYPE_NATIVE_MAP_CUE_BASIS_V1,
    };
    for row in snapshots.iter().filter(|row| {
        row.domain_id == STATE_DOMAIN_WORLD_PRESENTATION
            && row.snapshot_type == SNAPSHOT_TYPE_NATIVE_MAP_CUE_BASIS_V1
    }) {
        if ![
            CAPABILITY_PRESENTATION_V1,
            30,
            CAPABILITY_NATIVE_MAP_CUES_V1,
        ]
        .iter()
        .all(|capability| selected.contains(capability))
            || !row.payload.is_empty()
        {
            return Err(SessionError::SessionUnusable);
        }
    }
    if selected.contains(&CAPABILITY_AMBIENT_SOUND_V1) {
        let row = snapshots
            .iter()
            .find(|row| row.domain_id == STATE_DOMAIN_AMBIENT_SOUND)
            .ok_or(SessionError::MissingDomain(STATE_DOMAIN_AMBIENT_SOUND))?;
        if row.snapshot_type != 1 || !row.payload.is_empty() {
            return Err(SessionError::SessionUnusable);
        }
    }
    if !selected.contains(&CAPABILITY_PRESENTATION_V1) {
        return Ok(());
    }
    for domain in [
        STATE_DOMAIN_WORLD_PRESENTATION,
        STATE_DOMAIN_ACTOR_COOLDOWNS,
    ] {
        let row = snapshots
            .iter()
            .find(|row| row.domain_id == domain)
            .ok_or(SessionError::MissingDomain(domain))?;
        let native_basis = domain == STATE_DOMAIN_WORLD_PRESENTATION
            && row.snapshot_type == SNAPSHOT_TYPE_NATIVE_MAP_CUE_BASIS_V1
            && selected.contains(&CAPABILITY_NATIVE_MAP_CUES_V1);
        if row.snapshot_type != 1 && !native_basis {
            return Err(SessionError::SessionUnusable);
        }
        if domain == STATE_DOMAIN_WORLD_PRESENTATION {
            decode_presentation_snapshot(&row.payload)
                .map_err(|_| SessionError::SessionUnusable)?;
        } else {
            decode_actor_cooldowns(&row.payload).map_err(|_| SessionError::SessionUnusable)?;
        }
    }
    Ok(())
}

pub(super) fn apply<S: SessionStream>(
    session: &mut Session<S>,
    sequence: u64,
    payload: &[u8],
) -> Result<SessionEvent, SessionError> {
    let delta = decode_state_delta(payload)?;
    let ambient = delta.domain_id == STATE_DOMAIN_AMBIENT_SOUND;
    let item = delta.domain_id == STATE_DOMAIN_ITEM_EFFECTS;
    if ambient && delta.payload.len() > MAX_PRESENTATION_EVENT_BYTES {
        return Err(SessionError::SessionUnusable);
    }
    if !session.is_selected(if item {
        CAPABILITY_ITEM_EFFECTS_V1
    } else if ambient {
        CAPABILITY_AMBIENT_SOUND_V1
    } else {
        CAPABILITY_PRESENTATION_V1
    }) {
        return Err(SessionError::UnselectedDomain {
            domain_id: delta.domain_id,
        });
    }
    let native_basis = native_basis(session);
    let native_delta = !ambient
        && delta.domain_id == STATE_DOMAIN_WORLD_PRESENTATION
        && delta.delta_type
            == oteryn_protocol_oteryn::native_map_cues::DELTA_TYPE_NATIVE_MAP_PRESENTATION_V1;
    if native_delta && !native_basis {
        return Err(SessionError::SessionUnusable);
    }
    let row = session
        .gated_revisions
        .iter_mut()
        .find(|(id, _)| *id == delta.domain_id)
        .ok_or(SessionError::MissingDomain(delta.domain_id))?;
    check_delta(&delta, if native_delta { 2 } else { 1 }, row.1)?;
    let event = if item {
        SessionEvent::ItemEffects(AppliedDelta {
            server_sequence: sequence,
            base_revision: delta.base_revision,
            new_revision: delta.new_revision,
            value: decode_item_effect_batch(delta.payload)
                .map_err(|_| SessionError::SessionUnusable)?,
        })
    } else if ambient || delta.domain_id == STATE_DOMAIN_WORLD_PRESENTATION {
        let value = decode_events(delta.payload, native_delta, native_basis && !ambient)?;
        if ambient
            && (value.len() != 1
                || !matches!(value[0].kind, PresentationEventKind::AmbientSound { .. }))
        {
            return Err(SessionError::SessionUnusable);
        }
        if !ambient
            && value
                .iter()
                .any(|event| matches!(event.kind, PresentationEventKind::AmbientSound { .. }))
        {
            return Err(SessionError::SessionUnusable);
        }
        SessionEvent::Presentation(AppliedDelta {
            server_sequence: sequence,
            base_revision: delta.base_revision,
            new_revision: delta.new_revision,
            value,
        })
    } else {
        SessionEvent::ActorCooldowns(AppliedDelta {
            server_sequence: sequence,
            base_revision: delta.base_revision,
            new_revision: delta.new_revision,
            value: decode_actor_cooldowns(delta.payload)
                .map_err(|_| SessionError::SessionUnusable)?,
        })
    };
    row.1 = delta.new_revision;
    Ok(event)
}

pub(super) fn native_basis<S: SessionStream>(session: &Session<S>) -> bool {
    session.is_selected(oteryn_protocol_oteryn::native_map_cues::CAPABILITY_NATIVE_MAP_CUES_V1)
        && session.gated_snapshots.iter().any(|row| row.domain_id == STATE_DOMAIN_WORLD_PRESENTATION
            && row.snapshot_type == oteryn_protocol_oteryn::native_map_cues::SNAPSHOT_TYPE_NATIVE_MAP_CUE_BASIS_V1)
}
fn native_position(
    position: &mut oteryn_protocol_oteryn::world_spatial::ActorPosition,
) -> Result<(), SessionError> {
    // Only called after the strict legacy decoder accepted 0..=15.
    position.floor = position
        .floor
        .checked_neg()
        .ok_or(SessionError::SessionUnusable)?;
    Ok(())
}
fn project_legacy_events(events: &mut [PresentationEvent]) -> Result<(), SessionError> {
    for event in events {
        match &mut event.kind {
            PresentationEventKind::AmbientSound { .. } => {}
            PresentationEventKind::MagicEffect { position, .. }
            | PresentationEventKind::SpellWords { position, .. }
            | PresentationEventKind::ValueText { position, .. }
            | PresentationEventKind::Sound { position, .. } => native_position(position)?,
            PresentationEventKind::Missile { from, to, .. } => {
                native_position(from)?;
                native_position(to)?;
            }
        }
    }
    Ok(())
}
pub(super) fn project_legacy_chat(
    line: &mut oteryn_protocol_oteryn::chat::ChatLine,
) -> Result<(), SessionError> {
    if let oteryn_protocol_oteryn::chat::ChatLine::Local { position, .. } = line {
        native_position(position)?;
    }
    Ok(())
}

fn decode_events(
    payload: &[u8],
    native_delta: bool,
    native_basis: bool,
) -> Result<Vec<PresentationEvent>, SessionError> {
    if native_delta && !native_basis {
        return Err(SessionError::SessionUnusable);
    }
    let mut events = if native_delta {
        oteryn_protocol_oteryn::native_map_cues::decode_native_map_presentation_batch(payload)
    } else {
        decode_presentation_batch(payload)
    }
    .map_err(|_| SessionError::SessionUnusable)?;
    if native_basis && !native_delta {
        project_legacy_events(&mut events)?;
    }
    Ok(events)
}

#[cfg(test)]
mod native_basis_tests {
    use super::*;
    use oteryn_protocol_oteryn::world_spatial::ActorPosition;
    fn missile(from: i16, to: i16) -> PresentationEvent {
        PresentationEvent {
            decision_ordinal: 1,
            emission_ordinal: 1,
            kind: PresentationEventKind::Missile {
                from: ActorPosition {
                    x: 1,
                    y: 2,
                    floor: from,
                },
                to: ActorPosition {
                    x: 3,
                    y: 4,
                    floor: to,
                },
                missile_id: 31,
                source: PresentationSource::Creatures,
            },
        }
    }
    #[test]
    fn srx3_native_epoch_projects_legacy_positions_once_and_refuses_wrong_basis()
    -> Result<(), String> {
        let legacy =
            encode_presentation_batch(&[missile(14, 0)]).map_err(|error| format!("{error:?}"))?;
        assert_eq!(
            decode_events(&legacy, false, false).map_err(|error| format!("{error:?}"))?,
            [missile(14, 0)]
        );
        assert_eq!(
            decode_events(&legacy, false, true).map_err(|error| format!("{error:?}"))?,
            [missile(-14, 0)]
        );
        let native =
            oteryn_protocol_oteryn::native_map_cues::encode_native_map_presentation_batch(&[
                missile(-14, -13),
            ])
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(
            decode_events(&native, true, true).map_err(|error| format!("{error:?}"))?,
            [missile(-14, -13)]
        );
        assert!(decode_events(&native, true, false).is_err());
        assert!(decode_events(&native, false, true).is_err());
        assert!(decode_events(&legacy, true, true).is_err());
        let mut line = oteryn_protocol_oteryn::chat::ChatLine::Local {
            speaker: oteryn_protocol_oteryn::chat::ChatSpeaker {
                identity: [1; 16],
                generation: std::num::NonZeroU64::MIN,
            },
            speaker_name: "Human Speaker".into(),
            mode: oteryn_protocol_oteryn::chat::ChatSpeechMode::Say,
            text: "Source speech".into(),
            position: ActorPosition {
                x: 1,
                y: 2,
                floor: 14,
            },
        };
        let bytes = oteryn_protocol_oteryn::chat::encode_chat_line(&line)
            .map_err(|error| format!("{error:?}"))?;
        line = oteryn_protocol_oteryn::chat::decode_chat_line(&bytes)
            .map_err(|error| format!("{error:?}"))?;
        project_legacy_chat(&mut line).map_err(|error| format!("{error:?}"))?;
        assert!(matches!(
            line,
            oteryn_protocol_oteryn::chat::ChatLine::Local {
                position: ActorPosition { floor: -14, .. },
                ..
            }
        ));
        Ok(())
    }
}
