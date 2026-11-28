//! Connection-local presentation revisions; transient cues are never replayed from snapshots.
use super::*;
use oteryn_protocol_oteryn::presentation::*;

impl<S: SessionStream> Session<S> {
    pub(super) async fn apply_pushed_delta(
        &mut self,
        sequence: u64,
        payload: &[u8],
    ) -> Result<SessionEvent, SessionError> {
        let cooldown = decode_state_delta(payload)?;
        if cooldown.domain_id != STATE_DOMAIN_ACTOR_COOLDOWNS
            || cooldown.delta_type != DELTA_TYPE_ACTOR_COOLDOWNS_WITH_VITALS_V1
        {
            return self.apply_state_delta(sequence, payload);
        }
        if !self.is_selected(CAPABILITY_PRESENTATION_V1) {
            return Err(SessionError::UnselectedDomain {
                domain_id: cooldown.domain_id,
            });
        }
        let index = self
            .gated_revisions
            .iter()
            .position(|(domain, _)| *domain == STATE_DOMAIN_ACTOR_COOLDOWNS)
            .ok_or(SessionError::MissingDomain(STATE_DOMAIN_ACTOR_COOLDOWNS))?;
        check_delta(
            &cooldown,
            DELTA_TYPE_ACTOR_COOLDOWNS_WITH_VITALS_V1,
            self.gated_revisions[index].1,
        )?;
        let cooldowns = AppliedDelta {
            server_sequence: sequence,
            base_revision: cooldown.base_revision,
            new_revision: cooldown.new_revision,
            value: decode_update(
                cooldown.payload,
                !self.is_selected(CAPABILITY_ACTOR_COOLDOWN_DELTAS_V1),
            )?,
        };

        // Like the rest of a started frame, this continuation uses the frame
        // deadline even if the caller's idle window has ended.
        let frame = bounded(
            self.deadline,
            "Gift of Life vitals",
            read_frame(&mut self.stream),
        )
        .await?;
        self.last_received = tokio::time::Instant::now();
        let envelope = decode_wire_envelope(&frame)?;
        envelope.validate(Direction::ServerToClient, true)?;
        self.check_generation(&envelope)?;
        if let Some(error) = self.terminal_protocol_error(&envelope)? {
            return Err(error);
        }
        if envelope.message_type() != MessageType::StateDelta {
            return Err(SessionError::UnexpectedMessage {
                expected: MessageType::StateDelta,
                actual: envelope.message_type(),
            });
        }
        let sequence = self.next_sequence(&envelope)?;
        let delta = decode_state_delta(envelope.payload())?;
        if delta.domain_id != actor_spell::STATE_DOMAIN_ACTOR_VITALS {
            return Err(SessionError::UnexpectedDomain {
                expected: actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                actual: delta.domain_id,
            });
        }
        let SessionEvent::ActorVitals(vitals) =
            self.apply_state_delta(sequence, envelope.payload())?
        else {
            return Err(SessionError::SessionUnusable);
        };
        self.gated_revisions[index].1 = cooldowns.new_revision;
        Ok(SessionEvent::GiftOfLife { cooldowns, vitals })
    }
}

pub(super) fn validate_join(
    selected: &[u32],
    snapshots: &[GatedSnapshot],
) -> Result<Option<ActorCooldownUpdate>, SessionError> {
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
        return Ok(None);
    }
    let mut cooldowns = None;
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
            cooldowns = Some(decode_update(&row.payload, true)?);
        }
    }
    Ok(cooldowns)
}

fn decode_update(payload: &[u8], snapshot: bool) -> Result<ActorCooldownUpdate, SessionError> {
    let received_at = std::time::Instant::now();
    let value = decode_actor_cooldowns(payload).map_err(|_| SessionError::SessionUnusable)?;
    Ok(ActorCooldownUpdate {
        value,
        received_at,
        snapshot,
    })
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
    let cooldowns = delta.domain_id == STATE_DOMAIN_ACTOR_COOLDOWNS;
    let sparse_cooldowns = cooldowns && session.is_selected(CAPABILITY_ACTOR_COOLDOWN_DELTAS_V1);
    let row = session
        .gated_revisions
        .iter_mut()
        .find(|(id, _)| *id == delta.domain_id)
        .ok_or(SessionError::MissingDomain(delta.domain_id))?;
    let expected_type = if sparse_cooldowns {
        DELTA_TYPE_ACTOR_COOLDOWNS_SPARSE_V1
    } else if cooldowns {
        DELTA_TYPE_ACTOR_COOLDOWNS_V1
    } else if native_delta {
        2
    } else {
        1
    };
    check_delta(&delta, expected_type, row.1)?;
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
            value: decode_update(delta.payload, !sparse_cooldowns)?,
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
fn decode_events(
    payload: &[u8],
    native_delta: bool,
    native_basis: bool,
) -> Result<Vec<PresentationEvent>, SessionError> {
    if native_delta && !native_basis {
        return Err(SessionError::SessionUnusable);
    }
    if native_delta {
        oteryn_protocol_oteryn::native_map_cues::decode_native_map_presentation_batch(payload)
    } else {
        decode_presentation_batch(payload)
    }
    .map_err(|_| SessionError::SessionUnusable)
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
    fn srx3_native_epoch_preserves_global_positions_and_requires_native_marker()
    -> Result<(), String> {
        let legacy =
            encode_presentation_batch(&[missile(14, 0)]).map_err(|error| format!("{error:?}"))?;
        assert_eq!(
            decode_events(&legacy, false, false).map_err(|error| format!("{error:?}"))?,
            [missile(14, 0)]
        );
        assert_eq!(
            decode_events(&legacy, false, true).map_err(|error| format!("{error:?}"))?,
            [missile(14, 0)]
        );
        let native =
            oteryn_protocol_oteryn::native_map_cues::encode_native_map_presentation_batch(&[
                missile(14, 13),
            ])
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(
            decode_events(&native, true, true).map_err(|error| format!("{error:?}"))?,
            [missile(14, 13)]
        );
        assert!(decode_events(&native, true, false).is_err());
        assert_eq!(
            decode_events(&native, false, true).map_err(|error| format!("{error:?}"))?,
            [missile(14, 13)]
        );
        assert_eq!(
            decode_events(&legacy, true, true).map_err(|error| format!("{error:?}"))?,
            [missile(14, 0)]
        );
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
        assert!(matches!(
            line,
            oteryn_protocol_oteryn::chat::ChatLine::Local {
                position: ActorPosition { floor: 14, .. },
                ..
            }
        ));
        Ok(())
    }
}
