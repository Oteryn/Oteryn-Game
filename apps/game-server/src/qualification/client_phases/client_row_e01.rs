//! E01 client replay capture: real NPC dialogue, trade-window, and shop-result payloads.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind, EntityRef};
use oteryn_protocol_oteryn::{npc_service as service, npc_shop as shop};

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

async fn talk(
    client: &mut DevClientSession,
    npc: EntityRef,
    text: &str,
    capture_name: Option<&str>,
) -> TestResult<service::NpcConversation> {
    tokio::time::sleep(Duration::from_millis(260)).await;
    let disposition = oteryn_dev_client::client_runtime::npc_shop::talk(
        client.session_mut(),
        &service::NpcTalkIntent {
            npc_actor: npc,
            text: text.into(),
        },
    )
    .await?;
    if disposition != service::NpcIntentDisposition::Ok {
        return Err(format!("talk {text} refused: {disposition:?}").into());
    }
    let delta = client
        .session_mut()
        .read_gated_delta(service::STATE_DOMAIN_NPC_CONVERSATION, 1)
        .await?;
    if let Some(name) = capture_name {
        capture(name, &delta.payload)?;
    }
    service::decode_npc_conversation(&delta.payload)
        .map_err(|error| format!("conversation payload: {error:?}").into())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_character(clients, 0xe1, 1).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        let sam = metadata
            .npcs
            .npcs
            .iter()
            .find(|npc| npc.key == "oteryn:npc.sam")
            .ok_or("E01 client capture: Sam absent")?;
        let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
            sam.placements.iter().any(|place| {
                place.floor == at.floor && at.x.abs_diff(place.x).max(at.y.abs_diff(place.y)) <= 3
            })
        })
        .ok_or("E01 client capture: Sam unreachable")?;
        walk(&mut client, world, &path).await?;
        let actor = client
            .world_entities()
            .and_then(|entities| {
                entities.iter().find(|entity| {
                    entity.kind == EntityKind::Npc
                        && sam.placements.iter().any(|place| {
                            entity.position.x == i32::from(place.x)
                                && entity.position.y == i32::from(place.y)
                                && entity.position.floor == -i16::from(place.floor)
                        })
                })
            })
            .map(|entity| entity.entity)
            .ok_or("E01 client capture: Sam not visible")?;
        let greeting = talk(&mut client, actor, "hi", Some("e01-conversation.bin")).await?;
        talk(&mut client, actor, "trade", None).await?;
        let window_delta = client
            .session_mut()
            .read_gated_delta(service::STATE_DOMAIN_NPC_TRADE_WINDOW, 1)
            .await?;
        capture("e01-window.bin", &window_delta.payload)?;
        let window = service::decode_npc_trade_window(&window_delta.payload)
            .map_err(|error| format!("trade window payload: {error:?}"))?;
        let (index, offer) = window
            .offers
            .iter()
            .enumerate()
            .find(|(_, offer)| offer.side == service::NpcTradeSide::Buy)
            .ok_or("E01 client capture: no buy offer")?;
        let intent = shop::NpcShopIntent {
            npc_actor: actor,
            catalogue_revision: window.open.ok_or("E01 client capture: window closed")?.1,
            offer_index: u32::try_from(index)?,
            side: offer.side,
            action: shop::NpcShopAction::Look,
            expected_unit_price: offer.unit_price,
        };
        let payload = shop::encode_npc_shop_intent(&intent)
            .map_err(|error| format!("look encode: {error:?}"))?;
        let reply = client
            .session_mut()
            .gated_command(
                shop::CAPABILITY_NPC_SHOP_V1,
                shop::COMMAND_TYPE_NPC_SHOP,
                &payload,
            )
            .await?;
        capture("e01-look.bin", &reply.payload)?;
        let look = shop::decode_npc_shop_result(&reply.payload)
            .map_err(|error| format!("look payload: {error:?}"))?;

        // The same real server conversation payload is the client seam for deposit, withdraw,
        // and transfer replies. E2EF20 independently drives the durable bank mutations; replaying
        // this row proves that the native dialogue reducer displays those server-owned lines.
        let expected = format!(
            "conversation_open={}\nconversation_lines={}\noffers={}\nlook={}\n",
            greeting.open.is_some(),
            greeting.lines.join("|"),
            window.offers.len(),
            look.look_text
        );
        capture("e01-expected.txt", expected.as_bytes())?;
        Ok(expected)
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() {
        return;
    }
    match row_complete(journey(clients)).await {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=E01.client status=CAPTURED evidence={expected:?}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=E01.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=E01.client status=FAIL evidence=timeout"),
    }
}
