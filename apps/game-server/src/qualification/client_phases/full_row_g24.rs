//! G24 market details and statistics (capability 63, command 6126).
//! 1. Admit the node-boot character and walk to a depot; read catalogue page 0: every ware must satisfy
//!    the (empty) filter, and a name search built from the first ware must keep that ware.
//! 2. Query the statistics and the order book of that ware: the echoed definition must match.
//! 3. Pick the lowest inventory-ready ware ID, settle a trade, then verify both histories,
//!    item delivery, exact gold deltas and completed-price statistics after fresh admissions.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::market::CAPABILITY_MARKET_V1;
use oteryn_protocol_oteryn::market_details as wire;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(480), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = match result {
        Ok(Ok(())) => (
            "PASS",
            "market-catalogue-trade-item-gold-history-and-statistics-validated".to_owned(),
        ),
        Ok(Err(error)) => ("FAIL", compact(&error.to_string())),
        Err(_) => ("FAIL", "step=journey-observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G24.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn compact(value: &str) -> String {
    let joined = value.split_whitespace().collect::<Vec<_>>().join("-");
    joined.chars().take(400).collect()
}

fn gate(client: &DevClientSession, capability: u32) -> TestResult {
    if client.selected_capabilities().contains(&capability) {
        return Ok(());
    }
    Err(format!(
        "step=admit observed=capability-{capability}-not-selected needed=capability-{capability}-selected"
    )
    .into())
}

async fn send(
    client: &mut DevClientSession,
    capability: u32,
    command: u32,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    gate(client, capability)?;
    let reply = client
        .session_mut()
        .gated_command(capability, command, payload)
        .await?;
    client.service_liveness(Duration::from_millis(50)).await?;
    Ok((reply.status, reply.payload))
}

async fn accepted(
    client: &mut DevClientSession,
    capability: u32,
    command: u32,
    payload: &[u8],
) -> TestResult<Vec<u8>> {
    let (status, payload) = send(client, capability, command, payload).await?;
    if status != CommandStatus::Accepted {
        return Err(format!(
            "step=command-{command} observed=status:{status:?} needed=accepted-reply"
        )
        .into());
    }
    Ok(payload)
}

async fn finish(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Ok(())
}

fn search_term(name: &str) -> String {
    name.chars().take(4).collect::<String>().to_lowercase()
}

async fn ask(client: &mut DevClientSession, query: &wire::Query) -> TestResult<wire::Reply> {
    let payload = wire::encode_query(query)
        .map_err(|error| format!("step=query observed=encode-{error:?}"))?;
    let reply = accepted(
        client,
        CAPABILITY_MARKET_V1,
        wire::COMMAND_TYPE_MARKET_DETAILS,
        &payload,
    )
    .await?;
    let reply = wire::decode_reply(&reply)
        .map_err(|error| format!("step=query observed=decode-{error:?}"))?;
    if let wire::Reply::Refused { code } = reply {
        return Err(format!("step=market-details query={query:?} observed=refused-{code}").into());
    }
    Ok(reply)
}

const PRICE: u64 = 1;

fn market_request(
    action: u8,
    definition: u32,
    offer: [u8; 16],
    amount: u32,
    price: u64,
) -> oteryn_protocol_oteryn::market::MarketRequest {
    oteryn_protocol_oteryn::market::MarketRequest {
        action,
        definition,
        offer,
        amount,
        price,
        anonymous: false,
        page: 0,
    }
}

async fn market(
    client: &mut DevClientSession,
    request: oteryn_protocol_oteryn::market::MarketRequest,
) -> TestResult<oteryn_protocol_oteryn::market::MarketReply> {
    let action = request.action;
    let reply = super::full_row_e06::command(client, request).await?;
    if reply.code != 1 || reply.action != action {
        return Err(format!("step=market action={action} observed={reply:?}").into());
    }
    Ok(reply)
}

async fn history(
    client: &mut DevClientSession,
    offer: [u8; 16],
    definition: u32,
    side: u8,
) -> TestResult {
    let history = market(client, market_request(4, 0, [0; 16], 0, 0)).await?;
    if !history.rows.iter().any(|row| {
        row.offer == offer
            && row.definition == definition
            && row.amount == 1
            && row.price == PRICE
            && row.side == side
            && row.state == 2
    }) {
        return Err(format!("step=history side={side} observed={history:?}").into());
    }
    Ok(())
}

async fn walk_once(
    client: &mut DevClientSession,
    world: &BundleWorld,
    target: impl Fn(TilePos) -> bool,
) -> TestResult {
    let start = tile(client.world_spatial().actor_position)?;
    let path = route(world, start, target).ok_or("step=route observed=no-authored-route")?;
    for direction in path {
        let from = tile(client.world_spatial().actor_position)?;
        let expected =
            destination(world, from, direction).ok_or("step=route observed=invalid-route")?;
        let outcome = client.step(direction).await?;
        if outcome.disposition != StepDisposition::Moved
            || outcome.world_spatial_delta.is_none()
            || tile(client.world_spatial().actor_position)? != expected
        {
            return Err(format!(
                "step=route from={from:?} expected={expected:?} observed={outcome:?}"
            )
            .into());
        }
        verdict::Harness::global().progress("walk");
        client.take_events();
    }
    Ok(())
}

async fn at_depot(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    walk_once(client, world, |at| near_locker(world, at)).await
}

fn inventory_ware(
    native: &crate::content::native_gameplay::NativeGameplayState,
    definition: u32,
) -> Option<u32> {
    use oteryn_gameplay_items::item_ref_admission::generation_item_facts;
    let (key, revision) = native
        .item_index()?
        .definition_at(std::num::NonZeroU32::new(definition)?)?;
    let id = character_prep::item_id(key).ok()?;
    let facts = generation_item_facts(
        native,
        &crate::durability::item_mint::TypedDefinitionRef {
            family: "Item".into(),
            production_key: key.into(),
            revision_ref: revision.into(),
        },
    )
    .ok()?;
    (facts.stack != crate::durability::item_transfer::ItemStackClass::Unknown).then_some(id)
}

async fn catalogue_ware(
    client: &mut DevClientSession,
    native: &crate::content::native_gameplay::NativeGameplayState,
    mut items: Vec<wire::Ware>,
    mut has_next: bool,
) -> TestResult<(u32, u32)> {
    let mut selected = None;
    let mut previous = 0;
    for page in 0..=4096 {
        if items.is_empty() || items.len() > wire::PAGE_SIZE {
            return Err(format!("step=catalogue invalid-page={page} size={}", items.len()).into());
        }
        for ware in &items {
            if ware.definition <= previous {
                return Err(format!(
                    "step=catalogue page={page} unordered-definition={}",
                    ware.definition
                )
                .into());
            }
            previous = ware.definition;
            if let Some(id) = inventory_ware(native, ware.definition) {
                let candidate = (id, ware.definition);
                selected =
                    Some(selected.map_or(candidate, |current: (u32, u32)| current.min(candidate)));
            }
        }
        if !has_next {
            return selected.ok_or_else(|| "step=catalogue observed=no-inventory-ware".into());
        }
        let wire::Reply::Catalogue {
            items: next,
            page: echoed,
            has_next: more,
            ..
        } = ask(
            client,
            &wire::Query::Catalogue {
                filter: wire::Filter::default(),
                page: page + 1,
            },
        )
        .await?
        else {
            return Err("step=catalogue observed=not-a-catalogue".into());
        };
        if echoed != page + 1 {
            return Err(format!(
                "step=catalogue observed=page-{echoed} expected={}",
                page + 1
            )
            .into());
        }
        (items, has_next) = (next, more);
    }
    Err("step=catalogue observed=page-budget-exhausted".into())
}

async fn settle(
    buyer: &mut DevClientSession,
    buyer_scope: &SeamClients<'_>,
    seller_scope: &SeamClients<'_>,
    world: &BundleWorld,
    id: u32,
    definition: u32,
) -> TestResult<DevClientSession> {
    use super::full_row_e06::bank_balance;
    use super::full_row_e08::depot_intent;
    use super::full_row_e09::{depot_search, inbox};
    use oteryn_protocol_oteryn::depot::{DepotAction, DepotIntent, DepotOutcome};

    let seller_before = bank_balance(seller_scope, 0).await?;
    let buyer_before = bank_balance(buyer_scope, 0).await?;
    let inbox_before = inbox(&depot_search(buyer, id).await?);
    let offer = market(buyer, market_request(5, definition, [0; 16], 1, PRICE))
        .await?
        .offer;
    finish(buyer).await?;

    let mut seller = connect_character(seller_scope, 0x8b, 0).await?;
    character_prep::grant_item(&seller, id, 1).await?;
    finish(&mut seller).await?;
    drop(seller);
    let mut seller = connect_character(seller_scope, 0x8b, 0).await?;
    at_depot(&mut seller, world).await?;
    let item = seller
        .inventory()
        .and_then(|inventory| {
            inventory.entries.iter().find(|entry| {
                entry.item_definition_ref.get() == definition && entry.count.get() == 1
            })
        })
        .ok_or("step=seed observed=missing-exact-ware")?;
    let handle = item.handle;
    let browse = depot_intent(
        &mut seller,
        DepotIntent {
            town: None,
            box_number: 1,
            page: 0,
            action: DepotAction::Browse,
        },
    )
    .await?;
    if browse.outcome != DepotOutcome::Opened || browse.town.is_none() {
        return Err(format!("step=depot observed={browse:?}").into());
    }
    let deposit = depot_intent(
        &mut seller,
        DepotIntent {
            town: browse.town,
            box_number: 1,
            page: 0,
            action: DepotAction::Deposit(handle),
        },
    )
    .await?;
    if deposit.outcome != DepotOutcome::Moved {
        return Err(format!("step=deposit observed={deposit:?}").into());
    }
    market(&mut seller, market_request(8, 0, offer, 1, 0)).await?;
    let seller_session = seller.join_snapshot().game_session_id;
    finish(&mut seller).await?;
    drop(seller);

    let mut fresh_seller = connect_character(seller_scope, 0x8b, 0).await?;
    if fresh_seller.join_snapshot().game_session_id == seller_session {
        return Err("step=relog observed=seller-session-reused".into());
    }
    at_depot(&mut fresh_seller, world).await?;
    history(&mut fresh_seller, offer, definition, 1).await?;
    if seller_before.checked_add(PRICE) != Some(bank_balance(seller_scope, 0).await?) {
        return Err("step=relog observed=seller-gold-delta-mismatch".into());
    }
    finish(&mut fresh_seller).await?;
    drop(fresh_seller);

    let mut fresh_buyer = connect_character(buyer_scope, 0x8a, 0).await?;
    if fresh_buyer.join_snapshot().game_session_id == buyer.join_snapshot().game_session_id {
        return Err("step=relog observed=buyer-session-reused".into());
    }
    at_depot(&mut fresh_buyer, world).await?;
    history(&mut fresh_buyer, offer, definition, 1).await?;
    let fee = oteryn_gameplay_economy::market::placement_fee(1, PRICE)
        .map_err(|error| format!("step=fee observed={error:?}"))?;
    if inbox_before.checked_add(1) != Some(inbox(&depot_search(&mut fresh_buyer, id).await?))
        || buyer_before.checked_sub(PRICE + fee) != Some(bank_balance(buyer_scope, 0).await?)
    {
        return Err("step=relog observed=buyer-item-or-gold-delta-mismatch".into());
    }
    Ok(fresh_buyer)
}

async fn fund_buyer(
    buyer: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
) -> TestResult {
    let banker = npcs
        .npcs
        .iter()
        .find(|npc| npc.key == "oteryn:npc.paulie")
        .ok_or("step=bank observed=missing-paulie")?;
    walk_once(buyer, world, |at| {
        banker
            .placements
            .iter()
            .any(|p| p.floor == at.floor && p.x.abs_diff(at.x).max(p.y.abs_diff(at.y)) <= 3)
    })
    .await?;
    let actor = buyer
        .world_entities()
        .and_then(|entities| {
            entities
                .iter()
                .find(|entity| {
                    entity.kind == oteryn_dev_client::EntityKind::Npc
                        && banker.placements.iter().any(|p| {
                            entity.position.x == i32::from(p.x)
                                && entity.position.y == i32::from(p.y)
                                && entity.position.floor == -i16::from(p.floor)
                        })
                })
                .map(|entity| entity.entity)
        })
        .ok_or("step=bank observed=paulie-not-visible")?;
    let fee = oteryn_gameplay_economy::market::placement_fee(1, PRICE)
        .map_err(|error| format!("step=fee observed={error:?}"))?;
    let gold = fee + PRICE;
    let talk = super::client_row_e01::talk;
    talk(buyer, actor, "hi", None).await?;
    let quote = talk(buyer, actor, &format!("deposit {gold} gold"), None).await?;
    if !quote
        .lines
        .iter()
        .any(|line| line == &format!("Do you want to deposit {gold} gold?"))
    {
        return Err(format!("step=bank observed={:?}", quote.lines).into());
    }
    talk(buyer, actor, "yes", None).await?;
    Ok(())
}

async fn body(
    mut client: DevClientSession,
    buyer_scope: &SeamClients<'_>,
    seller_scope: &SeamClients<'_>,
    world: &BundleWorld,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult {
    let filter = wire::Filter::default();
    let wire::Reply::Catalogue {
        items,
        page: 0,
        has_next,
        ..
    } = ask(
        &mut client,
        &wire::Query::Catalogue {
            filter: filter.clone(),
            page: 0,
        },
    )
    .await?
    else {
        return Err("step=catalogue observed=not-a-catalogue".into());
    };
    let Some(first) = items.first() else {
        return Err("step=catalogue observed=empty needed=market-catalogue".into());
    };
    if items.len() > wire::PAGE_SIZE || !items.iter().all(|ware| filter.matches(ware)) {
        return Err(format!("step=catalogue observed=items:{}", items.len()).into());
    }
    let search = wire::Filter {
        search: search_term(&first.name),
        ..wire::Filter::default()
    };
    let wire::Reply::Catalogue {
        items: found,
        page: 0,
        ..
    } = ask(
        &mut client,
        &wire::Query::Catalogue {
            filter: search.clone(),
            page: 0,
        },
    )
    .await?
    else {
        return Err("step=search observed=not-a-catalogue".into());
    };
    if !found.iter().any(|ware| ware.definition == first.definition)
        || !found.iter().all(|ware| search.matches(ware))
    {
        return Err(format!("step=search observed=items:{}", found.len()).into());
    }
    let (id, definition) = catalogue_ware(&mut client, native, items, has_next).await?;
    evidence(&format!(
        "G24 ware={id} definition={definition} selection=lowest-inventory-ware"
    ));
    let mut client = settle(
        &mut client,
        buyer_scope,
        seller_scope,
        world,
        id,
        definition,
    )
    .await?;
    let wire::Reply::Statistics(statistics) =
        ask(&mut client, &wire::Query::Statistics { definition }).await?
    else {
        return Err("step=statistics observed=not-statistics".into());
    };
    if statistics.definition != definition {
        return Err(format!(
            "step=statistics observed=definition-{}",
            statistics.definition
        )
        .into());
    }
    let wire::Reply::Book {
        definition: echoed,
        page: 0,
        rows,
        ..
    } = ask(
        &mut client,
        &wire::Query::Book {
            definition,
            page: 0,
        },
    )
    .await?
    else {
        return Err("step=book observed=not-a-book".into());
    };
    if echoed != definition {
        return Err(format!("step=book observed=definition-{echoed}").into());
    }
    let trades = statistics.buy.trades + statistics.sell.trades;
    if trades != 1
        || statistics.trades != 1
        || statistics.units != 1
        || statistics.minimum != Some(PRICE)
        || statistics.maximum != Some(PRICE)
        || statistics.average != Some(PRICE)
        || !rows.is_empty()
    {
        return Err(format!(
            "step=statistics observed={statistics:?} book-rows={} needed=exact-settlement",
            rows.len()
        )
        .into());
    }
    finish(&mut client).await
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let world_id = WorldId::decode(&clients.world)?;
    let world = load_world(bytes, &pin, world_id, ChannelId::decode(&clients.channel)?)?;
    let input = crate::content::native_gameplay::NativeGameplayInput::from_manifest(Path::new(
        &required("NODE_BOOT_SPELL_MANIFEST")?,
    ))?;
    let room = crate::content::qualify_selected_native_gameplay_room(world_id, &input)?;
    let issuance = crate::content::NativeEntryActivationIssuance {
        world_id,
        activation_sequence: 1,
        server_artifact_digest: room.compiled().server_digest(),
        client_artifact_digest: room.compiled().client_digest(),
        frame_binding_digest: room.frame_binding().digest(),
    };
    let mut controller = crate::content::ContentActivationController::new();
    crate::content::activate_native_entry_room_with_gameplay(
        &mut controller,
        &crate::content::NodeBootQuiescence::before_channel_runtime(),
        world_id,
        &issuance,
        &input,
    )?;
    let native = controller
        .active()
        .and_then(|active| active.native_gameplay())
        .ok_or("step=content observed=missing-active-generation")?;

    let buyer = character_prep::dedicated_character(clients, "NODE_BOOT_G24_ACCOUNT").await?;
    let seller =
        character_prep::dedicated_character(clients, "NODE_BOOT_G24_SELLER_ACCOUNT").await?;
    let buyer_scope = buyer.clients(clients);
    let seller_scope = seller.clients(clients);
    character_prep::declare_seeded(
        "G24.full",
        "buyer gold and Premium, seller one selected ware",
        "market settlement is the mechanic under test",
    );
    let mut client = connect_character(&buyer_scope, 0x8a, 0).await?;
    character_prep::grant_item(&client, 3035, 1).await?;
    character_prep::grant_premium(&client).await?;
    finish(&mut client).await?;
    drop(client);
    let mut client = connect_character(&buyer_scope, 0x8a, 0).await?;
    fund_buyer(&mut client, world, &metadata.npcs).await?;
    at_depot(&mut client, world).await?;
    body(client, &buyer_scope, &seller_scope, world, native).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_is_one_bounded_token() {
        assert_eq!(
            compact("step=a  observed=b\nneeded=c"),
            "step=a-observed=b-needed=c"
        );
        assert!(!compact(&"x ".repeat(500)).contains(' '));
        assert!(compact(&"y".repeat(900)).len() <= 400);
    }

    #[test]
    fn search_term_is_a_short_lowercase_prefix() {
        assert_eq!(search_term("Magic Plate Armor"), "magi");
        assert_eq!(search_term("Ax"), "ax");
    }
}
