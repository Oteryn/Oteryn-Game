//! A fresh committed source gift must be visible through the actual House connection.
use super::wire::HouseWireCapture;
use super::*;

pub(super) async fn assert_and_capture(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    capture: &std::sync::Arc<std::sync::Mutex<HouseWireCapture>>,
) -> TestResult {
    let viewer = {
        let runtime = owner.runtime.lock().await;
        let state = runtime
            .house_actor_state(
                admitted.runtime_actor.ok_or("actual gift viewer")?,
                admitted.game_session_id,
            )
            .map_err(debug)?;
        serde_json::json!({"house":state.house,"ownership_generation":state.generation.get(),"position":[state.position.x,state.position.y,state.position.floor]})
    };
    let (maps, chats, lines, effects, vitals) = {
        let capture = capture
            .lock()
            .map_err(|_| "gift real presentation capture")?;
        (
            capture.maps.clone(),
            capture.chat_snapshots.clone(),
            capture.chat_lines.clone(),
            capture.effects.clone(),
            capture.vitals.clone(),
        )
    };
    assert_eq!(
        chats.len(),
        1,
        "fresh House source speech requires the actual existing chat bootstrap"
    );
    let line=lines.iter().find(|payload|matches!(oteryn_protocol_oteryn::chat::decode_chat_line(payload),Ok(oteryn_protocol_oteryn::chat::ChatLine::Local{mode:oteryn_protocol_oteryn::chat::ChatSpeechMode::MonsterSay,text,..}) if text=="You... will.... burn!!"))
        .ok_or("fresh committed House dragon must publish donor speech on native chat wire")?;
    let effect=effects.iter().find(|payload|oteryn_protocol_oteryn::presentation::decode_item_effect_batch(payload).is_ok_and(|events|events.iter().any(|event|matches!(event.kind,oteryn_protocol_oteryn::presentation::PresentationEventKind::MagicEffect{effect_id:6,..}))))
        .ok_or("actual House dragon explosionhit6 wire")?;
    let hp = vitals
        .iter()
        .find(|payload| {
            oteryn_protocol_oteryn::actor_spell::decode_actor_vitals(payload)
                .is_ok_and(|value| value.health == 6)
        })
        .ok_or("actual one-point source HP loss wire")?;
    let hex = |bytes: &[u8]| {
        bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    };
    let capture = serde_json::json!({
        "source":{"crystal_sha":"201189f871d23597b82caf88fd611454a2ddefb8","producer_test":"srv7_house_santa_dragon_damages_actor_once_and_replays"},
        "viewer":viewer,"world_map_snapshot_hex":hex(maps.first().ok_or("actual gift map snapshot")?),
        "chat_snapshot_hex":hex(&chats[0]),"chat_line_hex":hex(line),"effect_batch_hex":hex(effect),"vitals_hex":hex(hp),
    });
    println!("SRV7_SANTA_GIFT_PRESENTATION_CAPTURE {capture}");
    Ok(())
}
