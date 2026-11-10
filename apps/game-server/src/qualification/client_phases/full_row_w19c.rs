//! W19c novelty items (fireworks, toys), derived from the novelty-item owner
//! (`gameplay_transport/novelty_item_owner.rs`, `items/src/novelty_items`).
//!
//! 1. Walk to the authored NPC seller (Stan) and buy one trumpet (6572, 80 gold) through the real
//!    talk and buy commands; assert the exact inventory credit.
//! 2. USE the trumpet by its server-issued handle; assert `Committed` and the source transform:
//!    the trumpet is replaced by the played trumpet (6573).
//! 3. Keep answering liveness; the server's four-second decay must restore the trumpet (6572).
//! 4. Log out, obtain a fresh admission and assert the durable quantities are unchanged.
//! 5. A purchase or use that the server refuses reports FAIL with the exact reply.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use std::collections::{BTreeMap, BTreeSet};

const TRUMPET: u32 = 6572;
const PLAYED_TRUMPET: u32 = 6573;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(180), journey(clients)).await;
    let (status, detail) = match result {
        Ok(Ok(note)) => ("PASS", compact(&note)),
        Ok(Err(error)) => ("FAIL", compact(&error.to_string())),
        Err(_) => ("FAIL", "step=journey-observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=W19c.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn compact(value: &str) -> String {
    let token: String = value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .take(400)
        .collect();
    if token.is_empty() {
        "unknown".to_owned()
    } else {
        token
    }
}

fn quantities(client: &DevClientSession) -> TestResult<BTreeMap<u32, u64>> {
    let mut values = BTreeMap::new();
    for item in &client
        .inventory()
        .ok_or("inventory snapshot absent")?
        .entries
    {
        *values.entry(item.item_definition_ref.get()).or_default() += u64::from(item.count.get());
    }
    Ok(values)
}

fn quantity(client: &DevClientSession, reference: std::num::NonZeroU32) -> TestResult<u64> {
    Ok(*quantities(client)?.get(&reference.get()).unwrap_or(&0))
}

async fn wait_quantity(
    client: &mut DevClientSession,
    reference: std::num::NonZeroU32,
    expected: u64,
) -> TestResult<u64> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        if quantity(client, reference)? == expected {
            return Ok(0);
        }
        for event in client.take_events() {
            if let SessionEvent::Inventory(delta) = event
                && quantity(client, reference)? == expected
            {
                return Ok(delta.server_sequence);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "inventory delta timeout definition={reference} expected={expected} observed={}",
                quantity(client, reference)?
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }
}

async fn acquire(
    client: &mut DevClientSession,
    world: &BundleWorld,
    id: u32,
    amount: u32,
) -> TestResult<u64> {
    use oteryn_dev_client::client_runtime::npc_shop as runtime;
    use oteryn_protocol_oteryn::npc_service as wire;
    let index = provisioning::item_index()?;
    let reference = provisioning::item_reference(&index, id)?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let trades: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/services/trade/trade-00000-00323.json"),
    )?)?;
    let key = format!("oteryn:item.tibia.i{id}");
    let sellers: BTreeSet<_> = trades["records"]
        .as_array()
        .ok_or("trade records absent")?
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
        .collect();
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
        .ok_or_else(|| format!("no walk route to seller for item {id}; sellers={sellers:?}"))?;
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
        .ok_or("seller not visible after walk")?;
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
            return Err(format!("NPC {text} reply={reply:?}").into());
        }
    }
    let delta = client
        .session_mut()
        .read_gated_delta(wire::STATE_DOMAIN_NPC_TRADE_WINDOW, 1)
        .await?;
    let window = wire::decode_npc_trade_window(&delta.payload)
        .map_err(|error| format!("trade window decode: {error:?}"))?;
    let (offer_index, offer) = window
        .offers
        .iter()
        .enumerate()
        .find(|(_, offer)| {
            offer.side == wire::NpcTradeSide::Buy && offer.item_definition_ref == reference.get()
        })
        .ok_or("live seller omitted requested offer")?;
    let before = quantity(client, reference)?;
    client.take_events();
    let reply = runtime::buy(
        client.session_mut(),
        &wire::NpcTradeIntent {
            npc_actor: actor,
            catalogue_revision: window.open.ok_or("trade window closed")?.1,
            offer_index: u32::try_from(offer_index)?,
            side: offer.side,
            quantity: amount,
            expected_unit_price: offer.unit_price,
        },
    )
    .await?;
    if reply != wire::NpcIntentDisposition::Ok {
        return Err(format!(
            "buy item={id} reply={reply:?} price={} quantity={amount}",
            offer.unit_price
        )
        .into());
    }
    let expected = before + u64::from(offer.count) * u64::from(amount);
    wait_quantity(client, reference, expected).await
}

async fn use_one(
    client: &mut DevClientSession,
    reference: std::num::NonZeroU32,
) -> TestResult<(UseDisposition, u64)> {
    let before = quantity(client, reference)?;
    let handle = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|item| item.item_definition_ref == reference)
        })
        .map(|item| item.handle)
        .ok_or("purchased item handle absent")?;
    client.take_events();
    let reply = client.use_item(handle).await?;
    if reply.disposition == UseDisposition::Committed {
        wait_quantity(client, reference, before - 1).await?;
    }
    Ok((reply.disposition, before))
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("logout refused".into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let index = provisioning::item_index()?;
    let trumpet = provisioning::item_reference(&index, TRUMPET)?;
    let played = provisioning::item_reference(&index, PLAYED_TRUMPET)?;
    let mut client = connect_world(clients, 0xe6).await?;
    let trumpet_base = quantity(&client, trumpet)?;
    let played_base = quantity(&client, played)?;
    let bought = acquire(&mut client, world, TRUMPET, 1).await?;
    let (reply, _) = use_one(&mut client, trumpet).await?;
    if reply != UseDisposition::Committed {
        return Err(format!("step=use observed=reply-{reply:?}").into());
    }
    // Source debit and transform credit, then the decay: restored trumpet back, played one gone.
    wait_quantity(&mut client, played, played_base + 1).await?;
    wait_quantity(&mut client, trumpet, trumpet_base + 1).await?;
    wait_quantity(&mut client, played, played_base).await?;
    let durable = quantities(&client)?;
    let session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    let mut client = connect_world(clients, 0xe6).await?;
    if client.join_snapshot().game_session_id == session
        || quantities(&client)? != durable
        || quantity(&client, trumpet)? != trumpet_base + 1
        || quantity(&client, played)? != played_base
    {
        return Err("step=fresh-admission observed=durable-quantities-changed".into());
    }
    logout(&mut client).await?;
    Ok(format!(
        "trumpet-bought-sequence-{bought}-played-then-decayed-durable"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_is_one_token() {
        assert_eq!(compact("a  b"), "a-b");
        assert_eq!(compact(" "), "unknown");
    }

    #[test]
    fn the_trumpet_ids_are_the_novelty_rule_pair() {
        assert_eq!((TRUMPET, PLAYED_TRUMPET), (6572, 6573));
    }
}
