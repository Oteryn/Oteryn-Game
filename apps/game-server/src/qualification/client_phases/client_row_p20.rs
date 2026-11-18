//! P20 client replay capture: the real King Tibianus promotion dialogue (greeting, the 20000 gold
//! quote and the confirmation reply of a character that cannot be promoted yet). The consumer is
//! `DialoguePanel::request`/`apply` in `crates/client-ui/src/features/0003-npc-service`.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind, EntityRef};
use oteryn_protocol_oteryn::npc_service as wire;

pub(super) fn capture(row: &str, name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!(
            "{row}_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"
        ));
    }
}

/// Walks next to the authored NPC `key` and returns its visible actor.
pub(super) async fn approach_npc(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    key: &str,
) -> TestResult<EntityRef> {
    let npc = npcs
        .npcs
        .iter()
        .find(|npc| npc.key == key)
        .ok_or_else(|| format!("{key} absent from the compiled world"))?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        npc.placements.iter().any(|place| {
            place.floor == at.floor && at.x.abs_diff(place.x).max(at.y.abs_diff(place.y)) <= 3
        })
    })
    .ok_or_else(|| format!("{key} unreachable"))?;
    walk(client, world, &path).await?;
    client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entity| {
                entity.kind == EntityKind::Npc
                    && npc.placements.iter().any(|place| {
                        entity.position.x == i32::from(place.x)
                            && entity.position.y == i32::from(place.y)
                            && entity.position.floor == -i16::from(place.floor)
                    })
            })
        })
        .map(|entity| entity.entity)
        .ok_or_else(|| format!("{key} not visible after walking to it").into())
}

/// Sends one talk intent and returns the raw conversation payload with its decoded lines.
pub(super) async fn talk(
    client: &mut DevClientSession,
    npc: EntityRef,
    text: &str,
) -> TestResult<(Vec<u8>, Vec<String>)> {
    tokio::time::sleep(Duration::from_millis(260)).await;
    let payload = wire::encode_npc_talk_intent(&wire::NpcTalkIntent {
        npc_actor: npc,
        text: text.into(),
    })
    .map_err(|error| format!("{text} encode: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_NPC_SERVICE_V1,
            wire::COMMAND_TYPE_NPC_TALK_INTENT,
            &payload,
        )
        .await?;
    let disposition = wire::decode_npc_intent_result(&reply.payload)
        .map_err(|error| format!("{text} reply decode: {error:?}"))?;
    if reply.status != CommandStatus::Accepted || disposition != wire::NpcIntentDisposition::Ok {
        return Err(format!("talk {text:?} refused: {disposition:?}").into());
    }
    loop {
        let delta = client
            .session_mut()
            .read_gated_delta(wire::STATE_DOMAIN_NPC_CONVERSATION, 1)
            .await?;
        if delta.server_sequence <= reply.server_sequence {
            continue;
        }
        let conversation = wire::decode_npc_conversation(&delta.payload)
            .map_err(|error| format!("{text} conversation decode: {error:?}"))?;
        if conversation.open.map(|(actor, _)| actor) != Some(npc) {
            return Err(format!("{text} delta belonged to another NPC").into());
        }
        if conversation
            .lines
            .iter()
            .any(|line| line.contains(['|', '\n']))
        {
            return Err("conversation line contains a reserved separator".into());
        }
        return Ok((delta.payload, conversation.lines));
    }
}

pub(super) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn promotion_confirmation(answer: &[String], promoted: bool, unchanged: bool) -> bool {
    let [line] = answer else { return false };
    match line.as_str() {
        "You are now promoted." => promoted,
        "You are not eligible for this service yet."
        | "You already have this blessing."
        | "You need current Premium for this service."
        | "You do not have enough gold for this service." => unchanged,
        _ => false,
    }
}

#[test]
fn promotion_confirmation_must_explain_the_purchase_outcome() {
    assert!(!promotion_confirmation(&["Welcome.".into()], false, true));
    assert!(!promotion_confirmation(
        &["You are now promoted.".into()],
        false,
        true
    ));
    assert!(promotion_confirmation(
        &["You are now promoted.".into()],
        true,
        false
    ));
    assert!(!promotion_confirmation(
        &["You need current Premium for this service.".into()],
        true,
        false
    ));
    assert!(promotion_confirmation(
        &["You need current Premium for this service.".into()],
        false,
        true
    ));
}

async fn promotion_state(
    connection: &mut sqlx::PgConnection,
    clients: &SeamClients<'_>,
) -> TestResult<(bool, i64)> {
    let promoted = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_promotions WHERE character_id = encode($1, 'hex')::uuid)")
        .bind(clients.characters[0].as_slice()).fetch_one(&mut *connection).await?;
    Ok((
        promoted,
        super::client_row_p05::gold(connection, clients).await?,
    ))
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
    let mut client = connect_character(clients, 0x52, 0).await?;
    let npc = approach_npc(
        &mut client,
        world,
        &metadata.npcs,
        "oteryn:npc.king_tibianus",
    )
    .await?;
    let (greeting_bytes, greeting) = talk(&mut client, npc, "hi").await?;
    if greeting.is_empty() {
        return Err("hi produced an empty conversation".into());
    }
    capture("P20", "p20-greeting.bin", &greeting_bytes);
    let (quote_bytes, quote) = talk(&mut client, npc, "promotion").await?;
    if quote != ["Do you want to be promoted for 20000 gold coins?"] {
        return Err(format!("promotion quote observed={quote:?}").into());
    }
    capture("P20", "p20-quote.bin", &quote_bytes);
    use sqlx::Connection as _;
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let (promoted_before, gold_before) = promotion_state(&mut connection, clients).await?;
    let (confirm_bytes, confirm) = talk(&mut client, npc, "yes").await?;
    let (promoted_after, gold_after) = promotion_state(&mut connection, clients).await?;
    if !promotion_confirmation(
        &confirm,
        !promoted_before && promoted_after && gold_before.checked_sub(20_000) == Some(gold_after),
        (promoted_before, gold_before) == (promoted_after, gold_after),
    ) {
        return Err(format!("promotion confirmation disagrees with durable state: {confirm:?}; promoted {promoted_before}->{promoted_after}; gold {gold_before}->{gold_after}").into());
    }
    capture("P20", "p20-confirm.bin", &confirm_bytes);
    let expected = format!(
        "npc_identity={}\nnpc_generation={}\ngreeting={}\nquote={}\nconfirm={}\n",
        hex(&npc.identity),
        npc.generation,
        greeting.join("|"),
        quote.join("|"),
        confirm.join("|")
    );
    capture("P20", "p20-expected.txt", expected.as_bytes());
    let _ = client.session_mut().logout().await?;
    Ok(expected.replace('\n', ";").replace(' ', "_"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P20.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P20.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P20.client status=FAIL evidence=timeout"),
    }
}
