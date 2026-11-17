use oteryn_client::play::{PlayView, play_channel};
use oteryn_renderer::TileCoord;
use oteryn_session::{AppliedDelta, PresentationEvent, PresentationEventKind, SessionEvent};

#[test]
fn ambient_replaces_and_stops_independently_of_one_shots() -> Result<(), Box<dyn std::error::Error>>
{
    let (link, _commands, outbox) = play_channel();
    let mut view = PlayView::new(TileCoord::new(0, 0), 0, &[])?;
    for cue_id in [25, 28, 0] {
        outbox.push(vec![SessionEvent::Presentation(AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value: vec![PresentationEvent {
                decision_ordinal: 1,
                emission_ordinal: 1,
                kind: PresentationEventKind::AmbientSound { cue_id },
            }],
        })]);
        assert!(view.tick(&link).is_ok());
        let cues = view.take_sound_cues();
        assert_eq!(cues.len(), 1);
        assert!(cues[0].ambient);
        assert_eq!(cues[0].id, cue_id);
    }
    Ok(())
}

#[test]
fn pending_ambient_stop_survives_mailbox_saturation() -> Result<(), Box<dyn std::error::Error>> {
    let (link, _commands, outbox) = play_channel();
    let mut view = PlayView::new(TileCoord::new(0, 0), 0, &[])?;
    for cue_id in 1..=200 {
        outbox.push(vec![SessionEvent::Presentation(AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value: vec![
                PresentationEvent {
                    decision_ordinal: 1,
                    emission_ordinal: 1,
                    kind: PresentationEventKind::Sound {
                        position: oteryn_session::ActorPosition {
                            x: 0,
                            y: 0,
                            floor: 0,
                        },
                        cue_id,
                        secondary_cue_id: 0,
                        source: oteryn_session::PresentationSource::Own,
                    },
                },
                PresentationEvent {
                    decision_ordinal: 2,
                    emission_ordinal: 1,
                    kind: PresentationEventKind::AmbientSound {
                        cue_id: if cue_id == 200 { 0 } else { 25 },
                    },
                },
            ],
        })]);
    }
    assert!(view.tick(&link).is_ok());
    let cues = view.take_sound_cues();
    assert_eq!(cues.len(), 128);
    assert_eq!(cues.iter().filter(|cue| cue.ambient).count(), 1);
    assert!(cues.iter().any(|cue| cue.ambient && cue.id == 0));
    Ok(())
}
