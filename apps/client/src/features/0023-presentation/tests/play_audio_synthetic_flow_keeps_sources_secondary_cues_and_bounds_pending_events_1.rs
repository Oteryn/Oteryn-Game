#[test]
fn audio_synthetic_flow_keeps_sources_secondary_cues_and_bounds_pending_events()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, _commands, outbox) = crate::play::play_channel();
    let events = [
        PresentationSource::Own,
        PresentationSource::Others,
        PresentationSource::Creatures,
        PresentationSource::Global,
    ]
    .into_iter()
    .map(|source| PresentationEvent {
        decision_ordinal: 1,
        emission_ordinal: 1,
        kind: PresentationEventKind::Sound {
            position: oteryn_session::ActorPosition {
                x: 0,
                y: 0,
                floor: 0,
            },
            cue_id: 41,
            source,
            secondary_cue_id: 42,
        },
    })
    .collect();
    outbox.push(vec![oteryn_session::SessionEvent::Presentation(
        AppliedDelta {
            server_sequence: 41,
            base_revision: 0,
            new_revision: 1,
            value: events,
        },
    )]);
    let mut view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(0, 0), 0, &[])?;
    assert!(view.tick(&link).is_ok());
    let cues = view.take_sound_cues();
    assert_eq!(cues.len(), 8);
    assert_eq!(
        cues[0],
        SoundCue {
            id: 41,
            source: PresentationSource::Own,
            ambient: false,
        }
    );
    assert_eq!(
        cues[7],
        SoundCue {
            id: 42,
            source: PresentationSource::Global,
            ambient: false,
        }
    );
    assert!(view.take_sound_cues().is_empty());
    for _ in 0..200 {
        outbox.push(vec![oteryn_session::SessionEvent::Presentation(
            AppliedDelta {
                server_sequence: 42,
                base_revision: 1,
                new_revision: 2,
                value: vec![PresentationEvent {
                    decision_ordinal: 1,
                    emission_ordinal: 1,
                    kind: PresentationEventKind::Sound {
                        position: oteryn_session::ActorPosition {
                            x: 0,
                            y: 0,
                            floor: 0,
                        },
                        cue_id: 41,
                        source: PresentationSource::Own,
                        secondary_cue_id: 0,
                    },
                }],
            },
        )]);
    }
    assert!(view.tick(&link).is_ok());
    assert_eq!(view.take_sound_cues().len(), MAX_PENDING_SOUNDS);
    Ok(())
}
