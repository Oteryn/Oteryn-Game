//! I07 client replay capture: real quick-loot filter, Gold Pouch destination and loot-all replies.
use super::*;
use oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse;
use oteryn_protocol_oteryn::GameSessionId;
use oteryn_protocol_oteryn::quick_loot::{
    CAPABILITY_QUICK_LOOT_V1, COMMAND_TYPE_QUICK_LOOT, QuickLootAction, QuickLootCategory,
    QuickLootFilterMode, QuickLootRecipient, QuickLootRequest, decode_quick_loot_result,
    encode_quick_loot_request,
};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result =
        verdict::indexed_timeout(Duration::from_secs(30), verdict::observe(journey(clients))).await;
    let result = result.unwrap_or_else(|_| Err("I07 client capture timed out".into()));
    evidence(&format!(
        "e2e_all scenario=I07.client status={}",
        if result.is_ok() { "PASS" } else { "FAIL" }
    ));
    if let Err(error) = result {
        evidence(&format!("I07_CLIENT_GAP {error}"));
    }
}

fn request(action: QuickLootAction) -> QuickLootRequest {
    QuickLootRequest {
        action,
        corpse: None,
        category: None,
        recipient: None,
        recipient_id: None,
        fallback_to_main: false,
        filter_mode: None,
        filter_definition_ref: None,
    }
}

async fn pouch_id(clients: &SeamClients<'_>) -> TestResult<[u8; 16]> {
    const POUCH: &str = "oteryn:item.tibia.i23721";
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let character = clients.characters[1].as_slice();
    let world = clients.world.as_slice();
    let existing: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT uuid_send(i.item_instance_id)
         FROM game_item_character_inbox_locations l
         JOIN game_item_instances i USING (item_instance_id, world_id)
         WHERE l.character_id = encode($1,'hex')::uuid AND l.world_id = encode($2,'hex')::uuid
           AND i.definition_production_key = $3 AND i.lifecycle = 1
         ORDER BY l.ordinal LIMIT 1",
    )
    .bind(character)
    .bind(world)
    .bind(POUCH)
    .fetch_optional(&mut db)
    .await?;
    if let Some(id) = existing {
        return Ok(id.as_slice().try_into()?);
    }

    let items = provisioning::item_index()?;
    let revision = items
        .revision(POUCH)
        .ok_or("Gold Pouch is absent from served content")?;
    // Seed the disposable character's Store Inbox before admission, as in the
    // quick-loot PostgreSQL fixture. The node still validates ownership and the assignment.
    let mut tx = sqlx::Connection::begin(&mut db).await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let id: Vec<u8> = sqlx::query_scalar(
        "WITH ids AS (SELECT uuidv7() AS item, uuidv7() AS mint),
              pouch AS (
                  INSERT INTO game_item_instances
                      (item_instance_id, world_id, definition_family, definition_production_key,
                       definition_revision_ref, quantity, lifecycle, minted_transaction_id)
                  SELECT item, encode($2,'hex')::uuid, 'Item', $3, $4, 1, 1, mint FROM ids
                  RETURNING item_instance_id, world_id
              )
         INSERT INTO game_item_character_inbox_locations
             (item_instance_id, world_id, character_id, ordinal)
         SELECT item_instance_id, world_id, encode($1,'hex')::uuid,
             (SELECT coalesce(max(ordinal), 0) + 1 FROM game_item_character_inbox_locations
              WHERE character_id = encode($1,'hex')::uuid)
         FROM pouch RETURNING uuid_send(item_instance_id)",
    )
    .bind(character)
    .bind(world)
    .bind(POUCH)
    .bind(revision)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(id.as_slice().try_into()?)
}

pub(super) fn replay_envelope(
    session: GameSessionId,
    response: &ServerMechanicResponse,
) -> serde_json::Result<Vec<u8>> {
    serde_json::to_vec(&serde_json::json!({
        "command_id": response.command_id,
        "game_session_id": uuid_text(session.as_bytes()),
        "payload": response.payload,
        "server_sequence": response.server_sequence,
        "status": response.status as u32,
    }))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let pouch_id = pouch_id(clients).await?;
    let mut client = connect_world(clients, 0x07).await?;
    let session = client.join_snapshot().game_session_id;
    character_prep::grant_premium(&client).await?;
    evidence("I07_FIXTURE premium=active");
    let items = provisioning::item_index()?;
    let skip_ref = provisioning::item_reference(&items, 3031)?.get();
    let accept_ref = provisioning::item_reference(&items, 3350)?.get();
    let mut skip = request(QuickLootAction::FilterAdd);
    skip.filter_mode = Some(QuickLootFilterMode::Skip);
    skip.filter_definition_ref = Some(skip_ref);
    let mut accept = request(QuickLootAction::FilterAdd);
    accept.filter_mode = Some(QuickLootFilterMode::Accept);
    accept.filter_definition_ref = Some(accept_ref);
    let mut pouch = request(QuickLootAction::Assign);
    pouch.category = Some(QuickLootCategory::Gold);
    pouch.recipient = Some(QuickLootRecipient::GoldPouch);
    pouch.recipient_id = Some(pouch_id);
    pouch.fallback_to_main = true;
    let requests = [skip, accept, pouch, request(QuickLootAction::LootAll)];
    let mut probes = Vec::new();
    let mut item_count = 0;
    for request in requests {
        let payload = encode_quick_loot_request(&request).map_err(|error| format!("{error:?}"))?;
        let reply = client
            .session_mut()
            .gated_command(CAPABILITY_QUICK_LOOT_V1, COMMAND_TYPE_QUICK_LOOT, &payload)
            .await?;
        if reply.status != CommandStatus::Accepted {
            return Err(format!(
                "quick-loot {:?} refused: {:?}",
                request.action, reply.status
            )
            .into());
        }
        if request.action == QuickLootAction::LootAll {
            item_count = decode_quick_loot_result(&reply.payload)
                .map_err(|error| format!("quick-loot reply: {error:?}"))?
                .items
                .len();
        }
        probes.push((payload, reply));
    }
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = Path::new(&root);
        for (key, (request, response)) in (1..=4).zip(probes) {
            std::fs::write(root.join(format!("i07-{key}-request.bin")), request)?;
            std::fs::write(
                root.join(format!("i07-{key}.json")),
                replay_envelope(session, &response)?,
            )?;
        }
        let session = uuid_text(session.as_bytes());
        let pouch_hex = uuid_text(&pouch_id).replace('-', "");
        std::fs::write(
            root.join("i07-expected.txt"),
            format!(
                "session={session}\npouch={pouch_hex}\nskip={skip_ref} accept={accept_ref} confirmed=3 destinations=1 results={item_count}\n"
            ),
        )?;
    }
    Ok(())
}

#[test]
fn i07_n03_replay_envelope_has_exact_keys_and_a_uuid_v7_session() -> TestResult {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Envelope {
        command_id: u64,
        game_session_id: String,
        payload: Vec<u8>,
        server_sequence: u64,
        status: u32,
    }

    let session =
        GameSessionId::decode(&[1, 2, 3, 4, 5, 6, 0x70, 8, 0xa0, 10, 11, 12, 13, 14, 15, 16])?;
    let response = ServerMechanicResponse {
        command_id: 3,
        server_sequence: 7,
        status: CommandStatus::Accepted,
        payload: vec![0, 255],
    };
    // Deserialize straight from the wire bytes. Deserializing into Value first would
    // collapse repeated keys before the strict schema could reject them.
    let bytes = replay_envelope(session, &response)?;
    let decoded: Envelope = serde_json::from_slice(&bytes)?;
    let id = crate::node::uuid_bytes(&decoded.game_session_id).ok_or("invalid replay UUID text")?;
    assert_eq!(GameSessionId::decode(&id)?, session);
    assert_eq!(
        decoded.game_session_id,
        "01020304-0506-7008-a00a-0b0c0d0e0f10"
    );
    assert_eq!(decoded.command_id, response.command_id);
    assert_eq!(decoded.server_sequence, response.server_sequence);
    assert_eq!(decoded.status, response.status as u32);
    assert_eq!(decoded.payload, response.payload);
    Ok(())
}
