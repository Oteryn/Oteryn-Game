//! G09: pocket watch USE shows the world time (server package SRVPKG1, `novelty_item_use.rs`,
//! Crystal `watch.lua`).
//! 1. Admit the dedicated character, walk to Lubo and buy one watch (item 2906).
//! 2. USE it by handle: expect an accepted `Committed` reply and a private "Game" chat line
//!    `The time is H:MM.` published by the server with an in-range Tibian minute.
//! 3. The watch is not consumed: the carried count is unchanged.
//! 4. Log out, freshly admit and use again: the line is published again (not a one-shot).
//!
//! The fixture supplies the 20-gold purchase price from Crystal's `data-global/npc/lubo.lua`.
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
// Pinned Lubo behavior in content/behaviors/behaviors-02500-02999.json.
const LUBO_WANDER_RADIUS: u16 = 2;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
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

fn game_time_lines(client: &DevClientSession) -> usize {
    client.chat_log().map_or(0, |log| {
        log.lines()
            .filter(|line| {
                matches!(line, ChatLine::Private { speaker_name, text }
                    if speaker_name == "Game" && time_line_minute(text).is_some())
            })
            .count()
    })
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
) -> TestResult {
    let before = game_time_lines(client);
    let (status, disposition) = use_target(client, UseTarget::Item(handle)).await?;
    if status != CommandStatus::Accepted || disposition != UseDisposition::Committed {
        return Err(
            format!("step=use observed=status:{status:?},disposition:{disposition:?}").into(),
        );
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while game_time_lines(client) <= before {
        if tokio::time::Instant::now() >= deadline {
            return Err("step=chat observed=no-server-time-line;needed=The-time-is-H:MM".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    Ok(())
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

async fn buy_watch(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    watch: std::num::NonZeroU32,
    gold: std::num::NonZeroU32,
    starter_gold: u64,
) -> TestResult {
    use oteryn_dev_client::client_runtime::npc_shop;
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
    let path = route(world, here, |at| {
        at != home
            && within(
                at,
                home,
                npc::TALK_RANGE_TILES - u32::from(LUBO_WANDER_RADIUS),
            )
    })
    .ok_or("step=seller observed=no-route-to-lubo")?;
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

    for text in ["hi", "trade"] {
        let reply = npc_shop::talk(
            client.session_mut(),
            &npc::NpcTalkIntent {
                npc_actor: actor,
                text: text.into(),
            },
        )
        .await?;
        if reply != npc::NpcIntentDisposition::Ok {
            return Err(format!("step=seller observed=lubo-{text}:{reply:?}").into());
        }
    }
    let details = client
        .selected_capabilities()
        .contains(&oteryn_protocol_oteryn::trade_details::CAPABILITY_TRADE_DETAILS_V1);
    let delta = client
        .session_mut()
        .read_gated_delta(npc::STATE_DOMAIN_NPC_TRADE_WINDOW, 1)
        .await?;
    let window = npc::decode_npc_trade_window_for(&delta.payload, details)
        .map_err(|error| format!("step=buy observed=window-decode:{error:?}"))?;
    let (seller, revision) = window.open.ok_or("step=buy observed=trade-window-closed")?;
    if seller != actor {
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
    if !carried(client, watch).is_empty()
        || gold_balance(client, gold) != starter_gold + u64::from(PRICE)
    {
        return Err("step=buy observed=inventory-changed-before-purchase".into());
    }
    client.take_events();
    let reply = npc_shop::buy(
        client.session_mut(),
        &npc::NpcTradeIntent {
            npc_actor: actor,
            catalogue_revision: revision,
            offer_index: u32::try_from(offer_index)?,
            side: offer.side,
            quantity: 1,
            expected_unit_price: offer.unit_price,
        },
    )
    .await?;
    if reply != npc::NpcIntentDisposition::Ok {
        return Err(format!("step=buy observed=lubo-buy:{reply:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if client
            .take_events()
            .into_iter()
            .any(|event| matches!(event, oteryn_dev_client::SessionEvent::Inventory(_)))
            && matches!(carried(client, watch).as_slice(), [(_, 1)])
            && gold_balance(client, gold) == starter_gold
        {
            evidence(&format!(
                "G09_CHECK step=buy status=PASS watch={WATCH} price={PRICE}"
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
        &format!("{PRICE} gold"),
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
    character_prep::grant_item(&client, GOLD, PRICE).await?;
    logout(&mut client).await?;
    drop(client);

    let mut client = connect_character(clients, 0xce, 0).await?;
    if !carried(&client, definition).is_empty()
        || gold_balance(&client, gold) != starter_gold + u64::from(PRICE)
    {
        return Err("step=fixture observed=expected-only-purchase-gold".into());
    }
    buy_watch(&mut client, clients, definition, gold, starter_gold).await?;
    let inventory = carried(&client, definition);
    let [(handle, 1)] = inventory.as_slice() else {
        return Err("step=fixture observed=expected-exactly-one-served-watch".into());
    };
    let handle = *handle;
    read_time(&mut client, handle).await?;
    if carried(&client, definition) != inventory {
        return Err("step=consume observed=watch-consumed;needed=clock-read-keeps-item".into());
    }
    let previous_session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_character(clients, 0xcf, 0).await?;
    if fresh.join_snapshot().game_session_id == previous_session
        || gold_balance(&fresh, gold) != starter_gold
    {
        return Err("step=fresh observed=stale-session-or-gold-debit-lost".into());
    }
    let durable_inventory = carried(&fresh, definition);
    let [(again, 1)] = durable_inventory.as_slice() else {
        return Err("step=fresh observed=expected-exactly-one-durable-watch".into());
    };
    let result = verdict::observe(read_time(&mut fresh, *again)).await;
    let after = carried(&fresh, definition);
    logout(&mut fresh).await?;
    result?;
    if after != durable_inventory {
        return Err("step=fresh observed=watch-consumed-on-second-read".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lubo_approach_covers_the_wander_radius_on_the_same_floor() {
        let home = TilePos {
            x: 10,
            y: 10,
            floor: -7,
        };
        let wander = u32::from(LUBO_WANDER_RADIUS);
        let approach = npc::TALK_RANGE_TILES - wander;
        let player = TilePos {
            x: home.x - approach as u16,
            ..home
        };
        let furthest = TilePos {
            x: home.x + LUBO_WANDER_RADIUS,
            y: home.y + LUBO_WANDER_RADIUS,
            ..home
        };
        assert!(within(furthest, home, wander));
        assert!(within(furthest, player, npc::TALK_RANGE_TILES));
        assert!(!within(
            TilePos {
                x: furthest.x + 1,
                ..furthest
            },
            home,
            wander
        ));
        assert!(!within(
            TilePos {
                floor: home.floor - 1,
                ..furthest
            },
            home,
            wander
        ));
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
