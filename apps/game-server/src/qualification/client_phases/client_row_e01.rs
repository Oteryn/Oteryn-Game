//! E01 client replay capture: real NPC dialogue, trade-window, and shop-result payloads.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityRef};
use oteryn_protocol_oteryn::{npc_service as service, npc_shop as shop};

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!(
            "E01_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"
        ));
    }
}

pub(super) async fn talk(
    client: &mut DevClientSession,
    npc: EntityRef,
    text: &str,
    capture_name: Option<&str>,
) -> TestResult<service::NpcConversation> {
    accepted_talk(client, npc, text).await?;
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

async fn accepted_talk(client: &mut DevClientSession, npc: EntityRef, text: &str) -> TestResult {
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
    Ok(())
}

pub(super) async fn open_trade_window(
    client: &mut DevClientSession,
    actor: EntityRef,
    capture_name: Option<&str>,
) -> TestResult<service::NpcTradeWindow> {
    accepted_talk(client, actor, "trade").await?;
    let delta = client
        .session_mut()
        .read_gated_delta(service::STATE_DOMAIN_NPC_TRADE_WINDOW, 1)
        .await?;
    let details = client
        .selected_capabilities()
        .contains(&oteryn_protocol_oteryn::trade_details::CAPABILITY_TRADE_DETAILS_V1);
    let window = service::decode_npc_trade_window_for(&delta.payload, details)
        .map_err(|error| format!("trade window payload: {error:?}"))?;
    if window.open.map(|(npc, _)| npc) != Some(actor) {
        return Err("trade window did not open for Sam".into());
    }
    if let Some(name) = capture_name {
        capture(name, &delta.payload);
        capture(
            "e01-window-details.txt",
            if details { b"true" } else { b"false" },
        );
    }
    Ok(window)
}

async fn interaction(client: &mut DevClientSession, actor: EntityRef) -> TestResult<String> {
    let greeting = talk(client, actor, "hi", Some("e01-conversation.bin")).await?;
    if greeting.open.map(|(npc, _)| npc) != Some(actor) {
        return Err("greet: Sam did not open the native conversation".into());
    }
    let window = open_trade_window(client, actor, Some("e01-window.bin")).await?;
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
    let payload =
        shop::encode_npc_shop_intent(&intent).map_err(|error| format!("look encode: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            shop::CAPABILITY_NPC_SHOP_V1,
            shop::COMMAND_TYPE_NPC_SHOP,
            &payload,
        )
        .await?;
    capture("e01-look.bin", &reply.payload);
    let look = shop::decode_npc_shop_result(&reply.payload)
        .map_err(|error| format!("look payload: {error:?}"))?;

    if reply.status != CommandStatus::Accepted
        || look.disposition != service::NpcIntentDisposition::Ok
        || look.look_text.is_empty()
    {
        return Err(format!("look: status={:?} result={look:?}", reply.status).into());
    }

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

pub(super) async fn run_packet(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
) {
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() {
        return;
    }
    let result = verdict::observe(async {
        let mut client = connect_character(clients, 0xe1, 1).await?;
        // The enclosing indexed row guards travel by observed movement progress.
        let actor = super::full_e2ef4::approach_sam(&mut client, world, npcs).await?;
        verdict::indexed_timeout(Duration::from_secs(90), interaction(&mut client, actor)).await?
    })
    .await;
    match result {
        Ok(expected) => evidence(&format!(
            "e2e_all scenario=E01.client status=CAPTURED evidence={expected:?}"
        )),
        Err(error) => evidence(&format!(
            "e2e_all scenario=E01.client status=FAIL evidence={error}"
        )),
    }
}

#[cfg(test)]
#[test]
fn e01_shop_handshake_does_not_wait_for_a_second_conversation_delta() {
    let full = include_str!("full_e2ef4.rs");
    let full_shop = full.split_once("async fn shop(").unwrap().1;
    let full_handshake = full_shop.split_once("let (index, offer)").unwrap().0;
    assert!(
        !full_handshake.contains("talk(client, actor, \"trade\""),
        "trade acknowledgement must lead directly to the shop-window delta"
    );
    assert!(full_handshake.contains("open_trade_window(client, actor"));
}
