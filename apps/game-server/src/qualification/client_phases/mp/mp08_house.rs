//! MP08: two real Account admissions: A prospective owner and B outsider, then guest.
//! Reference (OTS_HYPOTHESIS_ONLY): Crystal main, 201189f871d23597b82caf88fd611454a2ddefb8,
//! src/map/house/house.cpp:200-228 (roles), 230-249 (kick), 252-280 (revocation),
//! 302-355 (move-out items go to the owner inbox), 928-999 (bank rent, calendar cycles).
//! src/game/game.cpp:12435-12512,12730-12754 (bank bid + rent, highest bid + 1), 12515-12544 (move out).
//! Native UUID ACLs/CAS
//! follow HOUSE-WIRE-1. No settlement, funds, house or item fixtures are synthesized.
use super::*;
use bots::Bot;
use oteryn_dev_client::client_runtime::{house as client, house_transfer as transfer, raw_feature};
use oteryn_protocol_oteryn::house as wire;

// Preserve the latest concrete response when a bounded future expires or a dependency fails.
// MP scenarios run serially; this recorder never drives a bot or supplies game state.
static LAST_REPLY: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

fn remember_reply(reply: &impl std::fmt::Debug) {
    if let Ok(mut last) = LAST_REPLY.lock() {
        *last = Some(format!("{reply:?}"));
    }
}

#[derive(Debug)]
struct Property {
    town: String,
    row: client::Row,
}

fn check<T: std::fmt::Debug>(
    case: &str,
    result: TestResult<T>,
    failures: &mut Vec<String>,
) -> Option<T> {
    match result {
        Ok(value) => {
            evidence(&format!("MP_CHECK MP08 {case} PASS {value:?}"));
            Some(value)
        }
        Err(error) => {
            let observed = LAST_REPLY
                .lock()
                .map(|last| last.clone())
                .unwrap_or_default();
            let error = format!("{error}; last_server_reply={observed:?}");
            verdict::record_assertion(case);
            evidence(&format!("MP_CHECK MP08 {case} FAIL {error}"));
            failures.push(format!("{case}: {error}"));
            None
        }
    }
}

async fn bounded<T>(
    case: &'static str,
    seconds: u64,
    future: impl std::future::Future<Output = TestResult<T>>,
) -> TestResult<T> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(seconds);
    verdict::Harness::global().bounded_wait(deadline, case);
    tokio::time::timeout_at(deadline, future)
        .await
        .map_err(|_| format!("MP08 {case} deadline; last reply is in MP08_WIRE evidence"))?
}

async fn send(
    bot: &mut Bot<'_, '_>,
    command: raw_feature::FeatureCommand,
) -> TestResult<raw_feature::GatedCommandResult> {
    let name = bot.name.clone();
    let reply = raw_feature::send(bot.client()?.session_mut(), &command).await?;
    remember_reply(&reply);
    evidence(&format!(
        "MP08_WIRE character={name:?} command={} reply={reply:?}",
        command.kind
    ));
    Ok(reply)
}

async fn mutate(
    bot: &mut Bot<'_, '_>,
    property: &Property,
    revision: u64,
    action: client::Action,
    expected: wire::HouseDisposition,
) -> TestResult<u64> {
    let command = client::mutation(bot.pair.world, &property.row.house, revision, &action)?;
    let reply = send(bot, command).await?;
    let decoded =
        client::result(&reply.payload).map_err(|error| format!("{error:?}; reply={reply:?}; House management route:crates/gameplay/dispatch/src/gameplay_transport/connection/serve/house_hireling/mod.rs:212"))?;
    if reply.status != CommandStatus::Accepted || decoded.0 != expected as u64 {
        return Err(format!(
            "action={action:?} expected={expected:?} reply={reply:?};source:crates/gameplay/dispatch/src/gameplay_transport/house_intent.rs:137"
        ).into());
    }
    if expected == wire::HouseDisposition::Ok
        && matches!(
            action,
            client::Action::Guests(_) | client::Action::Subowners(_)
        )
        && decoded.1 != revision.checked_add(1).ok_or("ACL revision overflow")?
    {
        return Err(format!("ACL revision did not advance exactly once; reply={reply:?}").into());
    }
    Ok(decoded.1)
}

async fn acl(
    bot: &mut Bot<'_, '_>,
    property: &Property,
    selector: wire::HouseAclSelector,
) -> TestResult<wire::HouseAclSnapshot> {
    let command = client::acl_query(
        bot.pair.world,
        &property.town,
        &wire::HouseAclRead {
            house: property.row.house.clone(),
            selector,
        },
    )?;
    let reply = send(bot, command).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("ACL query refused; reply={reply:?}").into());
    }
    wire::decode_house_acl_snapshot(&reply.payload)
        .map_err(|error| format!("ACL {selector:?}: {error:?}; reply={reply:?}").into())
}

/// All towns from the shipped catalogue, all keyset pages; never guess a house key.
async fn ownership(bots: &mut [Bot<'_, '_>]) -> TestResult<Property> {
    #[derive(serde::Deserialize)]
    struct Catalogue {
        houses: Vec<Record>,
    }
    #[derive(serde::Deserialize)]
    struct Record {
        town: Town,
    }
    #[derive(serde::Deserialize)]
    struct Town {
        key: String,
    }
    let mut towns = std::collections::BTreeSet::new();
    for body in [
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../content/houses/houses-00000-00499.json"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../content/houses/houses-00500-00994.json"
        )),
    ] {
        let catalogue: Catalogue = serde_json::from_str(body)?;
        towns.extend(catalogue.houses.into_iter().map(|record| record.town.key));
    }
    let mut candidate = None;
    let mut last_reply = String::new();
    let mut pages = 0;
    let mut rows = 0;
    for town in towns {
        let mut cursor = None;
        for page_index in 0..=50 {
            bots::pump(bots).await?;
            let command = client::query(bots[0].pair.world, &town, cursor.as_deref())?;
            let reply = send(&mut bots[0], command).await?;
            if reply.status != CommandStatus::Accepted {
                return Err(format!("catalogue refused; reply={reply:?}").into());
            }
            let page = client::page(&reply.payload)
                .map_err(|error| format!("town={town} {error}; reply={reply:?}"))?;
            last_reply = format!("town={town} page={page_index} reply={reply:?}");
            pages += 1;
            rows += page.rows.len();
            for row in page.rows {
                if row.kind == 3 {
                    continue;
                }
                let property = Property {
                    town: town.clone(),
                    row,
                };
                if candidate.is_none() && !property.row.owned && property.row.state <= 2 {
                    candidate = Some(property);
                }
            }
            let Some(next) = page.cursor else { break };
            if cursor.as_ref().is_some_and(|old| old >= &next) || page_index == 50 {
                return Err(format!("non-advancing/unbounded catalogue; reply={reply:?}").into());
            }
            cursor = Some(next);
        }
    }
    let property = candidate.ok_or_else(|| format!("free-house-unavailable; pages={pages} rows={rows}; last_catalogue={last_reply};source:crates/gameplay/durability/src/durability/house_ownership/bid.rs:74"))?;
    let mut database = sqlx::PgConnection::connect(bots[0].pair.url).await?;
    let house = &property.row.house;
    let bank_before: i64 = sqlx::query_scalar("SELECT coalesce((SELECT balance FROM game_account_bank_balances WHERE account_id=$1::uuid AND world_id=encode($2,'hex')::uuid),0)")
        .bind(&bots[0].pair.accounts[bots[0].index]).bind(bots[0].pair.world.as_slice()).fetch_one(&mut database).await?;
    // Crystal game.cpp:12498: an outbid advances by one gold, not a made-up fixture bid.
    let maximum = property
        .row
        .price
        .checked_add(1)
        .ok_or("house bid overflow")?;
    let command = client::mutation(
        bots[0].pair.world,
        &property.row.house,
        0,
        &client::Action::Bid(maximum),
    )?;
    let bid_reply = send(&mut bots[0], command).await?;
    let bank_after: i64 = sqlx::query_scalar("SELECT coalesce((SELECT balance FROM game_account_bank_balances WHERE account_id=$1::uuid AND world_id=encode($2,'hex')::uuid),0)")
        .bind(&bots[0].pair.accounts[bots[0].index]).bind(bots[0].pair.world.as_slice()).fetch_one(&mut database).await?;
    database.close().await?;
    // A successful bid reserves bank gold, but does not grant House custody.
    // Re-read the exact row without relying on the exclusive keyset cursor.
    let mut cursor = None;
    for _ in 0..=50 {
        let command = client::query(bots[0].pair.world, &property.town, cursor.as_deref())?;
        let reply = send(&mut bots[0], command).await?;
        if reply.status != CommandStatus::Accepted {
            return Err(format!(
                "post-bid catalogue refused; bid_reply={bid_reply:?} reply={reply:?}"
            )
            .into());
        }
        let page =
            client::page(&reply.payload).map_err(|error| format!("{error:?}; reply={reply:?}"))?;
        if let Some(row) = page
            .rows
            .into_iter()
            .find(|row| row.house == property.row.house)
        {
            if bid_reply.status == CommandStatus::Accepted
                && client::result(&bid_reply.payload)
                    .map_err(|error| format!("bid decode: {error}; reply={bid_reply:?}"))?
                    .0
                    == wire::HouseDisposition::Ok as u64
                && row.owned
            {
                let charged = i128::from(bank_before) - i128::from(bank_after);
                let needed = i128::from(maximum) + i128::from(property.row.rent);
                if charged != needed {
                    return Err(format!("bank bid/rent debit differs from Crystal; charged={charged} expected={needed} reply={bid_reply:?} row={row:?}").into());
                }
                return Ok(Property {
                    town: property.town,
                    row,
                });
            }
            return Err(format!("bank-funded-ownership-not-observed house={house} maximum={maximum} bank_before={bank_before} bank_after={bank_after} bid_reply={bid_reply:?} catalogue_reply={reply:?} row={row:?};source:crates/gameplay/durability/src/durability/house_ownership/settlement.rs:33; bank-debit:crates/gameplay/durability/src/durability/house_ownership/bid.rs:141").into());
        }
        let Some(next) = page.cursor else { break };
        if cursor.as_ref().is_some_and(|old| old >= &next) {
            break;
        }
        cursor = Some(next);
        bots::pump(bots).await?;
    }
    Err(format!("bid house disappeared; bid_reply={bid_reply:?} last_catalogue={last_reply};source:crates/gameplay/durability/src/durability/house_ownership/bid.rs:74").into())
}

async fn verify_acl(
    bot: &mut Bot<'_, '_>,
    property: &Property,
    selector: wire::HouseAclSelector,
    members: &[[u8; 16]],
    revision: u64,
) -> TestResult<String> {
    let snapshot = acl(bot, property, selector).await?;
    let mut expected = members.to_vec();
    expected.sort();
    if snapshot.members != expected || snapshot.revision != revision {
        return Err(format!("expected={expected:?} revision={revision} reply={snapshot:?}").into());
    }
    // Read-only observer, bounded to the protocol limit plus one. Never fabricate authority.
    let mut database = sqlx::PgConnection::connect(bot.pair.url).await?;
    let key = match selector {
        wire::HouseAclSelector::Guest => "guest",
        wire::HouseAclSelector::Subowner => "subowner",
        wire::HouseAclSelector::Door { .. } => return Err("unexpected door selector".into()),
    };
    let durable: Vec<Vec<u8>> = sqlx::query_scalar("SELECT uuid_send(character_id) FROM game_house_acl_entries WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 AND list_key=$3 ORDER BY character_id LIMIT 201")
        .bind(bot.pair.world.as_slice()).bind(&property.row.house).bind(key).fetch_all(&mut database).await?;
    let durable_revision: String = sqlx::query_scalar("SELECT acl_revision::text FROM game_house_properties WHERE world_id=encode($1,'hex')::uuid AND house_key=$2")
        .bind(bot.pair.world.as_slice()).bind(&property.row.house).fetch_one(&mut database).await?;
    database.close().await?;
    if durable_revision != revision.to_string() {
        return Err(
            format!("durable_revision={durable_revision} client_reply={snapshot:?}").into(),
        );
    }
    let expected: Vec<Vec<u8>> = expected.into_iter().map(|id| id.to_vec()).collect();
    if durable != expected {
        return Err(format!("durable={durable:?} client_reply={snapshot:?}").into());
    }
    Ok(format!(
        "client_reply={snapshot:?}; durable_members={durable:?}"
    ))
}

async fn enter(bot: &mut Bot<'_, '_>, property: &Property, allowed: bool) -> TestResult<String> {
    let payload = transfer::encode_house_transfer_intent(&transfer::HouseTransferIntent {
        exit: false,
        house: property.row.house.clone(),
    })
    .map_err(|error| format!("house transfer encode: {error:?}"))?;
    let reply = send(
        bot,
        raw_feature::FeatureCommand {
            capability: transfer::CAPABILITY_HOUSE_TRANSFER_V1,
            kind: transfer::COMMAND_TYPE_HOUSE_TRANSFER,
            payload,
        },
    )
    .await?;
    if allowed {
        if reply.status != CommandStatus::Accepted {
            return Err(format!("guest entry refused; reply={reply:?}").into());
        }
        let ack = transfer::decode_house_transfer_ack(&reply.payload)
            .map_err(|error| format!("{error:?}; reply={reply:?}"))?;
        if ack.house.as_deref() != Some(&property.row.house)
            || ack.world != bot.pair.world
            || ack.session != *bot.client()?.join_snapshot().game_session_id.as_bytes()
        {
            return Err(format!("wrong house/session/world; reply={reply:?}").into());
        }
    } else if reply.status != CommandStatus::Rejected || !reply.payload.is_empty() {
        return Err(format!("outsider entered house; reply={reply:?}").into());
    }
    Ok(format!("allowed={allowed} reply={reply:?}"))
}

async fn exit_house(bot: &mut Bot<'_, '_>, property: &Property) -> TestResult<String> {
    let payload = transfer::encode_house_transfer_intent(&transfer::HouseTransferIntent {
        exit: true,
        house: property.row.house.clone(),
    })
    .map_err(|error| format!("exit encode: {error:?}"))?;
    let reply = send(
        bot,
        raw_feature::FeatureCommand {
            capability: transfer::CAPABILITY_HOUSE_TRANSFER_V1,
            kind: transfer::COMMAND_TYPE_HOUSE_TRANSFER,
            payload,
        },
    )
    .await?;
    let ack = transfer::decode_house_transfer_ack(&reply.payload)
        .map_err(|error| format!("exit decode: {error:?}; reply={reply:?}"))?;
    if reply.status != CommandStatus::Accepted || ack.house.is_some() {
        return Err(format!("exit failed; reply={reply:?}").into());
    }
    Ok(format!("Channel handoff reply={reply:?}"))
}

async fn approach_entry(
    bots: &mut [Bot<'_, '_>],
    index: usize,
    world: &BundleWorld,
    property: &Property,
) -> TestResult {
    let (x, y, floor) = property
        .row
        .entrance
        .ok_or("catalogue omitted entrance metadata")?;
    let from = tile(bots[index].client()?.world_spatial().actor_position)?;
    let path = route(world, from, |at| {
        at.x == x && at.y == y && at.floor == floor
    })
    .ok_or_else(|| {
        format!(
            "no walkable route from {from:?} to server catalogue entrance {:?}; row={:?}",
            (x, y, floor),
            property.row
        )
    })?;
    bots::walk_bot(bots, index, world, &path).await
}

async fn observe_ejected(bots: &mut [Bot<'_, '_>], property: &Property) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        bots::pump(bots).await?;
        let bot = bots[1].client()?;
        let scope = bot
            .session_mut()
            .gated_snapshot(transfer::STATE_DOMAIN_HOUSE_SCOPE);
        if scope.is_none() {
            let at = tile(bot.world_spatial().actor_position)?;
            if property.row.entrance == Some((at.x, at.y, at.floor)) {
                return Ok(());
            }
        }
        if tokio::time::Instant::now() >= deadline {
            let position = bot.world_spatial().actor_position;
            let scope_reply = bot
                .session_mut()
                .gated_snapshot(transfer::STATE_DOMAIN_HOUSE_SCOPE);
            return Err(format!("guest not ejected to entrance; scope_reply={scope_reply:?} position={position:?};source:crates/gameplay/durability/src/durability/house_management/acl.rs:51").into());
        }
    }
}

#[derive(Debug)]
struct PlacedItem {
    id: String,
    position: oteryn_protocol_oteryn::item_view::WorldTilePosition,
    definition: std::num::NonZeroU32,
    count: std::num::NonZeroU32,
}

async fn house_items(bot: &Bot<'_, '_>, property: &Property) -> TestResult<Vec<String>> {
    let mut database = sqlx::PgConnection::connect(bot.pair.url).await?;
    let ids: Vec<String> = sqlx::query_scalar("SELECT item_instance_id::text FROM game_item_house_interior_locations WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 ORDER BY item_instance_id LIMIT 201")
        .bind(bot.pair.world.as_slice()).bind(&property.row.house).fetch_all(&mut database).await?;
    database.close().await?;
    if ids.len() > 200 {
        return Err("house item observation exceeded 200-item bound".into());
    }
    Ok(ids)
}

async fn place_item(
    bots: &mut [Bot<'_, '_>],
    world: &BundleWorld,
    property: &Property,
) -> TestResult<PlacedItem> {
    approach_entry(bots, 0, world, property).await?;
    enter(&mut bots[0], property, true).await?;
    let before = house_items(&bots[0], property).await?;
    let scope = bots[0]
        .client()?
        .session_mut()
        .gated_snapshot(transfer::STATE_DOMAIN_HOUSE_SCOPE)
        .ok_or("house scope snapshot absent after transfer")?;
    let ack = transfer::decode_house_transfer_ack(&scope.payload)
        .map_err(|error| format!("house position: {error:?}; reply={scope:?}"))?;
    let position = oteryn_protocol_oteryn::item_view::WorldTilePosition {
        x: ack.position[0],
        y: ack.position[1],
        floor: i16::try_from(ack.position[2])?,
    };
    let inventory = bots[0]
        .client()?
        .inventory()
        .ok_or_else(|| format!("A inventory snapshot absent after house reply={ack:?};source:crates/gameplay/dispatch/src/gameplay_transport/house_transfer/session_views.rs:84"))?;
    let item = inventory.entries.first().copied().ok_or_else(|| format!("no carried item to place; inventory_reply={inventory:?};source:crates/gameplay/dispatch/src/gameplay_transport/item_move.rs:323"))?;
    let reply = bots[0]
        .client()?
        .move_item(&oteryn_dev_client::ItemMoveIntent {
            source: item.handle,
            destination: oteryn_dev_client::ItemMoveDestination::Ground(position),
        })
        .await.map_err(|error| format!("house item move reply/error={error:?};source:crates/gameplay/dispatch/src/gameplay_transport/connection/serve/house_hireling/mod.rs:212; rejected-wire-reply:crates/gameplay/dispatch/src/gameplay_transport/connection/serve/house_hireling/reply.rs:114"))?;
    remember_reply(&reply);
    evidence(&format!("MP08_WIRE item-move reply={reply:?}"));
    if reply.outcome != oteryn_dev_client::ItemMoveOutcome::Moved {
        return Err(format!("house ground placement refused; reply={reply:?};source:crates/gameplay/durability/src/durability/item_loop_move/house.rs:114").into());
    }
    let after = house_items(&bots[0], property).await?;
    let added: Vec<_> = after
        .into_iter()
        .filter(|id| !before.contains(id))
        .collect();
    if added.len() != 1 {
        return Err(
            format!("expected one durable house item; reply={reply:?} added={added:?}").into(),
        );
    }
    Ok(PlacedItem {
        id: added[0].clone(),
        position,
        definition: item.item_definition_ref,
        count: item.count,
    })
}

async fn persistent_item(
    bot: &mut Bot<'_, '_>,
    property: &Property,
    item: &PlacedItem,
) -> TestResult<String> {
    let ids = house_items(bot, property).await?;
    // The real client must also see the floor item after its fresh admission.
    use oteryn_protocol_oteryn::{browse_field as browse, container_tree as tree};
    let payload = browse::encode_browse_field_intent(&browse::BrowseFieldIntent::Open {
        position: item.position,
    })
    .map_err(|error| format!("browse encode: {error:?}"))?;
    let reply = send(
        bot,
        raw_feature::FeatureCommand {
            capability: browse::CAPABILITY_BROWSE_FIELD_V1,
            kind: browse::COMMAND_TYPE_BROWSE_FIELD_INTENT,
            payload,
        },
    )
    .await?;
    let outcome = tree::decode_container_view_result(&reply.payload)
        .map_err(|error| format!("browse decode: {error:?}; reply={reply:?}"))?;
    if !ids.contains(&item.id)
        || reply.status != CommandStatus::Accepted
        || outcome != tree::ContainerViewOutcome::Opened
    {
        return Err(format!("item lost after relog/save; item={item:?} durable={ids:?} reply={reply:?};source:crates/gameplay/dispatch/src/gameplay_transport/house_transfer/session_views.rs:84").into());
    }
    let delta = bot
        .client()?
        .session_mut()
        .read_gated_delta(
            tree::STATE_DOMAIN_CONTAINER_VIEWS,
            tree::DELTA_TYPE_CONTAINER_VIEWS_V1,
        )
        .await?;
    let views = selected_container_views(bot.client()?.selected_capabilities(), &delta.payload)
        .map_err(|error| format!("container views: {error:?}; delta_reply={delta:?}"))?;
    if !views.views.iter().any(|view| {
        view.browse.is_some()
            && view.entries.iter().any(|entry| {
                entry.item_definition_ref == item.definition && entry.count == item.count
            })
    }) {
        return Err(format!(
            "durable item lacks floor projection; item={item:?} reply={reply:?} views={views:?}"
        )
        .into());
    }
    Ok(format!(
        "item={item:?} durable row survives fresh admission; reply={reply:?} views={views:?}"
    ))
}

async fn rent(bot: &Bot<'_, '_>, property: &Property, since_ms: u64) -> TestResult<String> {
    let mut database = sqlx::PgConnection::connect(bot.pair.url).await?;
    // Crystal house.cpp:982-993: 1/7/30/365 days, never a shortened harness cycle.
    let observation: (String, String, i64, Option<i64>, Option<i64>, Option<i64>) = sqlx::query_as("SELECT c.rent_period,p.paid_until::text,p.rent_gold,r.amount,e.balance_before-e.balance_after,(extract(epoch FROM p.paid_until-r.paid_at))::bigint FROM game_house_properties p CROSS JOIN game_house_rent_config c LEFT JOIN LATERAL (SELECT amount,bank_entry_id,paid_at FROM game_house_rent_payments WHERE world_id=p.world_id AND house_key=p.house_key AND owner_character_id=p.owner_character_id AND paid_at >= to_timestamp($3::double precision/1000.0) ORDER BY paid_at DESC LIMIT 1) r ON true LEFT JOIN game_account_bank_entries e ON e.entry_id=r.bank_entry_id WHERE p.world_id=encode($1,'hex')::uuid AND p.house_key=$2")
        .bind(bot.pair.world.as_slice()).bind(&property.row.house).bind(since_ms as f64).fetch_one(&mut database).await?;
    database.close().await?;
    let days = match observation.0.as_str() {
        "daily" => 1,
        "weekly" => 7,
        "monthly" => 30,
        "yearly" => 365,
        _ => 0,
    };
    if days == 0
        || observation.3 != Some(observation.2)
        || observation.4 != Some(observation.2)
        || observation.5 != Some(days * 86_400)
    {
        return Err(format!("Crystal rent cycle not observed; database_reply={observation:?};source:apps/game-server/src/node/house_rent_scheduler.rs:24; cycle:apps/game-server/migrations/0941_house_rent_parity.sql:16").into());
    }
    Ok(format!(
        "Crystal cycle_days={days} rent/bank receipt={observation:?}"
    ))
}

async fn move_out(
    bot: &mut Bot<'_, '_>,
    property: &Property,
    item: Option<&PlacedItem>,
) -> TestResult<String> {
    // Leave House custody through the native client handoff before managing the property.
    let in_house = bot
        .client()?
        .session_mut()
        .gated_snapshot(transfer::STATE_DOMAIN_HOUSE_SCOPE)
        .is_some();
    if in_house {
        exit_house(bot, property).await?;
    }
    // Ask for the real move-out flow, never issue an internal disposition job.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis();
    // Native HOUSE-WIRE-1 requires >= one day (house_management.rs:217).
    let due = u64::try_from(now)?
        .checked_add(86_400_000 + 60_000)
        .ok_or("move-out date overflow")?;
    let command = client::mutation(
        bot.pair.world,
        &property.row.house,
        property.row.acl_revision,
        &client::Action::Notice(due),
    )?;
    let reply = send(bot, command).await?;
    let mut database = sqlx::PgConnection::connect(bot.pair.url).await?;
    let id = item.map(|item| item.id.as_str());
    // Crystal calls this transferToDepot but actually delivers to player inbox: house.cpp:352.
    let recovered: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_character_inbox_locations WHERE item_instance_id=$1::uuid AND character_id=encode($2,'hex')::uuid)")
        .bind(id).bind(bot.character.as_slice()).fetch_one(&mut database).await?;
    let owned: bool = sqlx::query_scalar("SELECT coalesce(game_house_effective_owner(encode($1,'hex')::uuid,$2)=encode($3,'hex')::uuid,false)")
        .bind(bot.pair.world.as_slice()).bind(&property.row.house).bind(bot.character.as_slice()).fetch_one(&mut database).await?;
    database.close().await?;
    if reply.status != CommandStatus::Accepted
        || client::result(&reply.payload)
            .map_err(|error| format!("move-out decode: {error}; reply={reply:?}"))?
            .0
            != wire::HouseDisposition::Ok as u64
        || owned
        || !recovered
    {
        return Err(format!("move-out/depot recovery not completed; due_ms={due} owned={owned} recovered={recovered} item={item:?} reply={reply:?};source:crates/gameplay/durability/src/durability/house_management.rs:217; disposition:crates/gameplay/durability/src/durability/house_disposition/jobs.rs:1").into());
    }
    Ok(format!(
        "ownership released; item recovered to Crystal depot/inbox; reply={reply:?}"
    ))
}

pub(super) async fn run(
    core: &SeamClients<'_>,
    packets: &SeamClients<'_>,
    world: &BundleWorld,
    _towns: &oteryn_world_bundle::subarea::Table,
) -> TestResult {
    if let Ok(mut last) = LAST_REPLY.lock() {
        *last = None;
    }
    let started = std::time::Instant::now();
    let since_ms = u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis(),
    )?;
    let mut failures = Vec::new();
    let admission_deadline = tokio::time::Instant::now() + Duration::from_secs(180);
    verdict::Harness::global().bounded_wait(admission_deadline, "MP08-admissions");
    // A timed-out pool drops its partial sockets; release_all below still waits for
    // both indices of both pairs, exactly as after a returned partial setup error.
    let (mut bots, setup) = tokio::time::timeout_at(
        admission_deadline,
        bots::connect_bots(&[(core, 0, None), (packets, 0, None)]),
    ).await.unwrap_or_else(|_| (Vec::new(), Err("MP08 admissions deadline; partial sockets dropped;source:apps/game-server/src/qualification/client_phases/mp/bots.rs:78".into())));
    let admitted = check("admissions", setup, &mut failures).is_some();
    let property = if admitted {
        check(
            "free-house-bank-bid-ownership",
            bounded("ownership", 180, ownership(&mut bots)).await,
            &mut failures,
        )
    } else {
        check(
            "free-house-bank-bid-ownership",
            Err::<Property, _>(format!("admission failed: {failures:?}").into()),
            &mut failures,
        )
    };
    let owner_reply = format!("owner precondition failed; concrete reply: {failures:?}");
    // B is the non-guest before A invites that same real player.
    check(
        "non-guest-refused",
        bounded("non-guest-refused", 180, async {
            let property = property.as_ref().ok_or_else(|| owner_reply.clone())?;
            let origin = tile(bots[1].client()?.world_spatial().actor_position)?;
            approach_entry(&mut bots, 1, world, property).await?;
            let refusal = enter(&mut bots[1], property, false).await;
            let from = tile(bots[1].client()?.world_spatial().actor_position)?;
            let path = route(world, from, |at| at == origin).ok_or("B return route unavailable")?;
            bots::walk_bot(&mut bots, 1, world, &path).await?;
            refusal
        })
        .await,
        &mut failures,
    );
    let revision = check(
        "invite-guest",
        bounded("invite-guest", 30, async {
            let property = property.as_ref().ok_or_else(|| owner_reply.clone())?;
            let guest = bots[1].character;
            let revision = mutate(
                &mut bots[0],
                property,
                property.row.acl_revision,
                client::Action::Guests(vec![guest]),
                wire::HouseDisposition::Ok,
            )
            .await?;
            verify_acl(
                &mut bots[0],
                property,
                wire::HouseAclSelector::Guest,
                &[guest],
                revision,
            )
            .await?;
            Ok(revision)
        })
        .await,
        &mut failures,
    );
    let invite_reply = failures.last().cloned().unwrap_or_default();
    let entered = check(
        "guest-door-entry",
        bounded("guest-door-entry", 180, async {
            let property = property.as_ref().ok_or_else(|| owner_reply.clone())?;
            revision.ok_or_else(|| format!("invitation failed; {invite_reply}"))?;
            approach_entry(&mut bots, 1, world, property).await?;
            enter(&mut bots[1], property, true).await
        })
        .await,
        &mut failures,
    )
    .is_some();
    let placed = check(
        "item-into-house",
        bounded("item-into-house", 180, async {
            let property = property.as_ref().ok_or_else(|| owner_reply.clone())?;
            place_item(&mut bots, world, property).await
        })
        .await,
        &mut failures,
    );
    let placement_reply = failures.last().cloned().unwrap_or_default();
    let sessions: Vec<_> = bots
        .iter()
        .filter_map(|b| {
            b.session
                .as_ref()
                .map(|s| s.join_snapshot().game_session_id)
        })
        .collect();
    let relogged = check(
        "relog",
        bounded("relog", 1100, async {
            if !admitted {
                return Err(owner_reply.clone().into());
            }
            bots::relog_all(&mut bots).await?;
            for (bot, old) in bots.iter_mut().zip(&sessions) {
                let reply = bot.client()?.join_snapshot();
                if reply.game_session_id == *old {
                    return Err(format!("relog reused session; reply={reply:?}").into());
                }
            }
            Ok("two fresh real GameSessions")
        })
        .await,
        &mut failures,
    )
    .is_some();
    let relog_reply = failures.last().cloned().unwrap_or_default();
    check(
        "item-persists-after-relog-save",
        bounded("item-persists-after-relog-save", 45, async {
            let property = property.as_ref().ok_or_else(|| owner_reply.clone())?;
            if !relogged {
                return Err(format!("relog failed; {relog_reply}").into());
            }
            let item = placed
                .as_ref()
                .ok_or_else(|| format!("house placement failed; {placement_reply}"))?;
            persistent_item(&mut bots[0], property, item).await
        })
        .await,
        &mut failures,
    );
    check(
        "revoke-guest",
        bounded("revoke-guest", 45, async {
            let property = property.as_ref().ok_or_else(|| owner_reply.clone())?;
            let revision = revision.ok_or_else(|| format!("invitation failed; {invite_reply}"))?;
            let after = mutate(
                &mut bots[0],
                property,
                revision,
                client::Action::Guests(vec![]),
                wire::HouseDisposition::Ok,
            )
            .await?;
            let observation = verify_acl(
                &mut bots[0],
                property,
                wire::HouseAclSelector::Guest,
                &[],
                after,
            )
            .await?;
            if entered {
                observe_ejected(&mut bots, property).await?;
            }
            Ok(observation)
        })
        .await,
        &mut failures,
    );
    check(
        "rent-crystal-cycle",
        bounded("rent-crystal-cycle", 30, async {
            let property = property.as_ref().ok_or_else(|| {
                format!("{owner_reply};source:apps/game-server/src/node/house_rent_scheduler.rs:24")
            })?;
            rent(&bots[0], property, since_ms).await
        })
        .await,
        &mut failures,
    );
    check("leave-house-items-to-depot",bounded("leave-house-items-to-depot",45,async {
        let property = property.as_ref().ok_or_else(|| format!("{owner_reply};source:crates/gameplay/durability/src/durability/house_management.rs:217"))?;
        move_out(&mut bots[0],property,placed.as_ref()).await
    }).await,&mut failures);
    check(
        "release",
        bounded("release", 1800, bots::release_all(&mut bots, core, packets)).await,
        &mut failures,
    );
    let status = if failures.is_empty() { "PASS" } else { "FAIL" };
    evidence(&format!(
        "e2e_all scenario=MP08.full status={status} seconds={:.3}",
        started.elapsed().as_secs_f64()
    ));
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; ").into())
    }
}
