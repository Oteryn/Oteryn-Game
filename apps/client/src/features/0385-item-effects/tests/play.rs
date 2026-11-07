#[test]
fn srv7_house_item_magic_effects_decode_and_reach_native_scene()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::{
        ActorPosition, AppliedDelta, PresentationEvent, PresentationEventKind, PresentationSource,
        SessionEvent,
    };
    let position = ActorPosition {
        x: 0,
        y: 0,
        floor: 0,
    };
    let events = [PresentationEvent {
        decision_ordinal: 1,
        emission_ordinal: 1,
        kind: PresentationEventKind::MagicEffect {
            position,
            effect_id: 175,
            source: PresentationSource::Own,
        },
    }];
    // The native adapter sees only bytes accepted by the negotiated item-only decoder.
    let bytes = oteryn_session::item_effects::encode_item_effect_batch(&events)
        .map_err(|e| format!("item effect encode: {e:?}"))?;
    let effects = oteryn_session::item_effects::decode_item_effect_batch(&bytes)
        .map_err(|e| format!("item effect decode: {e:?}"))?;
    let (link, _commands, outbox) = crate::play::play_channel();
    let mut view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(0, 0), 0, &[])?;
    let base = view.scene().sprites().len();
    outbox.push(vec![SessionEvent::ItemEffects(AppliedDelta {
        server_sequence: 1,
        base_revision: 0,
        new_revision: 1,
        value: effects,
    })]);
    assert!(view.tick(&link).is_ok());
    assert_eq!(view.visuals.len(), 1);
    assert_eq!(view.visuals[0].kind, events[0].kind);
    assert!(
        view.scene().sprites().len() > base,
        "item effect is rendered by the native scene"
    );
    assert!(
        view.take_sound_cues().is_empty(),
        "magic-only item effects activate no audio"
    );
    view.advance_presentation(std::time::Instant::now() + std::time::Duration::from_secs(2))?;
    assert_eq!(view.scene().sprites().len(), base);
    Ok(())
}
#[test]
fn srv7_committed_house_fish_and_harp_captures_reach_native_scene()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            use oteryn_session::{AppliedDelta, PresentationEventKind, SessionEvent};
            use sha2::Digest;
            let capture: serde_json::Value =
                serde_json::from_str(include_str!("house-item-effects-wire.json"))?;
            assert_eq!(
                capture["schema"],
                "OTERYN_NATIVE_HOUSE_ITEM_EFFECTS_PLAY/v1"
            );
            assert_eq!(
                capture["source"]["crystal_sha"],
                "201189f871d23597b82caf88fd611454a2ddefb8"
            );
            for row in capture["effects"]
                .as_array()
                .ok_or("captured effect rows")?
            {
                let raw = row["payload_hex"]
                    .as_str()
                    .ok_or("captured House payload")?;
                if raw.len() % 2 != 0 {
                    return Err("odd captured House hex".into());
                }
                let bytes = raw
                    .as_bytes()
                    .as_chunks::<2>().0.iter()
                    .map(
                        |pair| -> Result<u8, Box<dyn std::error::Error + Send + Sync>> {
                            Ok(u8::from_str_radix(std::str::from_utf8(pair)?, 16)?)
                        },
                    )
                    .collect::<Result<Vec<_>, _>>()?;
                let digest = sha2::Sha256::digest(&bytes)
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>();
                assert_eq!(row["payload_sha256"].as_str(), Some(digest.as_str()));
                let events = oteryn_session::item_effects::decode_item_effect_batch(&bytes)
                    .map_err(|e| format!("real committed House effect decode: {e:?}"))?;
                let expected = row["expected_effect_id"]
                    .as_u64()
                    .ok_or("source effect ID")? as u32;
                let position = events
                    .iter()
                    .find_map(|event| match event.kind {
                        PresentationEventKind::MagicEffect {
                            position,
                            effect_id,
                            ..
                        } if effect_id == expected => Some(position),
                        _ => None,
                    })
                    .ok_or("captured donor fish/harp effect")?;
                let map_raw = row["world_map_snapshot_hex"]
                    .as_str()
                    .ok_or("actual captured House map")?;
                let map_bytes = map_raw
                    .as_bytes()
                    .as_chunks::<2>().0.iter()
                    .map(
                        |pair| -> Result<u8, Box<dyn std::error::Error + Send + Sync>> {
                            Ok(u8::from_str_radix(std::str::from_utf8(pair)?, 16)?)
                        },
                    )
                    .collect::<Result<Vec<_>, _>>()?;
                assert_eq!(
                    sha2::Sha256::digest(&map_bytes)
                        .iter()
                        .map(|byte| format!("{byte:02x}"))
                        .collect::<String>(),
                    row["world_map_snapshot_sha256"]
                        .as_str()
                        .ok_or("House map SHA")?
                );
                let map = oteryn_session::world_map::decode_world_map_snapshot(&map_bytes)
                    .map_err(|error| format!("actual House map decode: {error:?}"))?;
                let viewer = &row["viewer"]["position"];
                assert_eq!(Some(i64::from(map.header.origin.x)), viewer[0].as_i64());
                assert_eq!(Some(i64::from(map.header.origin.y)), viewer[1].as_i64());
                assert_eq!(
                    Some(i64::from(map.header.origin.floor)),
                    viewer[2].as_i64().map(|floor| -floor)
                );
                assert_eq!(
                    i32::from(position.floor),
                    viewer[2].as_i64().ok_or("runtime House viewer floor")? as i32
                );
                let (link, _commands, outbox) = crate::play::play_channel();
                let mut view = crate::play::PlayView::new(
                    oteryn_renderer::TileCoord::new(map.header.origin.x, map.header.origin.y),
                    map.header.origin.floor,
                    &[],
                )?;
                outbox.push(vec![SessionEvent::WorldMapSnapshot(AppliedDelta {
                    server_sequence: 1,
                    base_revision: 0,
                    new_revision: 1,
                    value: map.clone(),
                })]);
                assert!(view.tick(&link).is_ok());
                let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
                let world = crate::map::World::resolve_live_map(&root, &map)?;
                view.local.set_world(std::sync::Arc::new(world));
                view.rebuild()?;
                let base = view.scene().sprites().len();
                outbox.push(vec![SessionEvent::ItemEffects(AppliedDelta {
                    server_sequence: 2,
                    base_revision: 0,
                    new_revision: 1,
                    value: events.clone(),
                })]);
                assert!(view.tick(&link).is_ok());
                assert!(view.visuals.iter().any(|cue| matches!(cue.kind,
                PresentationEventKind::MagicEffect {position:actual,effect_id,..}
                    if effect_id==expected && actual.x==position.x && actual.y==position.y
                    && actual.floor==map.header.origin.floor)));
                assert!(
                    view.scene().sprites().len() > base,
                    "committed House effect reaches native rendering"
                );
                assert!(view.take_sound_cues().is_empty());
                view.advance_presentation(
                    std::time::Instant::now() + std::time::Duration::from_secs(2),
                )?;
                assert_eq!(view.scene().sprites().len(), base);
            }
            Ok(())
        })?
        .join()
        .map_err(|_| "native actual House effects thread panicked")?
}

include!("santa_gift_play.rs");
