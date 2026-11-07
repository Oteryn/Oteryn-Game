// Native panels consume exact real-world Edvard snapshot/delta bytes.
#[test]
fn srv7_edvard_actual_world_dialogue_shop_and_wrapper_reach_native_client()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::{AppliedDelta, SessionEvent, npc_service};
    let capture: serde_json::Value = serde_json::from_str(include_str!("edvard-wire.json"))?;
    let selected = capture["viewer"]["selected_capabilities"]
        .as_array()
        .ok_or("actual selection")?;
    assert!(selected.contains(&serde_json::json!(3)));
    let records = capture["packets"].as_array().ok_or("actual packets")?;
    let initial = |domain| {
        records
            .iter()
            .find(|r| r["kind"] == "snapshot" && r["domain_id"] == domain)
            .ok_or("actual initial domain")
    };
    let spatial = oteryn_session::decode_world_spatial_entities_snapshot_with_item_handles(
        &edvard_bytes(initial(1)?, "payload_hex")?,
    )
    .map_err(|e| format!("actual Edvard spatial: {e:?}"))?;
    let viewer = &capture["viewer"]["position"];
    assert_eq!(
        [
            i64::from(spatial.actor_position.x),
            i64::from(spatial.actor_position.y),
            i64::from(spatial.actor_position.floor)
        ],
        [
            viewer[0].as_i64().ok_or("actual viewer x")?,
            viewer[1].as_i64().ok_or("actual viewer y")?,
            viewer[2].as_i64().ok_or("actual viewer floor")?
        ]
    );
    let source = &capture["source"]["source_position"];
    let npc = spatial
        .entities
        .iter()
        .find(|e| {
            e.kind == oteryn_session::EntityKind::Npc
                && i64::from(e.position.x) == source[0].as_i64().unwrap_or(i64::MIN)
                && i64::from(e.position.y) == source[1].as_i64().unwrap_or(i64::MIN)
                && i64::from(e.position.floor) == source[2].as_i64().unwrap_or(i64::MIN)
        })
        .ok_or("source Edvard is actually visible to this independent viewer")?;
    let before = oteryn_session::item_view::decode_character_inventory_with_equip_drop(
        &edvard_bytes(initial(9)?, "payload_hex")?,
        false,
    )
    .map_err(|e| format!("actual Edvard initial inventory: {e:?}"))?;
    let (link, mut commands, outbox) = play_channel();
    outbox.inventory_initialize(Some(before.clone()), false);
    {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        mailbox.dialogue.selected = true;
        mailbox.npc_shop.live.selected = true;
    }
    let mut native = view()?.with_entities(spatial.entities.clone())?;
    native.place(spatial.actor_position);
    native.set_own_identity(spatial.own_identity);
    if selected.contains(&serde_json::json!(18)) {
        let row = initial(17)?;
        let map = oteryn_session::world_map::decode_world_map_snapshot(&edvard_bytes(
            row,
            "payload_hex",
        )?)
        .map_err(|e| format!("actual Edvard map:{e:?}"))?;
        assert_eq!(
            (
                map.header.origin.x,
                map.header.origin.y,
                map.header.origin.floor
            ),
            (
                spatial.actor_position.x,
                spatial.actor_position.y,
                -spatial.actor_position.floor
            )
        );
        native.set_live_map(&map)?;
    }
    native.rebuild()?;
    assert_eq!(
        native.npc_at(oteryn_renderer::TileCoord::new(
            npc.position.x,
            npc.position.y
        )),
        Some(npc.entity)
    );
    link.talk_to_npc(npc.entity, "hi");
    assert!(
        matches!(commands.try_recv(),Ok(crate::combat_input::PlayCommand::NpcTalk(intent)) if intent.npc_actor==npc.entity&&intent.text=="hi")
    );
    let refs = &capture["qualified_source_item_refs"];
    let source_ref = u32::try_from(refs["2976"].as_u64().ok_or("qualified source2976 ref")?)?;
    let wrapper_ref = u32::try_from(refs["23398"].as_u64().ok_or("qualified source23398 ref")?)?;
    let mut bought = false;
    for r in records.iter().filter(|r| r["kind"] == "delta") {
        let seq = r["server_sequence"].as_u64().ok_or("actual sequence")?;
        let base = r["base_revision"].as_u64().ok_or("actual base")?;
        let new = r["new_revision"].as_u64().ok_or("actual revision")?;
        let bytes = edvard_bytes(r, "payload_hex")?;
        match r["domain_id"].as_u64() {
            Some(7) => {
                let value = npc_service::decode_npc_conversation(&bytes)
                    .map_err(|e| format!("actual HI reply:{e:?}"))?;
                assert_eq!(
                    value.open.as_ref().map(|(actor, _)| *actor),
                    Some(npc.entity)
                );
                outbox.push(vec![SessionEvent::NpcConversation(AppliedDelta {
                    server_sequence: seq,
                    base_revision: base,
                    new_revision: new,
                    value,
                })]);
                native.tick(&link).map_err(|e| format!("native HI:{e:?}"))?;
                assert!(
                    !oteryn_client_ui::npc_dialogue::panel_text(&link.npc_dialogue_panel())
                        .is_empty()
                );
                link.talk_to_npc(npc.entity, "trade");
                assert!(
                    matches!(commands.try_recv(),Ok(crate::combat_input::PlayCommand::NpcTalk(intent)) if intent.npc_actor==npc.entity&&intent.text=="trade")
                );
            }
            Some(8) => {
                let value = npc_service::decode_npc_trade_window(&bytes)
                    .map_err(|e| format!("actual TRADE:{e:?}"))?;
                assert_eq!(value.open.map(|(actor, _)| actor), Some(npc.entity));
                assert!(!value.offers.is_empty());
                outbox.push(vec![SessionEvent::NpcTradeWindow(AppliedDelta {
                    server_sequence: seq,
                    base_revision: base,
                    new_revision: new,
                    value,
                })]);
                native
                    .tick(&link)
                    .map_err(|e| format!("native TRADE:{e:?}"))?;
                let panel = link.npc_shop_panel();
                assert!(panel.open);
                assert!(!panel.live.quotes().is_empty());
                assert!(!oteryn_client_ui::npc_shop::panel_text(&panel).is_empty());
                let quote = panel
                    .live
                    .quotes()
                    .into_iter()
                    .find(|quote| {
                        quote.item == source_ref && quote.side == oteryn_session::NpcTradeSide::Buy
                    })
                    .ok_or("actual source birdcage quote")?;
                link.npc_trade_request(quote, 1);
                assert!(
                    matches!(commands.try_recv(),Ok(crate::combat_input::PlayCommand::NpcTrade(intent)) if intent.npc_actor==npc.entity&&intent.catalogue_revision==quote.revision&&intent.expected_unit_price==quote.price&&intent.quantity==1&&intent.side==quote.side)
                );
            }
            Some(9) => {
                let value = oteryn_session::item_view::decode_character_inventory_with_equip_drop(
                    &bytes, false,
                )
                .map_err(|e| format!("actual purchased23398 inventory:{e:?}"))?;
                let added = value
                    .entries
                    .iter()
                    .find(|entry| !before.entries.iter().any(|old| old.handle == entry.handle))
                    .ok_or("real purchased wrapper handle")?;
                let wrapper = *added;
                assert_eq!(wrapper.item_definition_ref.get(), wrapper_ref);
                assert_eq!(wrapper.count.get(), 1);
                outbox.push(vec![SessionEvent::Inventory(AppliedDelta {
                    server_sequence: seq,
                    base_revision: base,
                    new_revision: new,
                    value,
                })]);
                native
                    .tick(&link)
                    .map_err(|e| format!("native wrapper:{e:?}"))?;
                assert!(link.inventory_panel().rows().contains(&wrapper));
                assert!(link.inventory_select(wrapper.handle));
                assert_eq!(link.inventory_panel().selected_item(), Some(wrapper));
                bought = true;
            }
            _ => {}
        }
    }
    assert!(
        bought,
        "actual server BUY must populate native inventory; producer qualifies source23398 and replay no duplicate"
    );
    Ok(())
}
fn edvard_bytes(
    record: &serde_json::Value,
    key: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let hex = record[key].as_str().ok_or("actual captured bytes")?;
    if !hex.len().is_multiple_of(2) {
        return Err("odd actual bytes".into());
    }
    let bytes = hex
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|p| Ok(u8::from_str_radix(std::str::from_utf8(p)?, 16)?))
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    assert_eq!(
        oteryn_client_assets::text_sha256(&bytes),
        record[format!("{key}_sha256")]
            .as_str()
            .ok_or("sealed actual bytes")?
    );
    Ok(bytes)
}
