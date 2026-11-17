//! E01 client replay capture: real NPC dialogue, trade-window, and shop-result payloads.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind, EntityRef};
use oteryn_protocol_oteryn::{npc_service as service, npc_shop as shop};

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!("E01_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
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
        capture(name, &delta.payload);
    }
    service::decode_npc_conversation(&delta.payload)
        .map_err(|error| format!("conversation payload: {error:?}").into())
}

fn shop_look_is_valid(status: CommandStatus, open: bool, disposition: service::NpcIntentDisposition) -> bool {
    status == CommandStatus::Accepted && open && disposition == service::NpcIntentDisposition::Ok
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
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
    capture("e01-window.bin", &window_delta.payload);
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
        .gated_command(shop::CAPABILITY_NPC_SHOP_V1, shop::COMMAND_TYPE_NPC_SHOP, &payload)
        .await?;
    capture("e01-look.bin", &reply.payload);
    let look = shop::decode_npc_shop_result(&reply.payload)
        .map_err(|error| format!("look payload: {error:?}"))?;

    if !shop_look_is_valid(reply.status, greeting.open.is_some(), look.disposition) {
        return Err(format!("E01 shop LOOK outcome: {look:?}").into());
    }

    bank_routes(&mut client, clients, world, &metadata).await?;
    let expected = format!(
        "conversation_open={}\nconversation_lines={}\noffers={}\nlook={}\n",
        greeting.open.is_some(),
        greeting.lines.join("|"),
        window.offers.len(),
        look.look_text
    );
    capture("e01-expected.txt", expected.as_bytes());
    Ok(expected)
}

async fn bank_routes(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    metadata: &oteryn_world_bundle::bundle::Visited,
) -> TestResult {
    use oteryn_dev_client::client_runtime::npc_shop::balances;
    full_gold::earn(client, world, &metadata.spawns, "E01_CLIENT", 2).await?;
    let (banker, _) = client_row_e02::find_exchange_npc(client, world, &metadata.npcs).await?;
    let before = balances(client.session_mut()).await?.ok_or("E01 sender balance absent")?;
    bank_reply(client, banker, "deposit 2 gold", "Do you want to deposit 2 gold?", "You have deposited 2 gold.", "e01-deposit.bin").await?;
    let deposited = balances(client.session_mut()).await?.ok_or("E01 deposit balance absent")?;
    if before.bank_gold.checked_add(2) != Some(deposited.bank_gold) || before.carried_gold.checked_sub(2) != Some(deposited.carried_gold) {
        return Err(format!("E01 deposit balance mismatch: {before:?} -> {deposited:?}").into());
    }
    bank_reply(client, banker, "withdraw 1 gold", "Do you want to withdraw 1 gold?", "You have withdrawn 1 gold.", "e01-withdraw.bin").await?;
    let withdrawn = balances(client.session_mut()).await?.ok_or("E01 withdrawal balance absent")?;
    if deposited.bank_gold.checked_sub(1) != Some(withdrawn.bank_gold) || deposited.carried_gold.checked_add(1) != Some(withdrawn.carried_gold) {
        return Err(format!("E01 withdrawal balance mismatch: {deposited:?} -> {withdrawn:?}").into());
    }
    let mut peer = with_peer(client, connect_character(clients, 0xe0, 0)).await?;
    let peer_name = {
        let entities = peer.world_entities().ok_or("E01 peer entity view absent")?;
        entities.iter().find(|entity| &entity.entity.identity == entities.own_identity())
            .and_then(|entity| entity.display_name).ok_or("E01 peer display name absent")?
            .as_str().map_err(|error| format!("E01 peer display name: {error:?}"))?.to_owned()
    };
    let peer_before = balances(peer.session_mut()).await?.ok_or("E01 recipient balance absent")?;
    with_peer(&mut peer, bank_reply(client, banker, &format!("transfer 1 gold to {peer_name}"),
        &format!("Do you want to transfer 1 gold to {peer_name}?"),
        &format!("Very well. You have transferred 1 gold to {peer_name}."),
        "e01-transfer.bin")).await?;
    let sender_after = balances(client.session_mut()).await?.ok_or("E01 transfer sender balance absent")?;
    let peer_after = balances(peer.session_mut()).await?.ok_or("E01 transfer recipient balance absent")?;
    if withdrawn.bank_gold.checked_sub(1) != Some(sender_after.bank_gold)
        || withdrawn.carried_gold != sender_after.carried_gold
        || peer_before.bank_gold.checked_add(1) != Some(peer_after.bank_gold)
    {
        return Err("E01 transfer did not debit the sender and credit the recipient".into());
    }
    Ok(())
}

async fn bank_reply(
    client: &mut DevClientSession, banker: EntityRef, request: &str,
    prompt: &str, receipt: &str, file: &str,
) -> TestResult {
    let asks = talk(client, banker, request, None).await?;
    if !asks.lines.iter().any(|line| line == prompt) {
        return Err(format!("E01 {request}: unexpected prompt {:?}", asks.lines).into());
    }
    let confirms = talk(client, banker, "yes", Some(file)).await?;
    if !confirms.lines.iter().any(|line| line == receipt) {
        return Err(format!("E01 {request}: unexpected receipt {:?}", confirms.lines).into());
    }
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=E01.client status=CAPTURED evidence={expected:?}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=E01.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=E01.client status=FAIL evidence=timeout"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shop_look_requires_an_open_conversation_and_successful_reply() {
        let ok = service::NpcIntentDisposition::Ok;
        assert!(!shop_look_is_valid(CommandStatus::Rejected, true, ok));
        assert!(!shop_look_is_valid(CommandStatus::Accepted, false, ok));
        assert!(shop_look_is_valid(CommandStatus::Accepted, true, ok));
    }

}
