//! Existing map callback receipts delivered on the native connection cadence.
use super::*;
use oteryn_protocol_oteryn::presentation as cooldown_wire;
use std::collections::BTreeMap;

type CooldownDeadlines = BTreeMap<(cooldown_wire::ActorCooldownKind, u32), u64>;

pub(super) struct CooldownProjection {
    revision: u64,
    deadlines: CooldownDeadlines,
}

fn deadlines(message: &cooldown_wire::ActorCooldowns) -> CooldownDeadlines {
    message
        .entries
        .iter()
        .map(|entry| ((entry.kind, entry.id), entry.expires_at_ms))
        .collect()
}

/// Presentation observation is independently negotiated from casting. Preserve
/// any existing producer's rows and cooldown data; fill only absent domains.
pub(super) fn observation_snapshots(
    selected: &crate::gameplay_transport::capabilities::SelectedCapabilities,
    presentation_revision: u64,
    cooldowns: Option<cooldown_wire::ActorCooldowns>,
    domains: &mut Vec<crate::gameplay_transport::item_view::ItemViewSnapshotDomain>,
) -> Result<Option<CooldownProjection>, ServeEnd> {
    use crate::gameplay_transport::item_view::ItemViewSnapshotDomain as Domain;
    use oteryn_protocol_oteryn::presentation as wire;
    if !selected.contains(wire::CAPABILITY_PRESENTATION_V1) {
        return Ok(None);
    }
    if !domains
        .iter()
        .any(|row| row.domain_id == wire::STATE_DOMAIN_WORLD_PRESENTATION)
    {
        domains.push(Domain {
            domain_id: wire::STATE_DOMAIN_WORLD_PRESENTATION,
            revision: presentation_revision,
            snapshot_type: wire::SNAPSHOT_TYPE_WORLD_PRESENTATION_V1,
            payload: wire::encode_presentation_snapshot(),
        });
    }
    if let Some(row) = domains
        .iter()
        .find(|row| row.domain_id == wire::STATE_DOMAIN_ACTOR_COOLDOWNS)
    {
        let cooldowns =
            wire::decode_actor_cooldowns(&row.payload).map_err(|_| ServeEnd::Disconnected)?;
        return Ok(Some(CooldownProjection {
            revision: row.revision,
            deadlines: deadlines(&cooldowns),
        }));
    }
    let cooldowns = cooldowns.unwrap_or(wire::ActorCooldowns {
        server_now_ms: 0,
        rtt_ms: None,
        entries: Vec::new(),
    });
    domains.push(Domain {
        domain_id: wire::STATE_DOMAIN_ACTOR_COOLDOWNS,
        revision: 0,
        snapshot_type: wire::SNAPSHOT_TYPE_ACTOR_COOLDOWNS_V1,
        payload: wire::encode_actor_cooldowns(&cooldowns).map_err(|_| ServeEnd::Disconnected)?,
    });
    Ok(Some(CooldownProjection {
        revision: 0,
        deadlines: deadlines(&cooldowns),
    }))
}

pub(super) async fn snapshot_observations<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &AdmittedSession,
    domains: &mut Vec<crate::gameplay_transport::item_view::ItemViewSnapshotDomain>,
) -> Result<Option<CooldownProjection>, ServeEnd> {
    let selected = admitted.continuity.selected_capabilities;
    let cooldowns = if selected.contains(cooldown_wire::CAPABILITY_PRESENTATION_V1) {
        authority
            .observe_actor_cooldowns(actor, admitted.game_session_id)
            .await
    } else {
        None
    };
    let projection = observation_snapshots(
        &selected,
        admitted.continuity.presentation_revision,
        cooldowns,
        domains,
    )?;
    if admitted
        .continuity
        .selected_capabilities
        .contains(oteryn_protocol_oteryn::native_map_cues::CAPABILITY_NATIVE_MAP_CUES_V1)
    {
        let native = authority
            .native_current_actor_map_cue_frame(actor, admitted.game_session_id)
            .await
            .ok_or(ServeEnd::Disconnected)?;
        let row = domains
            .iter_mut()
            .find(|row| {
                row.domain_id
                    == oteryn_protocol_oteryn::presentation::STATE_DOMAIN_WORLD_PRESENTATION
            })
            .ok_or(ServeEnd::Disconnected)?;
        if !row.payload.is_empty() || !matches!(row.snapshot_type, 1 | 2) {
            return Err(ServeEnd::Disconnected);
        }
        row.snapshot_type = if native {
            oteryn_protocol_oteryn::native_map_cues::SNAPSHOT_TYPE_NATIVE_MAP_CUE_BASIS_V1
        } else {
            1
        };
    }
    Ok(projection)
}

/// Publish changes to the negotiated own-actor domain after a successful cast.
pub(super) async fn refresh_cooldowns<S: AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
    committed_clearance: Option<&[cooldown_wire::ActorCooldown]>,
) -> Result<(), ServeEnd> {
    use cooldown_wire as wire;
    let Some(previous) = session.cooldowns.as_mut() else {
        return Ok(());
    };
    let Some(mut current) = authority
        .observe_actor_cooldowns(session.actor, session.admitted.game_session_id)
        .await
    else {
        return Ok(());
    };
    let next = deadlines(&current);
    current.entries.retain(|entry| {
        previous.deadlines.get(&(entry.kind, entry.id)) != Some(&entry.expires_at_ms)
    });
    if let Some(clearance) = committed_clearance {
        // This command captured its reset under the spell owner. Publication latency
        // cannot turn a committed reset into natural expiry. Preserve any newer deadline.
        current.entries.extend(
            clearance
                .iter()
                .copied()
                .filter(|entry| !next.contains_key(&(entry.kind, entry.id))),
        );
    } else {
        // Natural expiry is already known to the client. An early reset needs a tombstone.
        current.entries.extend(
            previous
                .deadlines
                .iter()
                .filter_map(|(&(kind, id), &expiry)| {
                    (expiry > current.server_now_ms && !next.contains_key(&(kind, id))).then_some(
                        wire::ActorCooldown {
                            kind,
                            id,
                            expires_at_ms: 0,
                        },
                    )
                }),
        );
    }
    if current.entries.is_empty() {
        previous.deadlines = next;
        return Ok(());
    }
    let revision = previous
        .revision
        .checked_add(1)
        .ok_or(ServeEnd::Disconnected)?;
    let sequence = session
        .sequence
        .checked_add(1)
        .ok_or(ServeEnd::Disconnected)?;
    let payload = wire::encode_actor_cooldowns(&current).map_err(|_| ServeEnd::Disconnected)?;
    let frame = encode_state_delta(
        session.generation,
        sequence,
        wire::STATE_DOMAIN_ACTOR_COOLDOWNS,
        previous.revision,
        revision,
        wire::DELTA_TYPE_ACTOR_COOLDOWNS_V1,
        &payload,
    )
    .map_err(|_| ServeEnd::Disconnected)?;
    write_frame(stream, &frame)
        .await
        .map_err(|_| ServeEnd::Disconnected)?;
    session.sequence = sequence;
    session.admitted.continuity.server_sequence = sequence;
    *previous = CooldownProjection {
        revision,
        deadlines: next,
    };
    Ok(())
}

/// House commands retain their own live loop, but consume the same scoped
/// movement-effect owner and negotiated presentation frame as Channel commands.
pub(super) async fn refresh_house_effects<S: AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
    stream: &mut S,
    authority: &A,
    actor: ExactActorRef,
    admitted: &mut AdmittedSession,
    cursor: &mut u64,
) -> Result<(), ServeEnd> {
    use oteryn_protocol_oteryn::presentation as wire;
    let selected = admitted.continuity.selected_capabilities;
    if !selected.contains(wire::CAPABILITY_PRESENTATION_V1) {
        return Ok(());
    }
    let effects = authority
        .observe_current_actor_movement_effects(actor, admitted.game_session_id, *cursor)
        .await;
    let Some(next) = effects.last().map(|entry| entry.0) else {
        return Ok(());
    };
    let Some(native) = authority
        .native_current_actor_map_cue_frame(actor, admitted.game_session_id)
        .await
    else {
        return Ok(());
    };
    let Some(channel) = authority.observe_visible_entities(actor).await else {
        return Ok(());
    };
    let Some(observer) = observer(&channel, actor) else {
        return Ok(());
    };
    let native = native
        && selected
            .contains(oteryn_protocol_oteryn::native_map_cues::CAPABILITY_NATIVE_MAP_CUES_V1);
    let events: Vec<_> = effects
        .into_iter()
        .filter(|(_, position, _)| super::outbound::movement_effect_visible(observer, *position))
        .enumerate()
        .filter_map(|(index, (_, position, effect))| {
            super::outbound::movement_magic_effect_event(
                u32::try_from(index + 1).ok()?,
                position,
                effect,
            )
        })
        .collect();
    if !events.is_empty() {
        if selected.contains(290) {
            authority.cast_effects(*admitted, &events).await;
        }
        let payload = if native {
            oteryn_protocol_oteryn::native_map_cues::encode_native_map_presentation_batch(&events)
        } else {
            wire::encode_presentation_batch(&events)
        }
        .map_err(|_| ServeEnd::Disconnected)?;
        let revision = admitted
            .continuity
            .presentation_revision
            .checked_add(1)
            .ok_or(ServeEnd::Disconnected)?;
        let sequence = admitted
            .continuity
            .server_sequence
            .checked_add(1)
            .ok_or(ServeEnd::Disconnected)?;
        let frame = encode_state_delta(
            admitted.continuity.connection_generation,
            sequence,
            wire::STATE_DOMAIN_WORLD_PRESENTATION,
            admitted.continuity.presentation_revision,
            revision,
            if native {
                oteryn_protocol_oteryn::native_map_cues::DELTA_TYPE_NATIVE_MAP_PRESENTATION_V1
            } else {
                wire::DELTA_TYPE_WORLD_PRESENTATION_BATCH_V1
            },
            &payload,
        )
        .map_err(|_| ServeEnd::Disconnected)?;
        write_frame(stream, &frame)
            .await
            .map_err(|_| ServeEnd::Disconnected)?;
        admitted.continuity.server_sequence = sequence;
        admitted.continuity.presentation_revision = revision;
    }
    *cursor = next;
    Ok(())
}

fn observer(
    channel: &crate::gameplay_transport::world_spatial::ChannelEntities,
    actor: ExactActorRef,
) -> Option<oteryn_protocol_oteryn::world_spatial::ActorPosition> {
    channel
        .entities
        .iter()
        .find(|entry| {
            entry.identity == channel.observer
                && entry.identity == actor.placement_identity()
                && entry.generation == actor.actor_local_generation()
        })
        .map(|entry| entry.position)
}
// Map callback positions use the bundle tile index's signed floor.
fn cue_floor(floor: i8) -> Option<i16> {
    (-15..=0).contains(&floor).then_some(-i16::from(floor))
}
fn speech_line(
    speech: &crate::map::facts::MovementSpeech,
    channel: &crate::gameplay_transport::world_spatial::ChannelEntities,
    observer: oteryn_protocol_oteryn::world_spatial::ActorPosition,
    monster_mode: bool,
) -> Option<ChatLine> {
    use crate::chat::{ChatPosition, Heard, SpeechMode};
    use oteryn_protocol_oteryn::chat::{ChatSpeaker, ChatSpeechMode};
    let floor = |floor: i16| u8::try_from(crate::movement::interest::tibia_floor(floor)?).ok();
    let mode = match speech.talk_type {
        36 => SpeechMode::Say,
        37 => SpeechMode::Yell,
        _ => return None,
    };
    if crate::chat::hears(
        mode,
        ChatPosition {
            x: i32::from(speech.position.x),
            y: i32::from(speech.position.y),
            floor: floor(i16::from(speech.position.floor))?,
        },
        ChatPosition {
            x: observer.x,
            y: observer.y,
            floor: floor(observer.floor)?,
        },
    ) != Some(Heard::Text)
    {
        return None;
    }
    let name = speech.captured_name.or_else(|| {
        channel
            .entities
            .iter()
            .find(|entry| {
                entry.identity == speech.speaker.placement_identity()
                    && entry.generation == speech.speaker.actor_local_generation()
            })
            .and_then(|entry| entry.display_name)
    })?;
    Some(ChatLine::Local {
        speaker: ChatSpeaker {
            identity: speech.speaker.placement_identity(),
            generation: std::num::NonZeroU64::new(speech.speaker.actor_local_generation())?,
        },
        speaker_name: name.as_str().ok()?.to_owned(),
        text: speech.text.to_owned(),
        mode: match speech.talk_type {
            37 => ChatSpeechMode::Yell,
            _ if monster_mode => ChatSpeechMode::MonsterSay,
            _ => ChatSpeechMode::Say,
        },
        position: oteryn_protocol_oteryn::world_spatial::ActorPosition {
            x: i32::from(speech.position.x),
            y: i32::from(speech.position.y),
            floor: cue_floor(speech.position.floor)?,
        },
    })
}

pub(super) async fn refresh<S: AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
) -> Result<(), ServeEnd> {
    use oteryn_protocol_oteryn::presentation as wire;
    let selected = session
        .selected_capabilities
        .contains(wire::CAPABILITY_PRESENTATION_V1);
    let chat = session.chat_refresh.is_some();
    if session.native_cue_actor != session.actor {
        session.native_cue_actor = session.actor;
        session.movement_effect_revision = authority
            .observe_current_actor_movement_effects(
                session.actor,
                session.admitted.game_session_id,
                0,
            )
            .await
            .last()
            .map_or(0, |entry| entry.0);
        session.movement_speech_revision = authority
            .observe_movement_speech(0)
            .last()
            .map_or(0, |entry| entry.0);
        session.creature_visual_revision = authority
            .observe_creature_visuals(0)
            .last()
            .map_or(0, |entry| entry.0);
        return Ok(());
    }
    let visuals = if selected {
        authority.observe_creature_visuals(session.creature_visual_revision)
    } else {
        Vec::new()
    };
    let effects = if selected {
        authority
            .observe_current_actor_movement_effects(
                session.actor,
                session.admitted.game_session_id,
                session.movement_effect_revision,
            )
            .await
    } else {
        Vec::new()
    };
    let speech = if chat {
        authority.observe_movement_speech(session.movement_speech_revision)
    } else {
        Vec::new()
    };
    if effects.is_empty() && visuals.is_empty() && speech.is_empty() {
        return Ok(());
    }
    let Some(native_floor) = authority
        .native_current_actor_map_cue_frame(session.actor, session.admitted.game_session_id)
        .await
    else {
        return Ok(());
    };
    let native_wire = native_floor
        && session
            .selected_capabilities
            .contains(oteryn_protocol_oteryn::native_map_cues::CAPABILITY_NATIVE_MAP_CUES_V1);
    let named = speech
        .iter()
        .any(|(_, speech)| speech.captured_name.is_none());
    let channel = if named {
        authority
            .observe_named_visible_entities(session.actor)
            .await
    } else {
        authority.observe_visible_entities(session.actor).await
    };
    let Some(channel) = channel else {
        return Ok(());
    };
    let Some(observer) = observer(&channel, session.actor) else {
        return Ok(());
    };

    let effect_cursor = effects.last().map(|entry| entry.0);
    let visual_cursor = visuals.last().map(|entry| entry.0);
    {
        let mut pending: Vec<_> = effects
            .into_iter()
            .map(|(seq, position, effect)| {
                (
                    seq,
                    crate::map::facts::CreatureVisualKind::Effect { position, effect },
                    false,
                )
            })
            .collect();
        pending.extend(visuals.into_iter().map(|(seq, kind)| (seq, kind, true)));
        pending.sort_by_key(|entry| entry.0);
        let events: Vec<_> = pending
            .into_iter()
            .filter_map(|(_, kind, creature)| {
                use crate::map::facts::CreatureVisualKind;
                let source = if creature {
                    wire::PresentationSource::Creatures
                } else {
                    wire::PresentationSource::Own
                };
                match kind {
                    CreatureVisualKind::Effect { position, effect }
                        if super::outbound::movement_effect_visible(observer, position) =>
                    {
                        let mut event =
                            super::outbound::movement_magic_effect_event(1, position, effect)?;
                        if let wire::PresentationEventKind::MagicEffect {
                            source: event_source,
                            ..
                        } = &mut event.kind
                        {
                            *event_source = source;
                        }
                        Some(event)
                    }
                    CreatureVisualKind::Missile { from, to, missile }
                        if super::outbound::movement_effect_visible(observer, from)
                            || super::outbound::movement_effect_visible(observer, to) =>
                    {
                        let position = |p: crate::map::overlay::TilePos| {
                            Some(oteryn_protocol_oteryn::world_spatial::ActorPosition {
                                x: i32::from(p.x),
                                y: i32::from(p.y),
                                floor: cue_floor(p.floor)?,
                            })
                        };
                        Some(wire::PresentationEvent {
                            decision_ordinal: 1,
                            emission_ordinal: 1,
                            kind: wire::PresentationEventKind::Missile {
                                from: position(from)?,
                                to: position(to)?,
                                missile_id: u32::from(missile),
                                source,
                            },
                        })
                    }
                    _ => None,
                }
            })
            .enumerate()
            .map(|(index, mut event)| {
                event.emission_ordinal = (index + 1) as u32;
                event
            })
            .collect();
        if !events.is_empty() {
            if session.selected_capabilities.contains(290) {
                authority.cast_effects(session.admitted, &events).await;
            }
            let payload = if native_wire {
                oteryn_protocol_oteryn::native_map_cues::encode_native_map_presentation_batch(
                    &events,
                )
            } else {
                wire::encode_presentation_batch(&events)
            }
            .map_err(|_| ServeEnd::Disconnected)?;
            let revision = session
                .admitted
                .continuity
                .presentation_revision
                .checked_add(1)
                .ok_or(ServeEnd::Disconnected)?;
            let sequence = session
                .sequence
                .checked_add(1)
                .ok_or(ServeEnd::Disconnected)?;
            let frame = encode_state_delta(
                session.generation,
                sequence,
                wire::STATE_DOMAIN_WORLD_PRESENTATION,
                session.admitted.continuity.presentation_revision,
                revision,
                if native_wire {
                    oteryn_protocol_oteryn::native_map_cues::DELTA_TYPE_NATIVE_MAP_PRESENTATION_V1
                } else {
                    wire::DELTA_TYPE_WORLD_PRESENTATION_BATCH_V1
                },
                &payload,
            )
            .map_err(|_| ServeEnd::Disconnected)?;
            write_frame(stream, &frame)
                .await
                .map_err(|_| ServeEnd::Disconnected)?;
            session.sequence = sequence;
            session.admitted.continuity.server_sequence = sequence;
            session.admitted.continuity.presentation_revision = revision;
        }
        if let Some(cursor) = effect_cursor {
            session.movement_effect_revision = cursor;
        }
        if let Some(cursor) = visual_cursor {
            session.creature_visual_revision = cursor;
        }
    }
    for (cursor, speech) in speech.into_iter().take(EGRESS_MAX_LINES) {
        let monster_mode = session
            .selected_capabilities
            .contains(wire::CAPABILITY_AMBIENT_SOUND_V1);
        if let Some(line) = speech_line(&speech, &channel, observer, monster_mode) {
            let encoded = if native_wire {
                oteryn_protocol_oteryn::native_map_cues::encode_native_map_chat_line(&line)
                    .ok()
                    .and_then(|payload| {
                        chat_delta(session.generation, session.sequence,
                        session.admitted.continuity.chat_revision,
                        oteryn_protocol_oteryn::native_map_cues::DELTA_TYPE_NATIVE_MAP_CHAT_LINE_V1,
                        &payload)
                    })
            } else {
                chat_line_delta(
                    session.generation,
                    session.sequence,
                    session.admitted.continuity.chat_revision,
                    &line,
                )
            };
            let Some((sequence, frame, revision)) = encoded else {
                return Err(ServeEnd::Disconnected);
            };
            write_frame(stream, &frame)
                .await
                .map_err(|_| ServeEnd::Disconnected)?;
            session.sequence = sequence;
            session.admitted.continuity.server_sequence = sequence;
            session.admitted.continuity.chat_revision = revision;
        }
        session.movement_speech_revision = cursor;
    }
    Ok(())
}

#[cfg(test)]
mod observation_tests {
    use super::*;
    use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
    use crate::gameplay_transport::item_view::ItemViewSnapshotDomain as Domain;
    use oteryn_protocol_oteryn::presentation as wire;

    #[test]
    fn native_observation_pair_preserves_existing_clocked_rows_and_revision() -> Result<(), String>
    {
        let selected = SelectedCapabilities::select(
            &[
                OfferedCapability {
                    id: 6,
                    requires: &[],
                },
                OfferedCapability {
                    id: 23,
                    requires: &[6],
                },
            ],
            &[6, 23],
        )
        .ok_or("negotiated observation")?;
        let mut domains = Vec::new();
        assert!(observation_snapshots(&SelectedCapabilities::NONE, 7, None, &mut domains).is_ok());
        assert!(domains.is_empty());
        assert!(observation_snapshots(&selected, 7, None, &mut domains).is_ok());
        assert_eq!(domains.len(), 2);
        let presentation = domains
            .iter()
            .find(|row| row.domain_id == 19)
            .ok_or("presentation domain")?;
        assert_eq!(presentation.revision, 7);
        assert!(presentation.payload.is_empty());
        let cooldowns = domains
            .iter()
            .find(|row| row.domain_id == 20)
            .ok_or("cooldown domain")?;
        assert!(
            wire::decode_actor_cooldowns(&cooldowns.payload)
                .map_err(|error| format!("cooldown codec: {error:?}"))?
                .entries
                .is_empty()
        );
        let existing = vec![
            Domain {
                domain_id: 19,
                revision: 11,
                snapshot_type: 1,
                payload: vec![],
            },
            Domain {
                domain_id: 20,
                revision: 13,
                snapshot_type: 1,
                payload: wire::encode_actor_cooldowns(&wire::ActorCooldowns {
                    server_now_ms: 42,
                    rtt_ms: Some(3),
                    entries: vec![],
                })
                .map_err(|error| format!("cooldown codec: {error:?}"))?,
            },
        ];
        let mut preserved = existing.clone();
        assert!(observation_snapshots(&selected, 99, None, &mut preserved).is_ok());
        assert_eq!(
            preserved, existing,
            "casting owner rows remain byte-for-byte intact"
        );
        domains.retain(|row| row.domain_id == 20);
        let retained = domains[0].clone();
        assert!(observation_snapshots(&selected, 17, None, &mut domains).is_ok());
        assert_eq!(
            domains.iter().find(|row| row.domain_id == 20),
            Some(&retained)
        );
        assert_eq!(
            domains
                .iter()
                .find(|row| row.domain_id == 19)
                .ok_or("restored presentation domain")?
                .revision,
            17
        );
        Ok(())
    }
}
