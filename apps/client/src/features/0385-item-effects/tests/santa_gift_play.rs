type SantaGiftError = Box<dyn std::error::Error + Send + Sync>;
fn santa_gift_payload(capture: &serde_json::Value, key: &str) -> Result<Vec<u8>, SantaGiftError> {
    use sha2::Digest;
    let raw = capture[key].as_str().ok_or("real Santa gift payload")?;
    if raw.len() % 2 != 0 {
        return Err("odd captured Santa gift hex".into());
    }
    let bytes = raw
        .as_bytes()
        .as_chunks::<2>().0.iter()
        .map(|pair| -> Result<u8, SantaGiftError> {
            Ok(u8::from_str_radix(std::str::from_utf8(pair)?, 16)?)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let digest = sha2::Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(
        Some(digest.as_str()),
        capture["payload_sha256"][key].as_str()
    );
    Ok(bytes)
}
#[test]
fn srv7_santa_dragon_actual_house_speech_vitals_and_magic_reach_native_client()
-> Result<(), SantaGiftError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            use oteryn_session::{
                AppliedDelta, ChatLine, ChatSpeechMode, PresentationEventKind, SessionEvent,
            };
            let capture: serde_json::Value =
                serde_json::from_str(include_str!("santa-gift-wire.json"))?;
            assert_eq!(capture["schema"], "OTERYN_NATIVE_SANTA_GIFT_PLAY/v1");
            let map = oteryn_session::world_map::decode_world_map_snapshot(&santa_gift_payload(
                &capture,
                "world_map_snapshot_hex",
            )?)
            .map_err(|error| format!("gift House map: {error:?}"))?;
            let viewer = &capture["viewer"]["position"];
            assert_eq!(Some(i64::from(map.header.origin.x)), viewer[0].as_i64());
            assert_eq!(Some(i64::from(map.header.origin.y)), viewer[1].as_i64());
            assert_eq!(
                Some(i64::from(map.header.origin.floor)),
                viewer[2].as_i64().map(|floor| -floor)
            );
            let line =
                oteryn_session::decode_chat_line(&santa_gift_payload(&capture, "chat_line_hex")?)
                    .map_err(|error| format!("gift actual speech: {error:?}"))?;
            let (name, text) = match &line {
                ChatLine::Local {
                    speaker_name,
                    mode: ChatSpeechMode::MonsterSay,
                    text,
                    ..
                } => (speaker_name.clone(), text.clone()),
                _ => return Err("source gift must use real local MonsterSay".into()),
            };
            assert_eq!(text, "You... will.... burn!!");
            let rooms = oteryn_session::ChatLog::decode_snapshot(1, &santa_gift_payload(
                &capture,
                "chat_snapshot_hex",
            )?, &[7,288])
            .map_err(|error| format!("gift chat bootstrap: {error:?}"))?;
            let vitals =
                oteryn_session::decode_actor_vitals(&santa_gift_payload(&capture, "vitals_hex")?)
                    .map_err(|error| format!("gift actual vitals: {error:?}"))?;
            assert_eq!(vitals.health, 6, "donor one-point hit after actual HP7 USE");
            let effects = oteryn_session::item_effects::decode_item_effect_batch(
                &santa_gift_payload(&capture, "effect_batch_hex")?,
            )
            .map_err(|error| format!("gift actual magic: {error:?}"))?;
            assert!(
                effects.iter().any(|event| matches!(
                    event.kind,
                    PresentationEventKind::MagicEffect { effect_id: 6, .. }
                )),
                "Crystal utils_definitions.hpp61 explosionhit6"
            );
            let (link, _commands, outbox) = crate::play::play_channel();
            outbox.chat_initialize(true);
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
            view.local
                .set_world(std::sync::Arc::new(crate::map::World::resolve_live_map(
                    &root, &map,
                )?));
            view.rebuild()?;
            // The production Session publisher replaces retained CHAT state and own
            // vitals through these hooks; chat deltas alone update other panel owners.
            outbox.chat_public_state(rooms.rooms(), vec![line]);
            outbox.vitals(Some(vitals));
            assert!(view.tick(&link).is_ok());
            let shown = oteryn_client_ui::chat::panel_text(&link.chat_panel());
            assert!(
                shown.contains(&name) && shown.contains(&text),
                "fresh decoded gift voice reaches the native chat formatter"
            );
            assert_eq!(link.actor_vitals().map(|value| value.health), Some(6));
            let base=view.scene().sprites().len();
            outbox.push(vec![SessionEvent::ItemEffects(AppliedDelta {server_sequence:5,base_revision:0,new_revision:1,value:effects})]);
            assert!(view.tick(&link).is_ok());
            assert!(view.visuals.iter().any(|cue|matches!(cue.kind,PresentationEventKind::MagicEffect{position,effect_id:6,..} if position.floor==map.header.origin.floor)),"new item effect adapter must use the actual native map floor");
            assert!(
                view.scene().sprites().len() > base,
                "actual explosion cue is visible on the captured House floor"
            );
            assert!(view.take_sound_cues().is_empty());
            Ok::<(), SantaGiftError>(())
        })?
        .join()
        .map_err(|_| "native source Santa gift presentation thread")?
}
