//! G09: pocket watch USE shows the world time (server package SRVPKG1, `novelty_item_use.rs`,
//! Crystal `watch.lua`).
//! 1. Admit the node-boot character and look for a carried watch (items 2906 and 6091).
//! 2. USE it by handle: expect an accepted `Committed` reply and a private "Game" chat line
//!    `The time is H:MM.` published by the server with an in-range Tibian minute.
//! 3. The watch is not consumed: the carried count is unchanged.
//! 4. Log out, freshly admit and use again: the line is published again (not a one-shot).
//!
//! The starter inventory carries no watch, so the row walks to an NPC seller named by the
//! committed trade content (item 2906), buys one through the real trade window, then uses it.
//! It never mints items; a failed purchase ends FAIL naming the precondition.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::{
    chat::ChatLine,
    world_object::{self, UseDisposition, UseIntent, UseTarget},
};
use std::collections::BTreeSet;

const WATCHES: [u32; 2] = [2906, 6091];

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(90), journey(clients)).await;
    let (status, detail) = match result {
        Ok(Ok(())) => ("PASS", "watch-time-line-and-not-consumed".to_owned()),
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
    let (hours, minutes): (u32, u32) = (hours.parse().ok()?, minutes.parse().ok()?);
    (minutes.to_string().len() <= 2 && hours < 24 && minutes < 60 && clock.len() >= 4)
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

/// Seller NPC keys (`oteryn:npc.<name>`) whose committed trade content sells `id` to players.
fn sellers_of(trades: &serde_json::Value, id: u32) -> BTreeSet<String> {
    let key = format!("oteryn:item.tibia.i{id}");
    let Some(records) = trades["records"].as_array() else {
        return BTreeSet::new();
    };
    records
        .iter()
        .filter_map(|record| {
            let declaration = &record["declaration"];
            declaration["offers"]
                .as_array()?
                .iter()
                .any(|offer| offer["direction"] == "SellToPlayer" && offer["item"]["key"] == key)
                .then(|| declaration["identity"]["key"].as_str())
                .flatten()?
                .strip_prefix("oteryn:service.trade.")
                .map(|name| format!("oteryn:npc.{name}"))
        })
        .collect()
}

/// Walk to the nearest NPC seller of item `id`, talk, and buy one through the real trade window.
async fn acquire(
    client: &mut DevClientSession,
    world: &BundleWorld,
    id: u32,
    definition: std::num::NonZeroU32,
) -> TestResult {
    use oteryn_dev_client::client_runtime::npc_shop as runtime;
    use oteryn_protocol_oteryn::npc_service as wire;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let trades: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/services/trade/trade-00000-00323.json"),
    )?)?;
    let sellers = sellers_of(&trades, id);
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let here = tile(client.world_spatial().actor_position)?;
    let (npc, path) = metadata
        .npcs
        .npcs
        .iter()
        .filter(|npc| sellers.contains(&npc.key))
        .filter_map(|npc| {
            route(world, here, |at| {
                npc.placements
                    .iter()
                    .any(|p| p.floor == at.floor && at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)) <= 3)
            })
            .map(|path| (npc, path))
        })
        .min_by_key(|(_, path)| path.len())
        .ok_or_else(|| format!("step=buy observed=no-walk-route-to-seller;item={id}"))?;
    walk(client, world, &path).await?;
    let actor = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entity| {
                entity.kind == oteryn_dev_client::EntityKind::Npc
                    && npc.placements.iter().any(|p| {
                        entity.position.x == i32::from(p.x)
                            && entity.position.y == i32::from(p.y)
                            && entity.position.floor == -i16::from(p.floor)
                    })
            })
        })
        .map(|entry| entry.entity)
        .ok_or("step=buy observed=seller-not-visible-after-walk")?;
    for text in ["hi", "trade"] {
        let reply = runtime::talk(
            client.session_mut(),
            &wire::NpcTalkIntent {
                npc_actor: actor,
                text: text.into(),
            },
        )
        .await?;
        if reply != wire::NpcIntentDisposition::Ok {
            return Err(format!("step=buy observed=npc-{text}:{reply:?}").into());
        }
    }
    let delta = client
        .session_mut()
        .read_gated_delta(wire::STATE_DOMAIN_NPC_TRADE_WINDOW, 1)
        .await?;
    let window = wire::decode_npc_trade_window(&delta.payload)
        .map_err(|error| format!("step=buy observed=window-decode:{error:?}"))?;
    let (offer_index, offer) = window
        .offers
        .iter()
        .enumerate()
        .find(|(_, offer)| {
            offer.side == wire::NpcTradeSide::Buy && offer.item_definition_ref == definition.get()
        })
        .ok_or("step=buy observed=live-seller-omitted-offer")?;
    let before = carried(client, definition).len();
    client.take_events();
    let reply = runtime::buy(
        client.session_mut(),
        &wire::NpcTradeIntent {
            npc_actor: actor,
            catalogue_revision: window.open.ok_or("step=buy observed=window-closed")?.1,
            offer_index: u32::try_from(offer_index)?,
            side: offer.side,
            quantity: 1,
            expected_unit_price: offer.unit_price,
        },
    )
    .await?;
    if reply != wire::NpcIntentDisposition::Ok {
        return Err(format!(
            "step=buy observed=reply:{reply:?};price={};needed=gold",
            offer.unit_price
        )
        .into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while carried(client, definition).len() <= before {
        if tokio::time::Instant::now() >= deadline {
            return Err("step=buy observed=no-inventory-delta".into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
        client.take_events();
    }
    Ok(())
}

fn carried(
    client: &DevClientSession,
    definition: std::num::NonZeroU32,
) -> Vec<oteryn_dev_client::ItemHandle> {
    client.inventory().map_or_else(Vec::new, |inventory| {
        inventory
            .main_backpack
            .iter()
            .chain(&inventory.entries)
            .filter(|entry| entry.item_definition_ref == definition)
            .map(|entry| entry.handle)
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
    let reply = client
        .session_mut()
        .gated_command(
            world_object::CAPABILITY_ITEM_USE_V1,
            world_object::COMMAND_TYPE_USE_INTENT,
            &payload,
        )
        .await?;
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
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let index = provisioning::item_index()?;
    let mut client = connect_world(clients, 0xcd).await?;
    let mut found = None;
    let mut observed = String::new();
    for id in WATCHES {
        let Ok(definition) = provisioning::item_reference(&index, id) else {
            continue;
        };
        if let Some(handle) = carried(&client, definition).first().copied() {
            found = Some((definition, handle, carried(&client, definition).len()));
            break;
        }
        let (status, disposition) =
            use_target(&mut client, UseTarget::ItemByDefinition(definition)).await?;
        observed.push_str(&format!("item={id}:{status:?}/{disposition:?};"));
    }
    if found.is_none() {
        // No carried watch: buy one from an NPC seller the committed trade content names.
        let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
        let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
        let world = load_world(
            bytes,
            &pin,
            WorldId::decode(&clients.world)?,
            ChannelId::decode(&clients.channel)?,
        )?;
        let definition = provisioning::item_reference(&index, WATCHES[0])?;
        if let Err(error) = acquire(&mut client, world, WATCHES[0], definition).await {
            let _ = logout(&mut client).await;
            return Err(format!("{error};{observed}needed=pocket-watch-from-npc-seller").into());
        }
        if let Some(handle) = carried(&client, definition).first().copied() {
            found = Some((definition, handle, carried(&client, definition).len()));
        }
    }
    let Some((definition, handle, count)) = found else {
        logout(&mut client).await?;
        return Err(format!("step=buy observed=no-watch-after-purchase;{observed}").into());
    };
    read_time(&mut client, handle).await?;
    if carried(&client, definition).len() != count {
        return Err("step=consume observed=watch-consumed;needed=clock-read-keeps-item".into());
    }
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_world(clients, 0xce).await?;
    let again = carried(&fresh, definition)
        .first()
        .copied()
        .ok_or("step=fresh observed=watch-missing-after-readmission")?;
    let result = read_time(&mut fresh, again).await;
    logout(&mut fresh).await?;
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sellers_of_reads_only_sell_to_player_offers() {
        let trades = serde_json::json!({"records":[{"declaration":{
            "identity":{"key":"oteryn:service.trade.zora"},
            "offers":[{"direction":"SellToPlayer","item":{"key":"oteryn:item.tibia.i2906"}}]}},
            {"declaration":{"identity":{"key":"oteryn:service.trade.bob"},
            "offers":[{"direction":"BuyFromPlayer","item":{"key":"oteryn:item.tibia.i2906"}}]}}]});
        let sellers = sellers_of(&trades, 2906);
        assert_eq!(sellers.len(), 1);
        assert!(sellers.contains("oteryn:npc.zora"));
    }

    #[test]
    fn time_line_is_parsed_only_for_the_server_format() {
        assert_eq!(time_line_minute("The time is 7:05."), Some(425));
        assert_eq!(time_line_minute("The time is 23:59."), Some(1439));
        assert_eq!(time_line_minute("The time is 24:00."), None);
        assert_eq!(time_line_minute("It is noon."), None);
        assert_eq!(token("a  b"), "a-b");
    }
}
