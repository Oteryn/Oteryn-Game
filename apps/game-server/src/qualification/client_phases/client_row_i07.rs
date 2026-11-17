//! I07 client replay capture: real quick-loot filter, Gold Pouch destination and loot-all replies.
use super::*;
use oteryn_protocol_oteryn::quick_loot::{
    QuickLootAction, QuickLootCategory, QuickLootFilterMode, QuickLootRecipient, QuickLootRequest,
    decode_quick_loot_result, encode_quick_loot_request,
};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = verdict::indexed_timeout(Duration::from_secs(200), verdict::observe(journey(clients))).await;
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

fn quick_loot_matches(confirmed_filters: usize, real_pouch: bool, corpse: bool, looted: usize) -> bool {
    confirmed_filters == 2 && real_pouch && corpse && looted > 0
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    use oteryn_protocol_oteryn::quick_loot::{CAPABILITY_QUICK_LOOT_V1, COMMAND_TYPE_QUICK_LOOT};
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(bytes, &pin, WorldId::decode(&clients.world)?, ChannelId::decode(&clients.channel)?)?;
    let mut client = connect_world(clients, 0x07).await?;
    let definitions = provisioning::item_index()?;
    let coin = provisioning::item_reference(&definitions, 3031)?;
    let skipped = provisioning::item_reference(&definitions, 3350)?;
    let pouch_definition = provisioning::item_reference(&definitions, 23721)?;

    if !client.inventory().is_some_and(|inventory| inventory.entries.iter().any(|entry| entry.item_definition_ref == pouch_definition)) {
        use oteryn_protocol_oteryn::depot_search as wire;
        let path = route(world, tile(client.world_spatial().actor_position)?, |at| near_locker(world, at))
            .ok_or("I07 cannot reach a depot for the fixture Gold Pouch")?;
        walk(&mut client, world, &path).await?;
        let payload = wire::encode_request(wire::Request::Retrieve { client_id: 23721, all: true })
            .map_err(|error| format!("I07 pouch retrieve encoding: {error:?}"))?;
        let reply = client.session_mut().gated_command(wire::CAPABILITY_DEPOT_SEARCH_V1,
            wire::COMMAND_TYPE_DEPOT_SEARCH, &payload).await?;
        let decoded = wire::decode_reply(&reply.payload).map_err(|error| format!("I07 pouch retrieve decoding: {error:?}"))?;
        if reply.status != CommandStatus::Accepted || decoded.outcome != wire::Outcome::Retrieved || decoded.retrieved != 1 {
            return Err(format!("I07 needs a legitimately held Gold Pouch: depot retrieval {decoded:?}").into());
        }
    }

    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let character = clients.characters[1];
    let pouch_id: Vec<u8> = sqlx::query_scalar(
        "SELECT decode(replace(i.item_instance_id::text,'-',''),'hex') FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id,world_id) WHERE e.character_id=encode($1,'hex')::uuid AND e.world_id=encode($2,'hex')::uuid AND i.definition_production_key='oteryn:item.tibia.i23721' AND i.lifecycle=1 ORDER BY e.placement_ordinal LIMIT 1",
    ).bind(character.as_slice()).bind(clients.world.as_slice()).fetch_one(&mut database).await?;
    let pouch: [u8; 16] = pouch_id.try_into().map_err(|_| "I07 invalid durable Gold Pouch identity")?;

    let mut skip = request(QuickLootAction::FilterAdd);
    skip.filter_mode = Some(QuickLootFilterMode::Skip);
    skip.filter_definition_ref = Some(skipped.get());
    let mut accept = request(QuickLootAction::FilterAdd);
    accept.filter_mode = Some(QuickLootFilterMode::Accept);
    accept.filter_definition_ref = Some(coin.get());
    let mut assign = request(QuickLootAction::Assign);
    assign.category = Some(QuickLootCategory::Gold);
    assign.recipient = Some(QuickLootRecipient::GoldPouch);
    assign.recipient_id = Some(pouch);
    assign.fallback_to_main = false;
    let mut payloads = Vec::new();
    for request in [skip, accept, assign] {
        let payload = encode_quick_loot_request(&request).map_err(|error| format!("I07 encode: {error:?}"))?;
        let reply = client.session_mut().gated_command(CAPABILITY_QUICK_LOOT_V1, COMMAND_TYPE_QUICK_LOOT, &payload).await?;
        if reply.status != CommandStatus::Accepted
            || !decode_quick_loot_result(&reply.payload).map_err(|error| format!("I07 decode: {error:?}"))?.items.is_empty()
        {
            return Err(format!("I07 {:?} did not commit: {reply:?}", request.action).into());
        }
        payloads.push(reply.payload);
    }
    let filters: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quick_loot_filters WHERE character_id=encode($1,'hex')::uuid AND ((accepted AND item_production_key='oteryn:item.tibia.i3031') OR (NOT accepted AND item_production_key='oteryn:item.tibia.i3350'))",
    ).bind(character.as_slice()).fetch_one(&mut database).await?;

    travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await?;
    let (corpse, _) = client_row_i06::kill_for_corpse(&mut client).await?;
    let handle = match corpse.detail {
        oteryn_dev_client::EntityDetail::Object { item_handle: Some(handle), .. }
        | oteryn_dev_client::EntityDetail::ObjectWithAppearance { item_handle: Some(handle), .. } => handle,
        _ => return Err("I07 attacked creature corpse has no item handle".into()),
    };
    let opened = client.use_item(handle).await?;
    if opened.status != CommandStatus::Accepted || opened.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed {
        return Err(format!("I07 corpse open refused: {opened:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let coins = loop {
        if let Some(container) = client.open_container().filter(|container| container.container_handle == Some(handle)) {
            break container.entries.iter().filter(|entry| entry.item_definition_ref == coin)
                .map(|entry| i64::from(entry.count.get())).sum::<i64>();
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("I07 selected corpse did not publish its inventory".into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    };
    if coins == 0 {
        return Err("I07 selected corpse carried no gold to route to the pouch".into());
    }
    let before = pouch_gold(&mut database, pouch).await?;
    let payload = encode_quick_loot_request(&request(QuickLootAction::LootAll))
        .map_err(|error| format!("I07 loot all encode: {error:?}"))?;
    let reply = client.session_mut().gated_command(CAPABILITY_QUICK_LOOT_V1, COMMAND_TYPE_QUICK_LOOT, &payload).await?;
    let result = decode_quick_loot_result(&reply.payload).map_err(|error| format!("I07 loot all decode: {error:?}"))?;
    let accepted = result.items.iter().filter(|item| item.accepted).count();
    let after = pouch_gold(&mut database, pouch).await?;
    if reply.status != CommandStatus::Accepted
        || !quick_loot_matches(usize::try_from(filters)?, before.checked_add(coins) == Some(after), true, accepted)
    {
        return Err(format!("I07 quick-loot evidence mismatch: filters={filters}, accepted={accepted}, pouch={before}->{after}, corpse={coins}").into());
    }
    payloads.push(reply.payload);
    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let path = Path::new(&path);
        for (index, payload) in payloads.iter().enumerate() {
            std::fs::write(path.join(format!("i07-{}.bin", index + 1)), payload)?;
        }
        std::fs::write(path.join("i07-expected.txt"),
            format!("skip={} accept={} confirmed={filters} results={}\n", skipped, coin, result.items.len()))?;
    }
    Ok(())
}

async fn pouch_gold(database: &mut sqlx::PgConnection, pouch: [u8; 16]) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT COALESCE(sum(i.quantity),0)::bigint FROM game_item_nested_entries e JOIN game_item_instances i USING(item_instance_id,world_id) WHERE e.parent_item_instance_id=encode($1,'hex')::uuid AND i.definition_production_key='oteryn:item.tibia.i3031' AND i.lifecycle=1",
    ).bind(pouch.as_slice()).fetch_one(database).await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quick_loot_requires_real_corpse_pouch_filters_and_custody() {
        assert!(!quick_loot_matches(2, true, true, 0));
        assert!(!quick_loot_matches(2, false, true, 1));
        assert!(!quick_loot_matches(2, true, false, 1));
        assert!(!quick_loot_matches(1, true, true, 1));
        assert!(quick_loot_matches(2, true, true, 1));
    }
}
