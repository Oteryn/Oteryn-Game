#[test]
fn srv7_pumpkin_real_claim_inventory_speech_and_magic_reach_native_client()
-> Result<(), PumpkinPortalPlayError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(pumpkin_claim_native_inner)?
        .join()
        .map_err(|_| "actual Pumpkin claim native thread")?
}
fn pumpkin_claim_native_inner() -> Result<(), PumpkinPortalPlayError> {
    use oteryn_client_runtime::world_interaction::{Action, Target};
    use oteryn_session::{
        AppliedDelta, ChatLine, ChatSpeechMode, PresentationEventKind, SessionEvent,
    };
    let capture: serde_json::Value =
        serde_json::from_str(include_str!("pumpkin-portal-wire.json"))?;
    let claim = &capture["claim_presentation"];
    assert_eq!(claim["fresh_speech"], "Happy Halloween!");
    assert_eq!(claim["effect_id"], 28);
    assert_eq!(claim["replay_cues"], 0);
    assert_eq!(
        oteryn_session::world_context::world_object::decode_use_result(&pumpkin_claim_record(
            claim,
            "use_result_hex"
        )?)
        .map_err(|e| format!("actual static reward result:{e:?}"))?,
        oteryn_session::world_context::world_object::UseDisposition::Committed,
    );
    let selected = claim["selected_capabilities"]
        .as_array()
        .ok_or("actual claim selection")?
        .iter()
        .map(|v| u32::try_from(v.as_u64().ok_or("selected capability")?).map_err(Into::into))
        .collect::<Result<Vec<_>, PumpkinPortalPlayError>>()?;
    assert!(selected.contains(&386));
    assert!(!selected.contains(&288));
    let snapshots = claim["snapshots"]
        .as_array()
        .ok_or("actual claim bootstrap snapshots")?;
    let snapshot = |domain: u64| {
        snapshots
            .iter()
            .find(|row| row["domain"].as_u64() == Some(domain))
            .ok_or("actual claim required domain")
    };
    let inventory = snapshot(9)?;
    let map_record = snapshot(17)?;
    let chat = snapshot(12)?;
    let before = oteryn_session::item_view::decode_character_inventory_with_equip_drop(
        &pumpkin_claim_record(inventory, "payload_hex")?,
        selected.contains(&12),
    )
    .map_err(|e| format!("actual before-claim inventory:{e:?}"))?;
    let map = oteryn_session::world_map::decode_world_map_snapshot(&pumpkin_claim_record(
        map_record,
        "payload_hex",
    )?)
    .map_err(|e| format!("actual reward-room map:{e:?}"))?;
    let viewer = &claim["observer_native"];
    assert_eq!(viewer["x"].as_i64(), Some(i64::from(map.header.origin.x)));
    assert_eq!(viewer["y"].as_i64(), Some(i64::from(map.header.origin.y)));
    assert_eq!(
        viewer["floor"].as_i64(),
        Some(i64::from(map.header.origin.floor))
    );
    assert_eq!(
        claim["observer_runtime"]["floor"].as_i64().map(|f| -f),
        viewer["floor"].as_i64()
    );
    let actor = claim["observer_actor_hex"]
        .as_str()
        .ok_or("independent actual claim actor")?;
    assert_eq!(actor.len(), 32);
    let body = map
        .tiles
        .iter()
        .find(|tile| tile.items.iter().any(|item| item.appearance_id == 12816))
        .ok_or("actual static source12816 in reward room")?
        .position;
    assert_eq!(body.floor, map.header.origin.floor);
    let rooms = oteryn_session::ChatLog::decode_snapshot(
        u32::try_from(chat["snapshot_type"].as_u64().ok_or("actual chat type")?)?,
        &pumpkin_claim_record(chat, "payload_hex")?,
        &selected,
    )?;
    let (link, mut commands, outbox) = play_channel();
    outbox.chat_initialize(selected.contains(&7));
    outbox.targeting_initialize(selected.contains(&15));
    outbox.inventory_initialize(Some(before.clone()), selected.contains(&61));
    outbox.chat_public_state(rooms.rooms(), rooms.lines().cloned().collect());
    let mut view = view()?;
    outbox.push(vec![SessionEvent::WorldMapSnapshot(AppliedDelta {
        server_sequence: map_record["server_sequence"].as_u64().unwrap_or(0),
        base_revision: 0,
        new_revision: map_record["revision"]
            .as_u64()
            .ok_or("actual map revision")?,
        value: map.clone(),
    })]);
    view.tick(&link)
        .map_err(|e| format!("actual claim native map frame:{e:?}"))?;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    view.local
        .set_world(std::sync::Arc::new(World::resolve_live_map(&root, &map)?));
    view.rebuild()?;
    let knife_ref = u32::try_from(
        claim["knife_definition_ref"]
            .as_u64()
            .ok_or("actual source5908 indexed reference")?,
    )?;
    let knife = before
        .entries
        .iter()
        .find(|entry| entry.item_definition_ref.get() == knife_ref)
        .ok_or("actual source knife inventory handle")?;
    assert!(link.inventory_select(knife.handle));
    assert!(link.inventory_use_with());
    assert!(view.use_with_click(TileCoord::new(body.x, body.y), &link));
    assert!(
        matches!(
            commands.try_recv(),
            Ok(crate::combat_input::PlayCommand::WorldContext(command))
                if command.action == Action::UseOn(knife.handle)
                    && matches!(command.target, Target::Object(_))
                    && command.selection.as_ref().is_some_and(|selection| {
                        selection.position.floor == map.header.origin.floor
                            && selection.binding == Some((
                                map.header.content_generation,
                                map.header.bundle_digest,
                                map.header.reset_epoch,
                            ))
                    })
        ),
        "actual knife→static-body gesture retains the server map binding"
    );
    let before_sprites = view.scene().sprites().len();
    let deltas = claim["deltas"]
        .as_array()
        .ok_or("actual fresh-claim native deltas")?;
    let mut lines = Vec::new();
    let mut inventory_updates = 0;
    let mut effect28 = 0;
    for row in deltas {
        let seq = row["server_sequence"]
            .as_u64()
            .ok_or("actual delta sequence")?;
        let base = row["base_revision"].as_u64().ok_or("actual delta base")?;
        let new = row["new_revision"]
            .as_u64()
            .ok_or("actual delta revision")?;
        let bytes = pumpkin_claim_record(row, "payload_hex")?;
        match row["domain"].as_u64() {
            Some(9) => {
                let value = oteryn_session::item_view::decode_character_inventory_with_equip_drop(
                    &bytes,
                    selected.contains(&12),
                )
                .map_err(|e| format!("actual claimed inventory:{e:?}"))?;
                outbox.push(vec![SessionEvent::Inventory(AppliedDelta {
                    server_sequence: seq,
                    base_revision: base,
                    new_revision: new,
                    value,
                })]);
                inventory_updates += 1;
            }
            Some(12) => {
                let value = oteryn_session::decode_chat_line(&bytes)
                    .map_err(|e| format!("actual HappyHalloween native speech:{e:?}"))?;
                if matches!(&value,ChatLine::Local{mode:ChatSpeechMode::MonsterSay,text,..} if text=="Happy Halloween!")
                {
                    lines.push(value);
                }
            }
            Some(105) => {
                let value = oteryn_session::item_effects::decode_item_effect_batch(&bytes)
                    .map_err(|e| format!("actual claim giftwrap cue:{e:?}"))?;
                effect28 += value
                    .iter()
                    .filter(|cue| {
                        matches!(
                            cue.kind,
                            PresentationEventKind::MagicEffect { effect_id: 28, .. }
                        )
                    })
                    .count();
                outbox.push(vec![SessionEvent::ItemEffects(AppliedDelta {
                    server_sequence: seq,
                    base_revision: base,
                    new_revision: new,
                    value,
                })]);
            }
            _ => {}
        }
    }
    assert!(inventory_updates > 0);
    assert_eq!(lines.len(), 1);
    assert_eq!(effect28, 1);
    outbox.chat_public_state(rooms.rooms(), lines);
    view.tick(&link)
        .map_err(|e| format!("actual Pumpkin claim native presentation:{e:?}"))?;
    assert!(oteryn_client_ui::chat::panel_text(&link.chat_panel()).contains("Happy Halloween!"));
    assert!(
        view.visuals.iter().any(|cue| matches!(
            cue.kind,
            PresentationEventKind::MagicEffect { effect_id: 28, position, .. }
                if position.floor == map.header.origin.floor
        )),
        "actual giftwrap pulse stays on the independently captured reward-room floor"
    );
    assert!(
        view.scene().sprites().len() > before_sprites,
        "native reward-room scene draws actual giftwrap cue"
    );
    let reward = &claim["claimed_reward"];
    let reference = u32::try_from(
        reward["item_definition_ref"]
            .as_u64()
            .ok_or("actual indexed claimed reward")?,
    )?;
    let count = u32::try_from(reward["count"].as_u64().ok_or("actual claimed count")?)?;
    let panel = link.inventory_panel();
    let shown = panel
        .rows()
        .into_iter()
        .find(|entry| entry.item_definition_ref.get() == reference)
        .ok_or("actual minted reward is visible in native inventory")?;
    let old: u32 = before
        .entries
        .iter()
        .filter(|entry| entry.item_definition_ref.get() == reference)
        .map(|entry| entry.count.get())
        .sum();
    let now: u32 = panel
        .rows()
        .iter()
        .filter(|entry| entry.item_definition_ref.get() == reference)
        .map(|entry| entry.count.get())
        .sum();
    assert!(
        now >= old,
        "actual reward inventory cannot lose the selected reward stack"
    );
    assert_eq!(now - old, count);
    assert!(link.inventory_select(shown.handle));
    assert_eq!(link.inventory_panel().selected_item(), Some(shown));
    assert!(
        link.inventory_panel()
            .text()
            .contains(&format!("×{}", shown.count)),
        "existing native inventory selection shows actual reward quantity"
    );
    Ok::<(), PumpkinPortalPlayError>(())
}

fn pumpkin_claim_record(
    row: &serde_json::Value,
    key: &str,
) -> Result<Vec<u8>, PumpkinPortalPlayError> {
    use sha2::Digest;
    let raw = row[key].as_str().ok_or("actual captured claim record")?;
    if !raw.len().is_multiple_of(2) {
        return Err("odd claim record hex".into());
    }
    let bytes = raw
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|p| Ok(u8::from_str_radix(std::str::from_utf8(p)?, 16)?))
        .collect::<Result<Vec<_>, PumpkinPortalPlayError>>()?;
    assert_eq!(
        row[format!("{key}_sha256")].as_str(),
        Some(
            sha2::Sha256::digest(&bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
                .as_str()
        )
    );
    Ok(bytes)
}
