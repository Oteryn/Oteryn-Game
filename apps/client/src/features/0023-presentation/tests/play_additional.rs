use super::*;

#[test]
fn combat_review_overflow_keeps_newest_in_mailbox_and_view()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, _commands, outbox) = crate::play::play_channel();
    for sequence in 1..=2 {
        outbox.push(vec![oteryn_session::SessionEvent::Presentation(
            AppliedDelta {
                server_sequence: sequence,
                base_revision: sequence - 1,
                new_revision: sequence,
                value: (1..=MAX_VISUAL_CUES + 1)
                    .map(|ordinal| PresentationEvent {
                        decision_ordinal: 1,
                        emission_ordinal: ordinal as u32,
                        kind: numbered_value(
                            (sequence - 1) * (MAX_VISUAL_CUES as u64 + 1) + ordinal as u64,
                        ),
                    })
                    .collect(),
            },
        )]);
    }
    let mut view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(0, 0), 0, &[])?;
    assert!(view.tick(&link).is_ok());
    assert_eq!(view.visuals.len(), MAX_VISUAL_CUES);
    assert_eq!(
        view.visuals[0].kind,
        numbered_value(MAX_VISUAL_CUES as u64 + 3)
    );
    assert_eq!(
        view.visuals[MAX_VISUAL_CUES - 1].kind,
        numbered_value(2 * (MAX_VISUAL_CUES as u64 + 1))
    );
    let now = std::time::Instant::now();
    view.receive_presentation(&[numbered_value(999)], now);
    assert_eq!(view.visuals.len(), MAX_VISUAL_CUES);
    assert_eq!(
        view.visuals[0].kind,
        numbered_value(MAX_VISUAL_CUES as u64 + 4)
    );
    assert_eq!(view.visuals[MAX_VISUAL_CUES - 1].kind, numbered_value(999));
    Ok(())
}

#[test]
fn combat_review_same_tile_values_reserve_separate_lines_in_order()
-> Result<(), Box<dyn std::error::Error>> {
    let mut view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(0, 0), 0, &[])?;
    let base = view.scene().sprites().len();
    let now = std::time::Instant::now();
    view.receive_presentation(&[numbered_value(8)], now);
    view.project_presentation(now)?;
    let first = view.scene().sprites().instances()[base..].to_vec();
    // The middle cue has a secondary number: it must reserve two lines.
    let mut middle = numbered_value(8);
    if let PresentationEventKind::ValueText {
        secondary, primary, ..
    } = &mut middle
    {
        *secondary = Some(*primary);
    }
    view.receive_presentation(&[middle, numbered_value(8)], now);
    view.project_presentation(now)?;
    let quads = &view.scene().sprites().instances()[base..];
    assert_eq!(quads.len(), first.len() * 4);
    for (line, group) in quads.chunks_exact(first.len()).enumerate() {
        for (a, b) in first.iter().zip(group) {
            assert_eq!(
                b.position,
                [a.position[0], a.position[1] - 12.0 * line as f32]
            );
        }
    }
    Ok(())
}

#[test]
fn novelty_server_effects_decode_into_distinct_native_bursts_and_expire()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_protocol_oteryn::presentation::{
        decode_item_effect_batch, encode_item_effect_batch,
    };
    let mut rendered = Vec::new();
    for effect_id in [29, 30, 31, 28, 27, 24] {
        let event = PresentationEvent {
            decision_ordinal: 1,
            emission_ordinal: 1,
            kind: PresentationEventKind::MagicEffect {
                position: oteryn_session::ActorPosition {
                    x: 0,
                    y: 0,
                    floor: 7,
                },
                effect_id,
                source: PresentationSource::Others,
            },
        };
        let bytes = encode_item_effect_batch(&[event]).map_err(|e| format!("{e:?}"))?;
        let decoded = decode_item_effect_batch(&bytes).map_err(|e| format!("{e:?}"))?;
        assert_eq!(decoded, vec![event]);
        let (link, _commands, outbox) = crate::play::play_channel();
        let mut view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(0, 0), -7, &[])?;
        let base = view.scene().sprites().len();
        outbox.push(vec![oteryn_session::SessionEvent::ItemEffects(
            AppliedDelta {
                server_sequence: 42,
                base_revision: 0,
                new_revision: 1,
                value: decoded,
            },
        )]);
        view.tick(&link).map_err(|e| format!("{e:?}"))?;
        // Fix the animation instant so comparisons prove the server effect ID is used.
        let now = std::time::Instant::now();
        for cue in &mut view.visuals {
            cue.received = now;
        }
        view.project_presentation(now)?;
        let quads = view.scene().sprites().instances()[base..].to_vec();
        assert!(!quads.is_empty());
        rendered.push(quads);
        view.advance_presentation(now + VISUAL_LIFETIME)?;
        assert_eq!(view.scene().sprites().len(), base);
    }
    for (i, one) in rendered.iter().enumerate() {
        for other in &rendered[i + 1..] {
            assert_ne!(
                one, other,
                "distinct donor effect IDs must produce distinct native visuals"
            );
        }
    }
    Ok(())
}

#[test]
fn legacy_presentation_ingress_normalizes_all_visual_kinds_once()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_protocol_oteryn::presentation::{
        PresentationColor, PresentationValue, PresentationValueKind,
    };
    use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
    use oteryn_session::ActorPosition;
    let entity = EntityRef {
        identity: [3; 16],
        generation: 1,
    };
    let kinds = |position: ActorPosition| {
        [
            PresentationEventKind::MagicEffect {
                position,
                effect_id: 1,
                source: PresentationSource::Own,
            },
            PresentationEventKind::Missile {
                from: position,
                to: ActorPosition {
                    x: position.x + 1,
                    ..position
                },
                missile_id: 1,
                source: PresentationSource::Creatures,
            },
            PresentationEventKind::SpellWords {
                caster: entity,
                position,
                spell: 1,
            },
            PresentationEventKind::ValueText {
                kind: PresentationValueKind::DamageDealt,
                position,
                target: entity,
                attacker: None,
                primary: PresentationValue {
                    value: 12,
                    color: PresentationColor::Red,
                },
                secondary: Some(PresentationValue {
                    value: 3,
                    color: PresentationColor::LightGrey,
                }),
            },
        ]
    };
    for floor in [0_i16, 6, 7, 8, 15] {
        let position = ActorPosition {
            x: 10,
            y: 20,
            floor,
        };
        let incoming = kinds(position);
        let expected = kinds(ActorPosition {
            floor: -floor,
            ..position
        });
        let mut view =
            crate::play::PlayView::new(oteryn_renderer::TileCoord::new(0, 0), -floor, &[])?;
        let now = std::time::Instant::now();
        view.receive_presentation(&incoming[..2], now);
        view.receive_presentation(&incoming[2..], now);
        assert_eq!(
            view.visuals.iter().map(|cue| cue.kind).collect::<Vec<_>>(),
            expected.to_vec()
        );
    }
    Ok(())
}
