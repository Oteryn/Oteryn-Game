//! Headless C28 scenario through the real native client's feature reducer and input.
use oteryn_client::play::{PlayView, play_channel};
use oteryn_input_actions::{ButtonState, KeyCode, Modifiers, NormalizedInputEvent};
use oteryn_renderer::TileCoord;
use oteryn_session::{
    ActorPosition, AppliedDelta, PresentationEvent, PresentationEventKind, PresentationSource,
    SessionEvent,
};

#[test]
fn audio_sound_batch_reaches_native_projection_and_mute_retains_master_volume()
-> Result<(), Box<dyn std::error::Error>> {
    let event = PresentationEvent {
        decision_ordinal: 1,
        emission_ordinal: 1,
        kind: PresentationEventKind::Sound {
            position: ActorPosition {
                x: 0,
                y: 0,
                floor: 0,
            },
            cue_id: 41,
            secondary_cue_id: 0,
            source: PresentationSource::Creatures,
        },
    };
    let (link, _commands, outbox) = play_channel();
    outbox.push(vec![SessionEvent::Presentation(AppliedDelta {
        server_sequence: 41,
        base_revision: 0,
        new_revision: 1,
        value: vec![event],
    })]);
    let mut view = PlayView::new(TileCoord::new(0, 0), 0, &[])?;
    assert!(view.tick(&link).is_ok());
    assert_eq!(
        view.take_sound_cues()[0].source,
        PresentationSource::Creatures
    );
    let key = |code| -> Result<NormalizedInputEvent, oteryn_input_actions::InputError> {
        Ok(NormalizedInputEvent::Key {
            code: KeyCode::new(code)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::NONE,
            repeat: false,
        })
    };
    assert!(view.audio_input(&[key(68)?]));
    assert_eq!(view.preferences().audio.master_volume, 95);
    assert!(view.audio_input(&[key(67)?]));
    assert_eq!(view.preferences().audio.gain(), 0.0);
    assert!(view.audio_input(&[key(67)?]));
    assert_eq!(view.preferences().audio.gain(), 0.95);
    assert!(view.take_sound_cues().is_empty());
    Ok(())
}
