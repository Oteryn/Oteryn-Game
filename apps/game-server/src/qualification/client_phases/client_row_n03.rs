//! N03 client replay capture: deposit two carried items at an authored depot locker, then capture the
//! real Depot Search holdings list (every page up to the item) and the RETRIEVE(one) reply. Client
//! owner: `activate_shared` in `apps/client/src/features/0153-depot-search/native.rs`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::{CommandStatus, depot, depot_search as wire};
use std::num::NonZeroU32;

// Authored depot in region-z07-x126-y125.b3 and the walkable tile directly north.
const DEPOT_LOCKER: TilePos = TilePos {
    x: 32354,
    y: 32231,
    floor: -7,
};
const DEPOT_APPROACH: TilePos = TilePos {
    x: 32354,
    y: 32230,
    floor: -7,
};
const DEPOT_TOWN: u16 = 8;
const DEPOT_BOX: u8 = 1;
const ITEM_ID: u32 = 3276;
const ITEM_KEY: &str = "oteryn:item.tibia.i3276";
const DEPOSIT_UNITS: u32 = 2;

fn capture(root: &Path, name: &str, bytes: &[u8]) -> TestResult {
    use std::io::Write as _;

    std::fs::File::create_new(root.join(name))
        .and_then(|mut file| file.write_all(bytes))
        .map_err(|error| format!("N03_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}").into())
}

async fn search_command(
    client: &mut DevClientSession,
    request: wire::Request,
) -> TestResult<(wire::Reply, Vec<u8>)> {
    let payload = wire::encode_request(request).map_err(|error| format!("encode {error:?}"))?;
    let response = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_DEPOT_SEARCH_V1,
            wire::COMMAND_TYPE_DEPOT_SEARCH,
            &payload,
        )
        .await?;
    if response.status != CommandStatus::Accepted {
        return Err(format!("depot search status {:?}", response.status).into());
    }
    let reply = wire::decode_reply(&response.payload).map_err(|error| format!("decode {error:?}"))?;
    let envelope = capture_envelope::encode(
        client.join_snapshot().game_session_id.as_bytes(),
        response.command_id,
        response.server_sequence,
        response.status as u64,
        &response.payload,
    )?;
    Ok((reply, envelope))
}

async fn depot_intent(
    client: &mut DevClientSession,
    intent: depot::DepotIntent,
) -> TestResult<depot::DepotReply> {
    let payload =
        depot::encode_depot_intent(intent).map_err(|error| format!("depot encode {error:?}"))?;
    let response = client
        .session_mut()
        .gated_command(depot::CAPABILITY_DEPOT_V1, depot::COMMAND_TYPE_DEPOT_INTENT, &payload)
        .await?;
    if response.status != CommandStatus::Accepted {
        return Err(format!("depot status {:?}", response.status).into());
    }
    depot::decode_depot_reply(&response.payload)
        .map_err(|error| format!("depot decode {error:?}").into())
}

fn verify_box(
    reply: &depot::DepotReply,
    expected: depot::DepotOutcome,
    definition: u32,
    total: u32,
) -> TestResult {
    let expected_entries = reply.entries.len() as u32 == total
        && reply
            .entries
            .iter()
            .all(|entry| entry.definition == definition && entry.count == 1);
    if reply.outcome != expected
        || reply.town != Some(DEPOT_TOWN)
        || reply.box_number != DEPOT_BOX
        || reply.page != 0
        || reply.total != total
        || !expected_entries
    {
        return Err(format!("N03 unexpected depot box: {reply:?}").into());
    }
    Ok(())
}

#[test]
fn n03_box_rejects_wrong_town_and_custody() -> TestResult {
    let mut reply = depot::DepotReply {
        outcome: depot::DepotOutcome::Opened,
        town: Some(DEPOT_TOWN),
        box_number: DEPOT_BOX,
        page: 0,
        entries: Vec::new(),
        total: 0,
    };
    verify_box(&reply, depot::DepotOutcome::Opened, 42, 0)?;
    reply.town = Some(2);
    assert!(verify_box(&reply, depot::DepotOutcome::Opened, 42, 0).is_err());
    reply.town = Some(DEPOT_TOWN);
    reply.outcome = depot::DepotOutcome::Moved;
    assert!(verify_box(&reply, depot::DepotOutcome::Moved, 42, 1).is_err());
    reply.total = 1;
    reply.entries.push(depot::DepotEntry {
        ordinal: 0,
        definition: 42,
        count: 1,
    });
    verify_box(&reply, depot::DepotOutcome::Moved, 42, 1)?;
    reply.entries[0].definition = 43;
    assert!(verify_box(&reply, depot::DepotOutcome::Moved, 42, 1).is_err());
    reply.entries[0].definition = 42;
    reply.entries[0].count = 2;
    assert!(verify_box(&reply, depot::DepotOutcome::Moved, 42, 1).is_err());
    reply.entries[0].count = 1;
    reply.box_number = 2;
    assert!(verify_box(&reply, depot::DepotOutcome::Moved, 42, 1).is_err());
    reply.box_number = DEPOT_BOX;
    reply.page = 1;
    assert!(verify_box(&reply, depot::DepotOutcome::Moved, 42, 1).is_err());
    Ok(())
}

fn carried_count(client: &DevClientSession, definition: NonZeroU32) -> TestResult<u64> {
    backpack_count(
        client.inventory().ok_or("N03 inventory snapshot missing")?,
        definition,
    )
}

fn backpack_count(
    inventory: &oteryn_protocol_oteryn::item_view::CharacterInventory,
    definition: NonZeroU32,
) -> TestResult<u64> {
    if inventory
        .main_backpack
        .iter()
        .chain(inventory.equipment.iter().map(|entry| &entry.item))
        .any(|item| item.item_definition_ref == definition)
    {
        return Err("N03 fixture item appeared outside the backpack entries".into());
    }
    Ok(inventory
        .entries
        .iter()
        .filter(|entry| entry.item_definition_ref == definition)
        .map(|entry| u64::from(entry.count.get()))
        .sum())
}

#[test]
fn n03_backpack_rejects_equipped_fixture_item() -> TestResult {
    use oteryn_protocol_oteryn::item_view::{
        CharacterInventory, EquipmentSlot, EquippedItem, ItemEntry, ItemHandle,
    };
    let item = ItemEntry {
        appearance_id: 0,
        handle: ItemHandle::new(1).ok_or("test item handle")?,
        item_definition_ref: NonZeroU32::MIN,
        count: NonZeroU32::MIN,
        sub_type: 0,
    };
    let mut inventory = CharacterInventory {
        entries: vec![item],
        ..Default::default()
    };
    assert_eq!(backpack_count(&inventory, item.item_definition_ref)?, 1);
    inventory.entries.clear();
    inventory.equipment.push(EquippedItem {
        slot: EquipmentSlot::Head,
        item,
    });
    assert!(backpack_count(&inventory, item.item_definition_ref).is_err());
    Ok(())
}

fn verify_approach(position: TilePos) -> TestResult {
    if position != DEPOT_APPROACH {
        return Err(format!(
            "N03 expected pinned depot approach {DEPOT_APPROACH:?}, got {position:?}"
        )
        .into());
    }
    Ok(())
}

#[test]
fn n03_approach_rejects_another_tile_in_locker_range() -> TestResult {
    verify_approach(DEPOT_APPROACH)?;
    assert!(
        verify_approach(TilePos {
            x: DEPOT_APPROACH.x + 1,
            ..DEPOT_APPROACH
        })
        .is_err()
    );
    Ok(())
}

async fn wait_carried(
    client: &mut DevClientSession,
    definition: NonZeroU32,
    expected: u64,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        let actual = carried_count(client, definition)?;
        if actual == expected {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("N03 inventory delta expected={expected} actual={actual}").into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
}

fn verify_holdings(reply: &wire::Reply, expected: u32) -> TestResult {
    let matches = match (expected, reply.results.as_slice()) {
        (0, []) => reply.outcome == wire::Outcome::NotFound,
        (1, [row]) => {
            reply.outcome == wire::Outcome::Found
                && (row.client_id, row.depot, row.stash, row.inbox) == (ITEM_ID, 1, 0, 0)
                && row.tiers == [wire::TierCount { tier: 0, count: 1 }]
        }
        _ => false,
    };
    if !matches || reply.retrieved != 0 {
        return Err(format!(
            "N03 expected depot holdings={expected} item={ITEM_ID} reply={reply:?}"
        )
        .into());
    }
    Ok(())
}

fn verify_page(
    reply: &wire::Reply,
    outcome: wire::Outcome,
    expected: &[wire::SearchResult],
) -> TestResult {
    if reply.outcome != outcome || reply.retrieved != 0 || reply.results != expected {
        return Err(format!("N03 unexpected holdings page: {reply:?}").into());
    }
    Ok(())
}

fn depot_row(client_id: u32, count: u32) -> wire::SearchResult {
    wire::SearchResult {
        client_id,
        depot: count,
        stash: 0,
        inbox: 0,
        tiers: vec![wire::TierCount { tier: 0, count }],
    }
}

#[test]
fn n03_pages_reject_extra_rows_overlap_and_wrong_terminal_status() -> TestResult {
    let target = depot_row(ITEM_ID, DEPOSIT_UNITS);
    let mut reply = wire::Reply {
        outcome: wire::Outcome::Listed,
        retrieved: 0,
        results: vec![target.clone()],
    };
    verify_page(&reply, wire::Outcome::Listed, std::slice::from_ref(&target))?;
    reply.results.insert(0, depot_row(ITEM_ID - 1, 1));
    assert!(verify_page(&reply, wire::Outcome::Listed, std::slice::from_ref(&target)).is_err());
    reply.results.remove(0);
    reply.outcome = wire::Outcome::More;
    assert!(verify_page(&reply, wire::Outcome::Listed, &[target]).is_err());
    Ok(())
}

async fn seed_pagination_inputs(
    db: &mut sqlx::PgConnection,
    clients: &SeamClients<'_>,
    index: &crate::content::item_ref::ItemDefinitionIndex,
) -> TestResult<Vec<wire::SearchResult>> {
    let definitions: Vec<_> = (100..ITEM_ID)
        .filter_map(|id| {
            let key = format!("oteryn:item.tibia.i{id}");
            let revision = index.revision(&key)?.to_owned();
            Some((id, key, revision))
        })
        .take(wire::MAX_RESULTS)
        .collect();
    if definitions.len() != wire::MAX_RESULTS {
        return Err("N03 requires a full page of distinct lower item IDs".into());
    }

    // Match the depot storage fixture: seed native identities, then place them
    // with the ordinary depot location guards and counter triggers enabled.
    let mut tx = db.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    let keys: Vec<_> = definitions.iter().map(|(_, key, _)| key.as_str()).collect();
    let revisions: Vec<_> = definitions
        .iter()
        .map(|(_, _, revision)| revision.as_str())
        .collect();
    let items: Vec<String> = sqlx::query_scalar(
        "INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,
         definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
         SELECT game_character_uuid_v7(),encode($1,'hex')::uuid,'Item',key,revision,1,1,
                game_character_uuid_v7() FROM unnest($2::text[],$3::text[]) AS input(key,revision)
         RETURNING item_instance_id::text",
    )
    .bind(clients.world.as_slice())
    .bind(keys)
    .bind(revisions)
    .fetch_all(&mut *tx)
    .await?;
    sqlx::query("SET LOCAL session_replication_role = origin")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO game_item_character_depot_locations
         (item_instance_id,world_id,character_id,town_id,box_number)
         SELECT id::uuid,encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,2
         FROM unnest($4::text[]) AS input(id)",
    )
    .bind(clients.world.as_slice())
    .bind(clients.characters[1].as_slice())
    .bind(i32::from(DEPOT_TOWN))
    .bind(items)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(definitions
        .into_iter()
        .map(|(id, _, _)| depot_row(id, 1))
        .collect())
}

async fn assert_holdings(client: &mut DevClientSession, expected: u32) -> TestResult {
    let (reply, _) = search_command(client, wire::Request::Search { client_id: ITEM_ID }).await?;
    verify_holdings(&reply, expected)
}

#[test]
fn n03_retrieve_one_rejects_empty_and_unchanged_holdings() -> TestResult {
    let mut reply = wire::Reply {
        outcome: wire::Outcome::NotFound,
        retrieved: 0,
        results: vec![],
    };
    verify_holdings(&reply, 0)?;
    assert!(verify_holdings(&reply, 1).is_err());
    reply.outcome = wire::Outcome::Found;
    reply.results.push(wire::SearchResult {
        client_id: ITEM_ID,
        depot: 2,
        stash: 0,
        inbox: 0,
        tiers: vec![],
    });
    assert!(verify_holdings(&reply, 1).is_err());
    reply.results[0].depot = 1;
    reply.results[0].tiers = vec![wire::TierCount { tier: 0, count: 1 }];
    verify_holdings(&reply, 1)
}

/// The wire inventory handles are scoped to an admission. Read the durable UUIDs
/// in addition to the wire observations, using the same character and world.
async fn item_custody(
    db: &mut sqlx::PgConnection,
    clients: &SeamClients<'_>,
) -> TestResult<Vec<(bool, String)>> {
    let rows: Vec<(bool, String, i64)> = sqlx::query_as(
        "SELECT false AS in_depot, i.item_instance_id::text AS id, i.quantity
           FROM game_item_container_entries c JOIN game_item_instances i USING(item_instance_id, world_id)
          WHERE c.character_id=encode($1,'hex')::uuid AND c.world_id=encode($2,'hex')::uuid
            AND i.definition_production_key=$3 AND i.lifecycle=1
         UNION ALL
         SELECT true, i.item_instance_id::text, i.quantity
           FROM game_item_character_depot_locations d JOIN game_item_instances i USING(item_instance_id, world_id)
          WHERE d.character_id=encode($1,'hex')::uuid AND d.world_id=encode($2,'hex')::uuid
            AND i.definition_production_key=$3 AND i.lifecycle=1
            AND d.town_id=$4 AND d.box_number=$5
          ORDER BY in_depot, id LIMIT 3",
    )
    .bind(clients.characters[1].as_slice())
    .bind(clients.world.as_slice())
    .bind(ITEM_KEY)
    .bind(i32::from(DEPOT_TOWN))
    .bind(i16::from(DEPOT_BOX))
    .fetch_all(db)
    .await?;
    rows.into_iter()
        .map(|(in_depot, id, quantity)| {
            if quantity != 1 {
                return Err(format!("N03 durable item {id} has quantity={quantity}").into());
            }
            Ok((in_depot, id))
        })
        .collect()
}

fn verify_custody(actual: &[(bool, String)], expected: &[(bool, String)]) -> TestResult {
    if actual != expected {
        return Err(
            format!("N03 durable item custody expected={expected:?} actual={actual:?}").into(),
        );
    }
    Ok(())
}

#[test]
fn n03_custody_rejects_item_substitution_and_retrieve_all() -> TestResult {
    let expected = [
        (false, "original-first".to_owned()),
        (true, "original-second".to_owned()),
    ];
    verify_custody(&expected, &expected)?;
    assert!(
        verify_custody(
            &[(false, "replacement".to_owned()), expected[1].clone()],
            &expected
        )
        .is_err()
    );
    assert!(
        verify_custody(
            &[expected[0].clone(), (false, expected[1].1.clone())],
            &expected
        )
        .is_err()
    );
    Ok(())
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let outcome = client.session_mut().logout().await?;
    if outcome.status != CommandStatus::Accepted
        || outcome.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err(format!("N03 logout refused: {outcome:?}").into());
    }
    Ok(())
}

async fn walk_to_depot(
    client: &mut DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
) -> TestResult {
    let mut step_wait_ms = 0;
    for &direction in path {
        if step_wait_ms != 0 {
            client.service_liveness(Duration::from_millis(u64::from(step_wait_ms))).await?;
        }
        let start = tile(client.world_spatial().actor_position)?;
        let landing = destination(world, start, direction).ok_or("N03 invalid depot route")?;
        let mut outcome = client.step(direction).await?;
        if outcome.disposition == StepDisposition::Blocked {
            let occupant = client.world_entities().and_then(|entities| {
                entities.iter().find(|entity| {
                    entity.kind == oteryn_dev_client::EntityKind::Creature
                        && tile(entity.position).ok() == Some(landing)
                }).map(|entity| entity.entity.identity)
            }).ok_or_else(|| format!("BLOCKER-SERVER N03 blocked without visible creature at {landing:?}: {outcome:?}"))?;
            evidence(&format!("N03_MOVE_BLOCKED occupant={occupant:?} at={landing:?} frame={outcome:?}"));
            let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
            loop {
                client.service_liveness(Duration::from_millis(100)).await?;
                let still_there = client.world_entities().is_some_and(|entities| {
                    entities.iter().any(|entity| entity.entity.identity == occupant
                        && tile(entity.position).ok() == Some(landing))
                });
                if !still_there {
                    evidence(&format!("N03_MOVE_CLEARED occupant={occupant:?} at={landing:?}"));
                    break;
                }
                if tokio::time::Instant::now() >= deadline {
                    return Err(format!("N03 creature {occupant:?} remained at {landing:?}").into());
                }
            }
            outcome = client.step(direction).await?;
        }
        if outcome.disposition != StepDisposition::Moved
            || outcome.world_spatial_delta.is_none()
            || tile(client.world_spatial().actor_position)? != landing
        {
            return Err(format!("N03 step {direction:?} expected={landing:?}: {outcome:?}").into());
        }
        step_wait_ms = outcome.step_wait_ms;
        verdict::Harness::global().progress("walk");
        client.take_events();
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let capture_root = required("E2E_ALL_CAPTURE_DIR")?;
    let capture_root = Path::new(&capture_root);
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_N03_ACCOUNT").await?;
    let (accounts, characters) = dedicated.replace_slot(clients, 1)?;
    let clients = &SeamClients {
        accounts: &accounts,
        characters: &characters,
        ..*clients
    };
    let item_index = provisioning::item_index()?;
    let definition = item_index
        .definition_ref("Item", ITEM_KEY, "definition-r1")
        .ok_or("N03 fixture item definition missing")?;
    let mut client = connect_character(clients, 0xac, 1).await?;
    if carried_count(&client, definition)? != 0 {
        return Err("N03 dedicated character already carries the fixture item".into());
    }
    character_prep::grant_item(&client, ITEM_ID, DEPOSIT_UNITS).await?;
    evidence(&format!(
        "FIXTURE row=N03.client account=q_n03 item={ITEM_ID} count={DEPOSIT_UNITS}"
    ));
    let fixture_session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);
    let mut client = connect_character(clients, 0xae, 1).await?;
    if client.join_snapshot().game_session_id == fixture_session {
        return Err("N03 fixture relog reused the original session".into());
    }
    if carried_count(&client, definition)? != u64::from(DEPOSIT_UNITS) {
        return Err("N03 fixture did not grant exactly two carried units".into());
    }
    if world.facts().depot_at(DEPOT_LOCKER, 2) != Some(DEPOT_TOWN) {
        return Err(format!(
            "N03 authored locker mismatch at {DEPOT_LOCKER:?} expected town={DEPOT_TOWN}"
        )
        .into());
    }
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at == DEPOT_APPROACH
    })
    .ok_or("N03 authored depot approach is unreachable")?;
    walk_to_depot(&mut client, world, &path).await?;
    verify_approach(tile(client.world_spatial().actor_position)?)?;
    assert_holdings(&mut client, 0).await?;
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let original = item_custody(&mut db, clients).await?;
    let [(false, first), (false, second)] = original.as_slice() else {
        return Err(format!("N03 expected two distinct carried item UUIDs: {original:?}").into());
    };
    let browse = depot_intent(
        &mut client,
        depot::DepotIntent {
            town: None,
            box_number: DEPOT_BOX,
            page: 0,
            action: depot::DepotAction::Browse,
        },
    )
    .await?;
    verify_box(&browse, depot::DepotOutcome::Opened, definition.get(), 0)?;
    for deposited in 1..=DEPOSIT_UNITS {
        let item = client
            .inventory()
            .and_then(|inventory| {
                inventory
                    .entries
                    .iter()
                    .find(|entry| entry.item_definition_ref == definition)
            })
            .copied()
            .ok_or("N03 no carried fixture item to deposit")?;
        if item.count.get() != 1 {
            return Err("N03 requires two distinct items, each of quantity one".into());
        }
        let moved = depot_intent(
            &mut client,
            depot::DepotIntent {
                town: Some(DEPOT_TOWN),
                box_number: DEPOT_BOX,
                page: 0,
                action: depot::DepotAction::Deposit(item.handle),
            },
        )
        .await?;
        verify_box(
            &moved,
            depot::DepotOutcome::Moved,
            definition.get(),
            deposited,
        )?;
        wait_carried(
            &mut client,
            definition,
            u64::from(DEPOSIT_UNITS - deposited),
        )
        .await?;
    }
    verify_custody(
        &item_custody(&mut db, clients).await?,
        &[(true, first.clone()), (true, second.clone())],
    )?;
    evidence(&format!(
        "N03_DEPOT_BOX locker={DEPOT_LOCKER:?} approach={DEPOT_APPROACH:?} town={DEPOT_TOWN} box={DEPOT_BOX} before=0 after={DEPOSIT_UNITS}"
    ));

    let client_id = ITEM_ID;
    let page_rows = seed_pagination_inputs(&mut db, clients, &item_index).await?;
    evidence(&format!(
        "FIXTURE row=N03.client depot_page_input_definitions={} box=2",
        page_rows.len()
    ));
    let (first_page, first_envelope) =
        search_command(&mut client, wire::Request::Open { after: 0 }).await?;
    verify_page(&first_page, wire::Outcome::More, &page_rows)?;
    let after = page_rows
        .last()
        .ok_or("N03 missing fixture page cursor")?
        .client_id;
    let (listed, list_envelope) =
        search_command(&mut client, wire::Request::Open { after }).await?;
    let row = depot_row(client_id, DEPOSIT_UNITS);
    verify_page(&listed, wire::Outcome::Listed, std::slice::from_ref(&row))?;
    let (retrieved, retrieved_envelope) = search_command(
        &mut client,
        wire::Request::Retrieve {
            client_id,
            all: false,
        },
    )
    .await?;
    if retrieved.outcome != wire::Outcome::Retrieved || retrieved.retrieved != 1 {
        return Err(format!("retrieve-one reply {retrieved:?}").into());
    }
    let tiers = row
        .tiers
        .iter()
        .map(|tier| format!("{}x{}", tier.tier, tier.count))
        .collect::<Vec<_>>()
        .join(",");
    let original_session = client.join_snapshot().game_session_id;
    let capture_id = uuid_text(original_session.as_bytes());
    let depot_search_selected = client.selected_capabilities().contains(&153);
    if !depot_search_selected {
        return Err("N03 live session did not negotiate depot search".into());
    }
    let expected = format!(
        "session={capture_id}\ndepot_search_selected={depot_search_selected}\nitem={client_id}\npages={}\nlisted={:?}\nrow={}|{}|{}|{tiers}\nretrieve={:?}|{}\n",
        2, listed.outcome, row.depot, row.stash, row.inbox, retrieved.outcome, retrieved.retrieved
    );
    wait_carried(&mut client, definition, 1).await?;
    assert_holdings(&mut client, 1).await?;
    // The retrieval owner selects the first depot candidate in UUID order.
    let expected_custody = [(false, first.clone()), (true, second.clone())];
    verify_custody(&item_custody(&mut db, clients).await?, &expected_custody)?;
    verify_approach(tile(client.world_spatial().actor_position)?)?;
    logout(&mut client).await?;
    drop(client);

    let mut fresh = connect_character(clients, 0xad, 1).await?;
    verify_approach(tile(fresh.world_spatial().actor_position)?)?;
    if fresh.join_snapshot().game_session_id == original_session
        || carried_count(&fresh, definition)? != 1
    {
        return Err(
            "N03 relog did not restore locker position and exactly one carried unit".into(),
        );
    }
    assert_holdings(&mut fresh, 1).await?;
    verify_custody(&item_custody(&mut db, clients).await?, &expected_custody)?;
    logout(&mut fresh).await?;
    evidence(&format!(
        "N03_DURABILITY carried_before=2 carried_after_deposit=0 depot_after_deposit=2 carried_after_retrieve=1 relog_carried=1 relog_depot=1 fresh_session=true retrieved_uuid={first} retained_uuid={second}"
    ));

    capture(capture_root, "n03-list-0.json", &first_envelope)?;
    capture(capture_root, "n03-list-1.json", &list_envelope)?;
    capture(capture_root, "n03-retrieve.json", &retrieved_envelope)?;
    capture(capture_root, "n03-expected.txt", expected.as_bytes())?;
    Ok(format!(
        "session={capture_id} item={client_id} pages={} depot={} retrieved={}",
        2, row.depot, retrieved.retrieved
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    match verdict::indexed_timeout(Duration::from_secs(150), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=N03.client status=CAPTURED seconds={:.3} evidence={observed}",
            started.elapsed().as_secs_f64()
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=N03.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=N03.client status=FAIL evidence=timeout"),
    }
}
