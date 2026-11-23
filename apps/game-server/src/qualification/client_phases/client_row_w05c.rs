//! W05c client replay capture: Rashid's weekday NPC dialogue and trade-window payloads.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityRef};
use oteryn_protocol_oteryn::npc_service as service;

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!("W05C_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
}

async fn talk(
    client: &mut DevClientSession,
    npc: EntityRef,
    text: &str,
    domain: u32,
    capture_name: &str,
) -> TestResult<Vec<u8>> {
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
    let delta = client.session_mut().read_gated_delta(domain, 1).await?;
    capture(capture_name, &delta.payload);
    Ok(delta.payload)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let elapsed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?;
    let destination = oteryn_gameplay_entities::rashid_schedule::destination_at(i64::try_from(
        elapsed.as_millis(),
    )?);
    let target = super::full_row_w05c::destination_tile(destination);
    let mut client = connect_world(clients, 0x08).await?;
    let actor = super::full_row_w05c::approach(&mut client, world, target).await?;
    let conversation_bytes = talk(
        &mut client,
        actor,
        "hi",
        service::STATE_DOMAIN_NPC_CONVERSATION,
        "w05c-conversation.bin",
    )
    .await?;
    let conversation = service::decode_npc_conversation(&conversation_bytes)
        .map_err(|error| format!("conversation payload: {error:?}"))?;
    if conversation.open.map(|open| open.0) != Some(actor) {
        return Err("W05c client capture: conversation not open for Rashid".into());
    }
    let window_bytes = talk(
        &mut client,
        actor,
        "trade",
        service::STATE_DOMAIN_NPC_TRADE_WINDOW,
        "w05c-window.bin",
    )
    .await?;
    let window = service::decode_npc_trade_window(&window_bytes)
        .map_err(|error| format!("trade window payload: {error:?}"))?;
    if window.open.map(|open| open.0) != Some(actor) || window.offers.is_empty() {
        return Err("W05c client capture: Rashid trade window empty".into());
    }
    let expected = format!(
        "conversation_open={}\nconversation_lines={}\noffers={}\n",
        conversation.open.is_some(),
        conversation.lines.join("|"),
        window.offers.len()
    );
    capture("w05c-expected.txt", expected.as_bytes());
    Ok(expected)
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() {
        return;
    }
    match verdict::indexed_timeout(Duration::from_secs(1_800), verdict::observe(journey(clients))).await {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=W05c.client status=CAPTURED evidence={expected:?}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W05c.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=W05c.client status=FAIL evidence=timeout"),
    }
}
