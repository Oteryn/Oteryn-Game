//! I07 client replay capture: real quick-loot filter, Gold Pouch destination and loot-all replies.
use super::*;
use oteryn_protocol_oteryn::quick_loot::{
    QuickLootAction, QuickLootCategory, QuickLootFilterMode, QuickLootRecipient, QuickLootRequest,
    decode_quick_loot_result, encode_quick_loot_request,
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

fn write_capture(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write as _;
    std::fs::File::create_new(path)?.write_all(bytes)
}

async fn require_filters(
    database: &mut sqlx::PgConnection,
    character: &[u8; 16],
    expected_mode: bool,
    expected: &[(bool, &str)],
) -> TestResult {
    let mode: Option<bool> = sqlx::query_scalar(
        "SELECT accept_only FROM game_character_quick_loot_settings
         WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character.as_slice())
    .fetch_optional(&mut *database)
    .await?;
    let filters: Vec<(bool, String)> = sqlx::query_as(
        "SELECT accepted, item_production_key FROM game_character_quick_loot_filters
         WHERE character_id=encode($1,'hex')::uuid ORDER BY accepted, item_production_key LIMIT 4",
    )
    .bind(character.as_slice())
    .fetch_all(database)
    .await?;
    if mode.unwrap_or(false) != expected_mode
        || !filters
            .iter()
            .map(|(accept, key)| (*accept, key.as_str()))
            .eq(expected.iter().copied())
    {
        return Err(format!(
            "I07 durable filters expected mode={expected_mode} entries={expected:?}; observed mode={mode:?} entries={filters:?}"
        ).into());
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Placement {
    item: Vec<u8>,
    key: String,
    quantity: i64,
    slot: i64,
}

async fn placements(
    database: &mut sqlx::PgConnection,
    parent: &[u8],
) -> TestResult<Vec<Placement>> {
    let rows: Vec<(Vec<u8>, String, i64, i64)> = sqlx::query_as(
        "SELECT uuid_send(i.item_instance_id), i.definition_production_key, i.quantity, e.placement_ordinal::bigint
         FROM (
             SELECT item_instance_id, placement_ordinal FROM game_item_corpse_container_entries
                WHERE parent_item_instance_id=encode($1,'hex')::uuid
             UNION ALL
             SELECT item_instance_id, placement_ordinal FROM game_item_container_entries
                WHERE parent_item_instance_id=encode($1,'hex')::uuid
             UNION ALL
             SELECT item_instance_id, placement_ordinal FROM game_item_nested_entries
                WHERE parent_item_instance_id=encode($1,'hex')::uuid
         ) e JOIN game_item_instances i USING (item_instance_id)
         WHERE i.lifecycle=1 ORDER BY e.placement_ordinal LIMIT 501",
    )
    .bind(parent)
    .fetch_all(database)
    .await?;
    if rows.len() > 500 {
        return Err("I07 placement snapshot exceeded its bound".into());
    }
    Ok(rows
        .into_iter()
        .map(|(item, key, quantity, slot)| Placement {
            item,
            key,
            quantity,
            slot,
        })
        .collect())
}

async fn require_placements(
    database: &mut sqlx::PgConnection,
    parent: &[u8],
    expected: &[Placement],
) -> TestResult {
    let actual = placements(database, parent).await?;
    if actual != expected {
        return Err(
            format!("I07 container delta: expected={expected:?} observed={actual:?}").into(),
        );
    }
    Ok(())
}

async fn require_routes(
    database: &mut sqlx::PgConnection,
    character: &[u8],
    pouch: &[u8],
) -> TestResult {
    let route: bool = sqlx::query_scalar(
        "SELECT EXISTS (
          SELECT 1 FROM game_character_quick_loot_settings s
          JOIN game_character_quick_loot_categories c USING (character_id)
          WHERE s.character_id=encode($1,'hex')::uuid AND s.accept_only AND s.fallback_to_main
            AND s.unassigned_gold_pouch AND s.unassigned_container_id=encode($2,'hex')::uuid
            AND c.gold_pouch AND c.category=0 AND c.container_id=s.unassigned_container_id)",
    )
    .bind(character)
    .bind(pouch)
    .fetch_one(database)
    .await?;
    if !route {
        return Err("I07 durable Gold and unassigned pouch routing differs".into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    use sqlx::Connection as _;
    const FILTERS: [(bool, &str); 2] = [
        (false, "oteryn:item.tibia.i3031"),
        (true, "oteryn:item.tibia.i3350"),
    ];
    let capture_dir = std::env::var_os("E2E_ALL_CAPTURE_DIR")
        .ok_or("I07 requires a fresh E2E_ALL_CAPTURE_DIR")?;
    let capture_dir = Path::new(&capture_dir);
    let index = provisioning::item_index()?;
    let skip_reference = provisioning::item_reference(&index, 3031)?.get();
    let accept_reference = provisioning::item_reference(&index, 3350)?.get();
    let dedicated = character_prep::dedicated_named_character(
        clients,
        "NODE_BOOT_I07_ACCOUNT",
        Some("Quick Loot Tester"),
    )
    .await?;
    let (accounts, characters) = dedicated.replace_slot(clients, 1)?;
    let scoped = SeamClients {
        accounts: &accounts,
        characters: &characters,
        ..*clients
    };
    let clients = &scoped;
    let mut client = connect_world(clients, 0x07).await?;
    if tile(client.world_spatial().actor_position)? != TOWN {
        return Err("I07 fixture requires admission at the seeded corpse tile".into());
    }
    request_fixture_control(
        uuid_text(client.join_snapshot().game_session_id.as_bytes()),
        "--profile",
        "i07-loot".into(),
    )
    .await?;
    let character = &clients.characters[1];
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    require_filters(&mut database, character, false, &[]).await?;
    let settings_exist: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM game_character_quick_loot_settings
         WHERE character_id=encode($1,'hex')::uuid)",
    )
    .bind(character.as_slice())
    .fetch_one(&mut database)
    .await?;
    if settings_exist {
        return Err("I07 dedicated character already has quick-loot settings".into());
    }
    let mut skip = request(QuickLootAction::FilterAdd);
    skip.filter_mode = Some(QuickLootFilterMode::Skip);
    skip.filter_definition_ref = Some(skip_reference);
    let mut accept = request(QuickLootAction::FilterAdd);
    accept.filter_mode = Some(QuickLootFilterMode::Accept);
    accept.filter_definition_ref = Some(accept_reference);
    let pouch_ids: Vec<Vec<u8>> = sqlx::query_scalar(
        "SELECT uuid_send(i.item_instance_id) FROM game_item_character_inbox_locations l
         JOIN game_item_instances i USING (item_instance_id, world_id)
         WHERE l.character_id=encode($1,'hex')::uuid AND i.lifecycle=1
           AND i.definition_production_key='oteryn:item.tibia.i23721' LIMIT 2",
    )
    .bind(character.as_slice())
    .fetch_all(&mut database)
    .await?;
    let [pouch_id] = pouch_ids.as_slice() else {
        return Err(format!(
            "I07 needs exactly one real Inbox Gold Pouch; found {}",
            pouch_ids.len()
        )
        .into());
    };
    let pouch_id: [u8; 16] = pouch_id.as_slice().try_into()?;
    let corpse: Vec<u8> = sqlx::query_scalar(
        "SELECT uuid_send(item_instance_id) FROM game_item_mint_receipts
         WHERE death_world_id=encode($1,'hex')::uuid AND death_channel_id=encode($2,'hex')::uuid
           AND death_scope_ownership_generation=$3::text::numeric
           AND corpse_top_damage_character_id=encode($4,'hex')::uuid
           AND loot_table_production_key='oteryn:qualification.i07.loot'
           AND loot_purpose_key='CORPSE_MATERIALIZATION' AND draw_ordinal=0",
    )
    .bind(clients.world.as_slice())
    .bind(clients.channel.as_slice())
    .bind(clients.scope_generation.to_string())
    .bind(character.as_slice())
    .fetch_one(&mut database)
    .await?;
    let backpack: Vec<u8> = sqlx::query_scalar(
        "SELECT uuid_send(item_instance_id) FROM game_item_container_slots
         WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character.as_slice())
    .fetch_one(&mut database)
    .await?;
    let before = placements(&mut database, &corpse).await?;
    if !before
        .iter()
        .map(|row| (row.key.as_str(), row.quantity, row.slot))
        .eq([
            ("oteryn:item.tibia.i3031", 7, 1),
            ("oteryn:item.tibia.i3350", 1, 2),
            ("oteryn:item.tibia.i3577", 1, 3),
        ])
    {
        return Err(format!("I07 seeded corpse inputs differ: {before:?}").into());
    }
    let [coin, armor, filtered] = before.as_slice() else {
        return Err("I07 needs exactly three seeded corpse items".into());
    };
    let mut expected_backpack = placements(&mut database, &backpack).await?;
    require_placements(&mut database, &pouch_id, &[]).await?;
    let mut bag_armor = armor.clone();
    bag_armor.slot = expected_backpack.last().map_or(1, |item| item.slot + 1);
    expected_backpack.push(bag_armor);
    let expected_pouch = [coin.clone()];
    let remaining = [filtered.clone()];
    let expected_filters = [FILTERS[0], (true, "oteryn:item.tibia.i3031"), FILTERS[1]];
    let original_session = client.join_snapshot().game_session_id;
    let session = uuid_text(original_session.as_bytes());
    let mut pouch = request(QuickLootAction::Assign);
    pouch.category = Some(QuickLootCategory::Gold);
    pouch.recipient = Some(QuickLootRecipient::GoldPouch);
    pouch.recipient_id = Some(pouch_id);
    pouch.fallback_to_main = true;
    let requests = [skip, accept, pouch, request(QuickLootAction::LootAll)];
    let mut item_count = 0;
    let mut previous = (0, 0);
    for (index, request) in requests.into_iter().enumerate() {
        let payload = encode_quick_loot_request(&request).map_err(|error| format!("{error:?}"))?;
        let reply = client
            .session_mut()
            .gated_command(98, 114, &payload)
            .await?;
        if reply.status != CommandStatus::Accepted {
            return Err(format!(
                "I07 probe {} {:?} refused: {:?}",
                index + 1,
                request.action,
                reply.status
            )
            .into());
        }
        if client.join_snapshot().game_session_id != original_session
            || reply.command_id <= previous.0
            || reply.server_sequence <= previous.1
        {
            return Err("I07 probe reply does not advance the same live session".into());
        }
        previous = (reply.command_id, reply.server_sequence);
        if index == 0 {
            require_filters(&mut database, character, false, &FILTERS[..1]).await?;
        } else if index == 1 {
            require_filters(&mut database, character, true, &FILTERS).await?;
        }
        if index == 1 {
            let coin_filter = QuickLootRequest {
                filter_definition_ref: Some(skip_reference),
                ..accept
            };
            let unassigned_pouch = QuickLootRequest {
                category: None,
                ..pouch
            };
            for extra in [coin_filter, unassigned_pouch] {
                let payload =
                    encode_quick_loot_request(&extra).map_err(|error| format!("{error:?}"))?;
                let extra_reply = client
                    .session_mut()
                    .gated_command(98, 114, &payload)
                    .await?;
                if extra_reply.status != CommandStatus::Accepted {
                    return Err(format!(
                        "I07 supporting {:?} refused: {:?}",
                        extra.action, extra_reply.status
                    )
                    .into());
                }
            }
            require_filters(&mut database, character, true, &expected_filters).await?;
        }
        if index == 2 {
            require_routes(&mut database, character, &pouch_id).await?;
        }
        if request.action == QuickLootAction::LootAll {
            let result = decode_quick_loot_result(&reply.payload)
                .map_err(|error| format!("quick-loot reply: {error:?}"))?
                .items;
            let moved: std::collections::BTreeMap<_, _> = result
                .iter()
                .map(|item| (item.item.as_slice(), item.accepted))
                .collect();
            let expected = [
                (coin.item.as_slice(), true),
                (armor.item.as_slice(), true),
                (filtered.item.as_slice(), false),
            ]
            .into_iter()
            .collect();
            if result.len() != 3 || moved != expected {
                return Err(format!("I07 LootAll item outcomes differ: {result:?}").into());
            }
            item_count = result.len();
            require_placements(&mut database, &corpse, &remaining).await?;
            require_placements(&mut database, &pouch_id, &expected_pouch).await?;
            require_placements(&mut database, &backpack, &expected_backpack).await?;
            evidence("I07_LOOT_CUSTODY pouch=3031x7@1 backpack=3350x1 filtered=3577x1@3");
        }
        let key = index + 1;
        write_capture(
            &capture_dir.join(format!("i07-{key}-request.bin")),
            &payload,
        )?;
        let envelope = serde_json::json!({
            "game_session_id": session,
            "command_id": reply.command_id,
            "server_sequence": reply.server_sequence,
            "status": reply.status as u32,
            "payload": reply.payload,
        });
        write_capture(
            &capture_dir.join(format!("i07-{key}.json")),
            &serde_json::to_vec(&envelope)?,
        )?;
    }
    let logout = client.session_mut().logout().await?;
    if logout.status != CommandStatus::Accepted
        || logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err(format!("I07 first logout refused: {logout:?}").into());
    }
    drop(client);
    let mut fresh = connect_world(clients, 0x07).await?;
    if fresh.join_snapshot().game_session_id == original_session {
        return Err("I07 relog reused its original session".into());
    }
    require_filters(&mut database, character, true, &expected_filters).await?;
    require_routes(&mut database, character, &pouch_id).await?;
    require_placements(&mut database, &pouch_id, &expected_pouch).await?;
    require_placements(&mut database, &backpack, &expected_backpack).await?;
    let logout = fresh.session_mut().logout().await?;
    if logout.status != CommandStatus::Accepted
        || logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err(format!("I07 post-readback logout refused: {logout:?}").into());
    }
    evidence(
        "I07_FILTER_CUSTODY prestate=empty skip=3031 accept=3350 mode=accept fresh_session=true",
    );
    let pouch_hex: String = pouch_id.iter().map(|byte| format!("{byte:02x}")).collect();
    write_capture(
        &capture_dir.join("i07-expected.txt"),
        format!(
            "session={session}\npouch={pouch_hex}\nskip={skip_reference} accept={accept_reference} confirmed=3 destinations=1 results={item_count}\n"
        )
        .as_bytes(),
    )?;
    evidence(&format!("I07_LIVE_CAPTURE session={session}"));
    Ok(())
}
