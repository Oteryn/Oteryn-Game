use oteryn_client::play::{PlayView, play_channel};
use oteryn_renderer::TileCoord;
use oteryn_session::{
    ActorPosition, AppliedDelta, PresentationEvent, PresentationEventKind, PresentationSource,
    SessionEvent,
};

#[test]
fn effects_sliders_apply_every_percent_through_play_queue() -> Result<(), Box<dyn std::error::Error>>
{
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| run_effects_sliders().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "effects slider test thread panicked")??;
    Ok(())
}

fn run_effects_sliders() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client::play::{ReferencePreferences, SettingsState};
    for index in 0..3 {
        for percent in 0..=100 {
            let (link, _commands, outbox) = play_channel();
            let mut settings = SettingsState::new(None);
            let mut preferences = ReferencePreferences::default();
            preferences.set_effect_opacity_slider(index, percent)?;
            assert_eq!(preferences.effect_opacity[index], percent as u8);
            for other in 0..3 {
                if other != index {
                    assert_eq!(preferences.effect_opacity[other], 100);
                }
            }
            settings.apply(preferences);
            settings.sync_local_preferences(&link);
            let mut view = PlayView::new(TileCoord::new(0, 0), 7, &[])?;
            settings.restore(&mut view, [900, 600])?;
            let baseline = view.scene().sprites().len();
            outbox.push(vec![SessionEvent::Presentation(AppliedDelta {
                server_sequence: 1,
                base_revision: 0,
                new_revision: 1,
                value: vec![PresentationEvent {
                    decision_ordinal: 1,
                    emission_ordinal: 1,
                    kind: PresentationEventKind::MagicEffect {
                        position: ActorPosition {
                            x: 4,
                            y: 4,
                            floor: 7,
                        },
                        effect_id: 1,
                        source: [
                            PresentationSource::Own,
                            PresentationSource::Others,
                            PresentationSource::Creatures,
                        ][index],
                    },
                }],
            })]);
            assert!(view.tick(&link).is_ok());
            assert_eq!(
                view.preferences().effect_opacity,
                settings.preferences.effect_opacity
            );
            let sprites = view.scene().sprites();
            let bytes = sprites.lighting.instance_bytes(sprites.instances());
            let alpha = f32::from(percent as u8) / 100.0;
            if percent == 0 {
                assert_eq!(sprites.len(), baseline);
            } else {
                assert!(sprites.len() > baseline);
                assert!(
                    bytes
                        .as_chunks::<{ oteryn_renderer::lighting::LIT_INSTANCE_BYTES }>()
                        .0
                        .iter()
                        .any(|chunk| chunk[chunk.len() - 4..] == alpha.to_ne_bytes())
                );
            }
        }
    }
    let mut preferences = ReferencePreferences::default();
    let before = preferences.clone();
    for (index, percent) in [(0, -1), (1, 101), (3, 50)] {
        assert!(
            preferences
                .set_effect_opacity_slider(index, percent)
                .is_err()
        );
        assert_eq!(preferences, before);
    }
    Ok(())
}
#[test]
fn source_opacity_reaches_confirmed_effect_gpu_upload() -> Result<(), Box<dyn std::error::Error>> {
    for (source, index) in [
        (PresentationSource::Own, 0),
        (PresentationSource::Others, 1),
        (PresentationSource::Creatures, 2),
    ] {
        let (link, _commands, outbox) = play_channel();
        let mut view = PlayView::new(TileCoord::new(0, 0), 7, &[])?;
        let mut preferences = view.preferences().clone();
        preferences.effect_opacity[index] = 25;
        view.set_preferences(preferences)?;
        outbox.push(vec![SessionEvent::Presentation(AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value: vec![PresentationEvent {
                decision_ordinal: 1,
                emission_ordinal: 1,
                kind: PresentationEventKind::MagicEffect {
                    position: ActorPosition {
                        x: 4,
                        y: 4,
                        floor: 7,
                    },
                    effect_id: 1,
                    source,
                },
            }],
        })]);
        assert!(view.tick(&link).is_ok());
        let sprites = view.scene().sprites();
        let terrain = sprites.lighting.opaque_instance_bytes(sprites.instances());
        assert!(
            terrain
                .as_chunks::<{ oteryn_renderer::lighting::LIT_INSTANCE_BYTES }>()
                .0
                .iter()
                .all(|chunk| chunk[chunk.len() - 4..] == 1.0f32.to_ne_bytes())
        );
        let bytes = sprites.lighting.instance_bytes(sprites.instances());
        assert!(
            bytes
                .as_chunks::<{ oteryn_renderer::lighting::LIT_INSTANCE_BYTES }>()
                .0
                .iter()
                .any(|chunk| chunk[chunk.len() - 4..] == 0.25f32.to_ne_bytes())
        );
    }
    Ok(())
}
