//! I17c: hotkey equip, automatic two-hand/shield swap and ammunition into the quiver.
//! Buys a wooden shield (3412) and a two-handed Espada (3265) and equips the shield, then the
//! two-hander: the shield must return to the backpack and the weapon must be worn. Then buys a
//! quiver (35562) and arrows (3447): arrows before a quiver must be refused `SlotMismatch` with
//! no change, after the quiver is worn the arrows must leave the backpack for the quiver.
use super::*;
use oteryn_dev_client::DevClientSession;
use sqlx::Connection;
use std::collections::{BTreeMap, BTreeSet};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(600), journey(clients)).await;
    let (status, detail) = match result {
        Ok(Ok(detail)) => ("PASS", token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=I17c.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

fn quantities(client: &DevClientSession) -> TestResult<BTreeMap<u32, u64>> {
    let inventory = client.inventory().ok_or("inventory snapshot absent")?;
    Ok(inventory_quantities(inventory))
}

fn inventory_quantities(inventory: &oteryn_dev_client::CharacterInventory) -> BTreeMap<u32, u64> {
    let mut values = BTreeMap::new();
    for item in &inventory.entries {
        *values.entry(item.item_definition_ref.get()).or_default() += u64::from(item.count.get());
    }
    values
}

fn quantity(client: &DevClientSession, reference: std::num::NonZeroU32) -> TestResult<u64> {
    Ok(*quantities(client)?.get(&reference.get()).unwrap_or(&0))
}

async fn wait_quantity(
    client: &mut DevClientSession,
    reference: std::num::NonZeroU32,
    expected: u64,
) -> TestResult<u64> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        // The expected state may already be in the snapshot: `settle` drained its event.
        client.take_events();
        if quantity(client, reference)? == expected {
            return Ok(client.last_server_sequence());
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

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("logout refused".into());
    }
    Ok(())
}

async fn open_world(clients: &SeamClients<'_>) -> TestResult<&'static BundleWorld> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )
}

fn reference(id: u32) -> TestResult<std::num::NonZeroU32> {
    let index = provisioning::item_index()?;
    provisioning::item_reference(&index, id)
}

fn worn(client: &DevClientSession, definition: std::num::NonZeroU32) -> Option<String> {
    client
        .inventory()?
        .equipment
        .iter()
        .find(|entry| entry.item.item_definition_ref == definition)
        .map(|entry| format!("{:?}", entry.slot))
}

async fn settle(
    client: &mut DevClientSession,
    definition: std::num::NonZeroU32,
    want_worn: bool,
) -> TestResult<()> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(6);
    loop {
        client.take_events();
        if worn(client, definition).is_some() == want_worn {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "step=settle observed=worn:{:?};wanted={want_worn}",
                worn(client, definition)
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}

async fn hotkey(
    client: &mut DevClientSession,
    definition: std::num::NonZeroU32,
) -> TestResult<oteryn_dev_client::ItemMoveOutcome> {
    client.take_events();
    Ok(
        oteryn_dev_client::client_runtime::hotkey_equip::equip_definition(
            client.session_mut(),
            definition,
        )
        .await?,
    )
}

async fn expect(
    client: &mut DevClientSession,
    definition: std::num::NonZeroU32,
    step: &str,
    want: oteryn_dev_client::ItemMoveOutcome,
) -> TestResult<()> {
    let outcome = hotkey(client, definition).await?;
    if outcome != want {
        return Err(format!("step={step} observed={outcome:?};needed={want:?}").into());
    }
    Ok(())
}

async fn buy(
    client: &mut DevClientSession,
    world: &BundleWorld,
    id: u32,
    amount: u32,
) -> TestResult<()> {
    acquire(client, world, id, amount)
        .await
        .map(|_| ())
        .map_err(|error| {
            format!("step=buy-{id} observed={error};needed:player-gold-and-reachable-seller").into()
        })
}

/// Arrows held by quivers (read-only): the quiver contents are not part of the client snapshot.
async fn quiver_arrows(clients: &SeamClients<'_>) -> TestResult<i64> {
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let total: i64 = sqlx::query_scalar(
        "SELECT COALESCE(sum(a.quantity),0)::bigint FROM game_item_nested_entries e JOIN game_item_instances a ON a.item_instance_id=e.item_instance_id JOIN game_item_instances q ON q.item_instance_id=e.parent_item_instance_id WHERE a.definition_production_key='oteryn:item.tibia.i3447' AND a.lifecycle=1 AND q.definition_production_key='oteryn:item.tibia.i35562' AND q.lifecycle=1 AND EXISTS (SELECT 1 FROM game_character_equipment_slots s WHERE s.item_instance_id=q.item_instance_id AND s.character_id=encode($1,'hex')::uuid AND s.world_id=encode($2,'hex')::uuid AND s.slot=6)",
    )
    .bind(clients.characters[1].as_slice())
    .bind(clients.world.as_slice())
    .fetch_one(&mut connection)
    .await?;
    Ok(total)
}

async fn relog(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    tag: u8,
) -> TestResult<DevClientSession> {
    logout(client).await?;
    connect_world(clients, tag).await
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let world = open_world(clients).await?;
    use oteryn_dev_client::ItemMoveOutcome as Out;
    let shield = reference(3412)?;
    let sword = reference(3265)?;
    let quiver = reference(35562)?;
    let arrow = reference(3447)?;
    let mut client = connect_world(clients, 0xe3).await?;
    let shield_base = quantity(&client, shield)?;
    buy(&mut client, world, 3412, 1).await?;
    buy(&mut client, world, 3265, 1).await?;
    expect(&mut client, shield, "equip-shield", Out::Moved).await?;
    settle(&mut client, shield, true).await?;
    expect(&mut client, sword, "equip-two-hander", Out::Moved).await?;
    settle(&mut client, sword, true).await?;
    settle(&mut client, shield, false).await?;
    wait_quantity(&mut client, shield, shield_base + 1).await?;
    buy(&mut client, world, 3447, 10).await?;
    let before = quantity(&client, arrow)?;
    let refused = hotkey(&mut client, arrow).await?;
    if refused != Out::SlotMismatch
        || quantity(&client, arrow)? != before
        || worn(&client, arrow).is_some()
    {
        return Err(format!(
            "step=arrow-without-quiver observed={refused:?};needed=SlotMismatch-and-no-change"
        )
        .into());
    }
    buy(&mut client, world, 35562, 1).await?;
    expect(&mut client, quiver, "equip-quiver", Out::Moved).await?;
    settle(&mut client, quiver, true).await?;
    let quiver_before = quiver_arrows(clients).await?;
    expect(&mut client, arrow, "arrow-into-quiver", Out::Moved).await?;
    wait_quantity(&mut client, arrow, 0).await?;
    // Credit side: every arrow that left the backpack must now sit in the quiver.
    let moved = i64::try_from(before)?;
    let credited = quiver_arrows(clients).await? - quiver_before;
    if moved == 0 || credited != moved {
        return Err(format!("step=arrow-into-quiver observed=backpack-debit:{moved};quiver-credit:{credited};needed=equal-and-positive").into());
    }
    let mut client = relog(&mut client, clients, 0xe3).await?;
    if worn(&client, sword).is_none()
        || worn(&client, quiver).is_none()
        || quantity(&client, shield)? != shield_base + 1
        || quantity(&client, arrow)? != 0
    {
        return Err("step=fresh-admission observed=swap-or-quiver-state-lost".into());
    }
    let credited = quiver_arrows(clients).await? - quiver_before;
    if credited != moved {
        return Err(format!(
            "step=fresh-admission observed=quiver-credit:{credited};needed={moved}"
        )
        .into());
    }
    logout(&mut client).await?;
    Ok("two-hand-swap-and-arrows-to-quiver-durable".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_tokens_cover_empty_and_whitespace() {
        assert_eq!(token(""), "");
        assert!(!token("a b\tc\nd").chars().any(char::is_whitespace));
        assert_eq!(token("source:path.rs:12"), "source:path.rs:12");
    }
}

#[cfg(test)]
mod quantity_tests {
    use super::*;
    #[test]
    fn totals_do_not_wrap_at_the_wire_count_limit() {
        let mut inventory = oteryn_dev_client::CharacterInventory::default();
        assert!(inventory_quantities(&inventory).is_empty());
        let item = oteryn_dev_client::ItemEntry {
            handle: std::num::NonZeroU64::MIN,
            item_definition_ref: std::num::NonZeroU32::MIN,
            count: std::num::NonZeroU32::MAX,
            sub_type: 0,
        };
        inventory.entries = vec![item, item];
        assert_eq!(
            inventory_quantities(&inventory).get(&1),
            Some(&(2 * u64::from(u32::MAX)))
        );
    }
}
