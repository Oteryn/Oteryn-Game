use crate::play::{PlayView, play_channel};
use oteryn_renderer::{TileCoord, lighting::LIT_INSTANCE_BYTES};
use oteryn_session::{
    ActorPosition, AppliedDelta, PresentationEvent, PresentationEventKind, PresentationSource,
    SessionEvent,
};

#[test]
fn pinned_boss_area_category_reaches_wire_caller_and_gpu_alpha()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir().join(format!("oteryn-boss-opacity-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("manifest.json");
    let bytes = br#"{"files":[],"boss_area_effects":[41]}"#;
    std::fs::write(&path, bytes)?;
    let assets = oteryn_client_assets::AssetStore::open_pinned(
        &dir,
        &path,
        &oteryn_client_assets::text_sha256(bytes),
    )?;
    let world = crate::map::World::builtin()?.with_effect_catalog(assets.effects().clone());
    let mut view = PlayView::new(TileCoord::new(0, 0), 7, &[])?;
    view.local.set_world(std::sync::Arc::new(world));
    let mut preferences = view.preferences().clone();
    preferences.boss_area_opacity = 10;
    preferences.effect_opacity[2] = 40;
    view.set_preferences(preferences.clone())?;
    let events: Vec<_> = [41, 42]
        .into_iter()
        .enumerate()
        .map(|(i, id)| PresentationEvent {
            decision_ordinal: i as u32 + 1,
            emission_ordinal: 1,
            kind: PresentationEventKind::MagicEffect {
                position: ActorPosition {
                    x: 4,
                    y: 4,
                    floor: 7,
                },
                effect_id: id,
                source: PresentationSource::Creatures,
            },
        })
        .collect();
    let encoded = oteryn_protocol_oteryn::presentation::encode_presentation_batch(&events)
        .map_err(|_| "encode effects")?;
    let decoded = oteryn_protocol_oteryn::presentation::decode_presentation_batch(&encoded)
        .map_err(|_| "decode effects")?;
    let (link, _commands, outbox) = play_channel();
    outbox.push(vec![SessionEvent::Presentation(AppliedDelta {
        server_sequence: 1,
        base_revision: 0,
        new_revision: 1,
        value: decoded,
    })]);
    assert!(view.tick(&link).is_ok());
    let alpha = |view: &PlayView| {
        let sprites = view.scene().sprites();
        sprites
            .lighting
            .instance_bytes(sprites.instances())
            .as_chunks::<LIT_INSTANCE_BYTES>()
            .0
            .iter()
            .map(|chunk| {
                f32::from_ne_bytes(
                    chunk[LIT_INSTANCE_BYTES - 4..]
                        .try_into()
                        .unwrap_or_default(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert!(alpha(&view).contains(&0.1));
    assert!(alpha(&view).contains(&0.4));
    preferences.boss_area_opacity = 0;
    view.set_preferences(preferences.clone())?;
    assert!(!alpha(&view).contains(&0.1));
    assert!(alpha(&view).contains(&0.4));
    preferences.boss_area_opacity = 101;
    assert!(view.set_preferences(preferences).is_err());
    std::fs::remove_dir_all(dir)?;
    Ok(())
}
