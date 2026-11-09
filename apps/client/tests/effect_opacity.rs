use oteryn_client::play::{PlayView, play_channel};
use oteryn_renderer::TileCoord;
use oteryn_session::{
    ActorPosition, AppliedDelta, PresentationEvent, PresentationEventKind, PresentationSource,
    SessionEvent,
};
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
