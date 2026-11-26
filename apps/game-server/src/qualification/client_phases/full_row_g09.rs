//! G09: pocket watch USE shows the world time (server package SRVPKG1, `novelty_item_use.rs`,
//! Crystal `watch.lua`).
//! 1. Admit the dedicated character, walk to Lubo and buy one watch (item 2906).
//! 2. USE it by handle: expect an accepted `Committed` reply and a private "Game" chat line
//!    `The time is H:MM.` published by the server matching the World clock during the command.
//! 3. The watch is not consumed: the carried count is unchanged.
//! 4. Log out, freshly admit and use again: require a new line with the advanced World time.
//!
//! The fixture adds 19 gold toward Crystal Lubo's 20-gold price. The purchase
//! leaves one fewer coin than the character's observed starter balance.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::{
    chat::ChatLine,
    npc_service as npc,
    world_object::{self, UseDisposition, UseIntent, UseTarget},
};
const GOLD: u32 = 3031;
const WATCH: u32 = 2906;
const PRICE: u32 = 20;
const PURCHASE_FUNDING: u32 = PRICE - 1;
// Pinned Lubo behavior in content/behaviors/behaviors-02500-02999.json.
const LUBO_WANDER_RADIUS: u16 = 2;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    // The cardinal walk from Thais to Lubo may take several minutes at the
    // server's ordinary step pace. Each step still has to commit on the first send.
    let result =
        verdict::indexed_timeout(Duration::from_secs(600), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = match result {
        Ok(Ok(())) => (
            "PASS",
            "lubo-purchase-watch-time-line-and-not-consumed".to_owned(),
        ),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G09.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

/// Minute of day from a `The time is H:MM.` line, or `None` for any other text.
fn time_line_minute(text: &str) -> Option<u32> {
    let clock = text.strip_prefix("The time is ")?.strip_suffix('.')?;
    let (hours, minutes) = clock.split_once(':')?;
    if minutes.len() != 2
        || !minutes.bytes().all(|byte| byte.is_ascii_digit())
        || !hours.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let (hours, minutes): (u32, u32) = (hours.parse().ok()?, minutes.parse().ok()?);
    (hours < 24 && minutes < 60 && clock == format!("{hours}:{minutes:02}"))
        .then_some(hours * 60 + minutes)
}

fn clock_tick() -> TestResult<u64> {
    use crate::world_clock::{DEFAULT_WORLD_CLOCK_EPOCH_UTC_MS, TIBIAN_MINUTE_MS, now_utc_ms};
    let now = now_utc_ms().ok_or("step=clock observed=utc-unavailable")?;
    Ok(now
        .checked_sub(DEFAULT_WORLD_CLOCK_EPOCH_UTC_MS)
        .ok_or("step=clock observed=before-world-epoch")?
        / TIBIAN_MINUTE_MS)
}

fn minute_in_command(minute: u32, first: u64, last: u64) -> bool {
    let day = u64::from(oteryn_protocol_oteryn::world_clock::TIBIAN_MINUTES_PER_DAY);
    last.checked_sub(first).is_some_and(|span| span <= 3)
        && (first..=last).any(|tick| tick % day == u64::from(minute))
}

fn carried(
    client: &DevClientSession,
    definition: std::num::NonZeroU32,
) -> Vec<(oteryn_dev_client::ItemHandle, u32)> {
    client.inventory().map_or_else(Vec::new, |inventory| {
        inventory
            .main_backpack
            .iter()
            .chain(&inventory.entries)
            .filter(|entry| entry.item_definition_ref == definition)
            .map(|entry| (entry.handle, entry.count.get()))
            .collect()
    })
}

async fn use_target(
    client: &mut DevClientSession,
    target: UseTarget,
) -> TestResult<(CommandStatus, UseDisposition)> {
    let payload = world_object::encode_item_use_intent(&UseIntent {
        target,
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(|error| format!("step=encode observed={error:?}"))?;
    let reply = client.session_mut().world_context_use(&payload).await?;
    let disposition = world_object::decode_use_result(&reply.payload)
        .map_err(|error| format!("step=use observed=reply-decode:{error:?}"))?;
    Ok((reply.status, disposition))
}

async fn read_time(
    client: &mut DevClientSession,
    handle: oteryn_dev_client::ItemHandle,
) -> TestResult<u32> {
    let before = client.last_server_sequence();
    client.take_events();
    let first = clock_tick()?;
    let (status, disposition) = use_target(client, UseTarget::Item(handle)).await?;
    let last = clock_tick()?;
    if status != CommandStatus::Accepted || disposition != UseDisposition::Committed {
        return Err(
            format!("step=use observed=status:{status:?},disposition:{disposition:?}").into(),
        );
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        for event in client.take_events() {
            if let oteryn_dev_client::SessionEvent::ChatLine(delta) = event
                && let ChatLine::Private { speaker_name, text } = delta.value
                && speaker_name == "Game"
                && text.starts_with("The time is ")
            {
                let minute =
                    time_line_minute(&text).ok_or("step=chat observed=invalid-clock-format")?;
                if delta.server_sequence <= before || !minute_in_command(minute, first, last) {
                    return Err(format!(
                        "step=chat observed=minute:{minute},sequence:{} expected=command-ticks:{first}..={last},sequence-after:{before}",
                        delta.server_sequence
                    ).into());
                }
                evidence(&format!(
                    "G09_CHECK step=clock status=PASS minute={minute} sequence={}",
                    delta.server_sequence
                ));
                return Ok(minute);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step=chat observed=no-server-time-line;needed=The-time-is-H:MM".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let result = client.session_mut().logout().await?.result;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=logout observed={result:?}").into());
    }
    Ok(())
}

fn within(a: TilePos, b: TilePos, range: u32) -> bool {
    a.floor == b.floor && u32::from(a.x.abs_diff(b.x).max(a.y.abs_diff(b.y))) <= range
}

fn gold_balance(client: &DevClientSession, gold: std::num::NonZeroU32) -> u64 {
    carried(client, gold)
        .into_iter()
        .map(|(_, count)| u64::from(count))
        .sum()
}

async fn npc_command(
    client: &mut DevClientSession,
    command: u32,
    payload: &[u8],
) -> TestResult<u64> {
    let reply = client
        .session_mut()
        .gated_command(npc::CAPABILITY_NPC_SERVICE_V1, command, payload)
        .await?;
    let disposition = npc::decode_npc_intent_result(&reply.payload)
        .map_err(|error| format!("step=npc command={command} reply-decode={error:?}"))?;
    if reply.status != CommandStatus::Accepted || disposition != npc::NpcIntentDisposition::Ok {
        return Err(format!(
            "step=npc command={command} status={:?} disposition={disposition:?}",
            reply.status
        )
        .into());
    }
    Ok(reply.server_sequence)
}

async fn talk(
    client: &mut DevClientSession,
    actor: oteryn_dev_client::EntityRef,
    text: &str,
) -> TestResult<u64> {
    let payload = npc::encode_npc_talk_intent(&npc::NpcTalkIntent {
        npc_actor: actor,
        text: text.into(),
    })
    .map_err(|error| format!("step=talk-{text} encode={error:?}"))?;
    npc_command(client, npc::COMMAND_TYPE_NPC_TALK_INTENT, &payload).await
}

async fn buy_watch(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    watch: std::num::NonZeroU32,
    gold: std::num::NonZeroU32,
    funded_gold: u64,
    remaining_gold: u64,
) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let bundle = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let seller = bundle
        .npcs
        .npcs
        .iter()
        .find(|seller| seller.key == "oteryn:npc.lubo")
        .ok_or("step=seller observed=lubo-missing-from-bundle")?;
    let [placement] = seller.placements.as_slice() else {
        return Err("step=seller observed=expected-one-lubo-placement".into());
    };
    let home = TilePos {
        x: placement.x,
        y: placement.y,
        floor: placement.floor,
    };
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let here = tile(client.world_spatial().actor_position)?;
    // Keep the original three-tile approach. The live actor check below uses
    // Lubo's actual position, which can differ from the authored placement.
    let path = route(world, here, |at| at != home && within(at, home, 3))
        .ok_or("step=seller observed=no-route-to-lubo")?;
    evidence(&format!(
        "G09_ROUTE from={here:?} seller_home={home:?} steps={}",
        path.len()
    ));
    for direction in path {
        let before = tile(client.world_spatial().actor_position)?;
        let outcome = client.step(direction).await?;
        let after = tile(client.world_spatial().actor_position)?;
        if outcome.disposition != StepDisposition::Moved
            || outcome.world_spatial_delta.is_none()
            || destination(world, before, direction) != Some(after)
        {
            return Err(format!(
                "step=walk from={before:?} direction={direction:?} to={after:?} observed={:?}",
                outcome.disposition
            )
            .into());
        }
        verdict::Harness::global().progress("walk");
        client.take_events();
        if outcome.step_wait_ms != 0 {
            client
                .service_liveness(Duration::from_millis(u64::from(outcome.step_wait_ms)))
                .await?;
        }
    }
    let here = tile(client.world_spatial().actor_position)?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let actor = loop {
        let seller = {
            let mut visible = client
                .world_entities()
                .into_iter()
                .flat_map(|entities| entities.others())
                .filter(|entry| {
                    entry.kind == oteryn_dev_client::EntityKind::Npc
                        && entry
                            .display_name
                            .is_some_and(|name| name.as_str() == Ok("Lubo"))
                        && tile(entry.position).is_ok_and(|at| {
                            within(at, home, u32::from(LUBO_WANDER_RADIUS))
                                && within(at, here, npc::TALK_RANGE_TILES)
                        })
                });
            let first = visible.next();
            if visible.next().is_some() {
                return Err("step=seller observed=ambiguous-lubo-identity".into());
            }
            first.map(|entry| (entry.entity, entry.position))
        };
        if let Some((actor, position)) = seller {
            evidence(&format!(
                "G09_CHECK step=seller status=PASS position={position:?}"
            ));
            break actor;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(
                format!("step=seller observed=lubo-not-visible-after-walk at={here:?}").into(),
            );
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    };

    let conversation = client
        .session_mut()
        .gated_snapshot(npc::STATE_DOMAIN_NPC_CONVERSATION)
        .ok_or("step=seller observed=missing-conversation-snapshot")?;
    let initial = npc::decode_npc_conversation(&conversation.payload)
        .map_err(|error| format!("step=seller observed=conversation-snapshot-decode:{error:?}"))?;
    let details = client
        .selected_capabilities()
        .contains(&oteryn_protocol_oteryn::trade_details::CAPABILITY_TRADE_DETAILS_V1);
    let window = client
        .session_mut()
        .gated_snapshot(npc::STATE_DOMAIN_NPC_TRADE_WINDOW)
        .ok_or("step=seller observed=missing-trade-snapshot")?;
    let initial_window = npc::decode_npc_trade_window_for(&window.payload, details)
        .map_err(|error| format!("step=seller observed=trade-snapshot-decode:{error:?}"))?;
    if initial.open.is_some() || initial_window.open.is_some() {
        return Err("step=seller observed=conversation-or-trade-already-open".into());
    }
    let greeting = talk(client, actor, "hi").await?;
    let delta = client
        .session_mut()
        .read_gated_delta(npc::STATE_DOMAIN_NPC_CONVERSATION, 1)
        .await?;
    let conversation = npc::decode_npc_conversation(&delta.payload)
        .map_err(|error| format!("step=seller observed=conversation-delta-decode:{error:?}"))?;
    if delta.server_sequence <= greeting
        || conversation.open.map(|open| open.0) != Some(actor)
        || conversation.lines.iter().all(|line| line.trim().is_empty())
    {
        return Err("step=seller observed=missing-new-lubo-greeting".into());
    }
    let trade = talk(client, actor, "trade").await?;
    let delta = client
        .session_mut()
        .read_gated_delta(npc::STATE_DOMAIN_NPC_TRADE_WINDOW, 1)
        .await?;
    let window = npc::decode_npc_trade_window_for(&delta.payload, details)
        .map_err(|error| format!("step=buy observed=window-decode:{error:?}"))?;
    let (seller, revision) = window.open.ok_or("step=buy observed=trade-window-closed")?;
    if delta.server_sequence <= trade || seller != actor {
        return Err("step=buy observed=trade-window-for-other-seller".into());
    }
    let (offer_index, offer) = window
        .offers
        .iter()
        .enumerate()
        .find(|(_, offer)| {
            offer.side == npc::NpcTradeSide::Buy && offer.item_definition_ref == watch.get()
        })
        .ok_or("step=buy observed=lubo-omitted-watch-offer")?;
    if offer.unit_price != PRICE || offer.count != 1 {
        return Err(format!(
            "step=buy observed=price:{},quantity:{}",
            offer.unit_price, offer.count
        )
        .into());
    }
    if !carried(client, watch).is_empty() || gold_balance(client, gold) != funded_gold {
        return Err("step=buy observed=inventory-changed-before-purchase".into());
    }
    client.take_events();
    let payload = npc::encode_npc_trade_intent(&npc::NpcTradeIntent {
        npc_actor: actor,
        catalogue_revision: revision,
        offer_index: u32::try_from(offer_index)?,
        side: offer.side,
        quantity: 1,
        expected_unit_price: offer.unit_price,
    })
    .map_err(|error| format!("step=buy encode={error:?}"))?;
    let purchase = npc_command(client, npc::COMMAND_TYPE_NPC_TRADE_INTENT, &payload).await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if client
            .take_events()
            .into_iter()
            .any(|event| matches!(event, oteryn_dev_client::SessionEvent::Inventory(delta) if delta.server_sequence > purchase))
            && matches!(carried(client, watch).as_slice(), [(_, 1)])
            && gold_balance(client, gold) == remaining_gold
        {
            evidence(&format!(
                "G09_CHECK step=buy status=PASS watch={WATCH} price={PRICE} gold_before={funded_gold} gold_after={remaining_gold}"
            ));
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step=buy observed=missing-exact-watch-credit-and-gold-debit".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    character_prep::declare_seeded(
        "G09",
        &format!("{PURCHASE_FUNDING} gold"),
        "Lubo's authored watch price",
    );
    let identity = character_prep::dedicated_character(clients, "NODE_BOOT_G09_ACCOUNT").await?;
    let clients = &identity.clients(clients);
    let index = provisioning::item_index()?;
    let definition = provisioning::item_reference(&index, WATCH)?;
    let gold = provisioning::item_reference(&index, GOLD)?;
    let mut client = connect_character(clients, 0xcd, 0).await?;
    if !carried(&client, definition).is_empty() {
        return Err("step=fixture observed=watch-already-present".into());
    }
    let starter_gold = gold_balance(&client, gold);
    let funded_gold = starter_gold + u64::from(PURCHASE_FUNDING);
    let remaining_gold = funded_gold
        .checked_sub(u64::from(PRICE))
        .ok_or("step=fixture observed=no-starter-coin-for-purchase")?;
    character_prep::grant_item(&client, GOLD, PURCHASE_FUNDING).await?;
    logout(&mut client).await?;
    drop(client);

    let mut client = connect_character(clients, 0xce, 0).await?;
    if !carried(&client, definition).is_empty() || gold_balance(&client, gold) != funded_gold {
        return Err("step=fixture observed=expected-only-purchase-gold".into());
    }
    buy_watch(
        &mut client,
        clients,
        definition,
        gold,
        funded_gold,
        remaining_gold,
    )
    .await?;
    let inventory = carried(&client, definition);
    let [(handle, 1)] = inventory.as_slice() else {
        return Err("step=fixture observed=expected-exactly-one-served-watch".into());
    };
    let handle = *handle;
    let first_minute = read_time(&mut client, handle).await?;
    if carried(&client, definition) != inventory {
        return Err("step=consume observed=watch-consumed;needed=clock-read-keeps-item".into());
    }
    let previous_session = client.join_snapshot().game_session_id;
    let purchased_at = client.world_spatial().actor_position;
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_character(clients, 0xcf, 0).await?;
    if fresh.join_snapshot().game_session_id == previous_session
        || fresh.world_spatial().actor_position != purchased_at
        || gold_balance(&fresh, gold) != remaining_gold
    {
        return Err("step=fresh observed=stale-session-or-position-or-gold-debit-lost".into());
    }
    let durable_inventory = carried(&fresh, definition);
    let [(again, 1)] = durable_inventory.as_slice() else {
        return Err("step=fresh observed=expected-exactly-one-durable-watch".into());
    };
    let second_minute = read_time(&mut fresh, *again)
        .await
        .map_err(|error| verdict::context("step=fresh-clock".into(), error))?;
    let after = carried(&fresh, definition);
    logout(&mut fresh).await?;
    if second_minute == first_minute {
        return Err("step=fresh observed=clock-did-not-advance".into());
    }
    if after != durable_inventory {
        return Err("step=fresh observed=watch-consumed-on-second-read".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lubo_wander_and_talk_range_are_independent() {
        let home = TilePos {
            x: 10,
            y: 10,
            floor: -7,
        };
        let player = TilePos {
            x: home.x - 3,
            ..home
        };
        let moved = TilePos {
            x: home.x + 1,
            ..home
        };
        let furthest = TilePos {
            x: home.x + LUBO_WANDER_RADIUS,
            ..home
        };
        let wander = u32::from(LUBO_WANDER_RADIUS);
        assert!(within(moved, home, wander));
        assert!(within(moved, player, npc::TALK_RANGE_TILES));
        assert!(within(furthest, home, wander));
        assert!(!within(furthest, player, npc::TALK_RANGE_TILES));
        assert!(!within(
            TilePos {
                floor: home.floor - 1,
                ..moved
            },
            home,
            wander
        ));
    }

    #[test]
    fn clock_match_is_bounded_and_wraps_at_midnight() {
        assert!(minute_in_command(1439, 1439, 1440));
        assert!(minute_in_command(0, 1439, 1440));
        assert!(!minute_in_command(1, 1439, 1440));
        assert!(!minute_in_command(0, 1439, 1443));
        assert!(!minute_in_command(0, 1440, 1439));
    }

    #[test]
    fn time_line_is_parsed_only_for_the_server_format() {
        assert_eq!(time_line_minute("The time is 7:05."), Some(425));
        assert_eq!(time_line_minute("The time is 23:59."), Some(1439));
        assert_eq!(time_line_minute("The time is 24:00."), None);
        assert_eq!(time_line_minute("The time is 7:5."), None);
        assert_eq!(time_line_minute("The time is 07:05."), None);
        assert_eq!(time_line_minute("It is noon."), None);
        assert_eq!(token("a  b"), "a-b");
    }
}
