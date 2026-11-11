//! I18c: browse field, move up from a ground container.
//! Buys a bag (2853), drops it on the character's tile, browses the tile, opens the bag from the
//! browse view in place and expects the bag view's parent to be the browse view's container,
//! then sends `Up` and expects the browse view of that tile back. The bag is picked up again.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use std::collections::{BTreeMap, BTreeSet};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(360), journey(clients)).await;
    let (status, detail) = match result {
        Ok(Ok(detail)) => ("PASS", token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=I18c.full status={status} seconds={:.3} evidence={detail}",
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

use oteryn_dev_client::{ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome};
use oteryn_protocol_oteryn::browse_field as browse;
use oteryn_protocol_oteryn::container_tree as tree;
use oteryn_protocol_oteryn::item_view::WorldTilePosition;

fn view_of(payload: &[u8]) -> TestResult<tree::ContainerViews> {
    tree::decode_container_views_for(payload, true)
        .map_err(|error| format!("step=views-decode observed={error:?}").into())
}

async fn browse_open(
    client: &mut DevClientSession,
    at: WorldTilePosition,
) -> TestResult<(tree::ContainerViewOutcome, Option<tree::ContainerView>)> {
    let payload =
        browse::encode_browse_field_intent(&browse::BrowseFieldIntent::Open { position: at })
            .map_err(|error| format!("step=browse-encode observed={error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            browse::CAPABILITY_BROWSE_FIELD_V1,
            browse::COMMAND_TYPE_BROWSE_FIELD_INTENT,
            &payload,
        )
        .await?;
    let outcome = tree::decode_container_view_result(&reply.payload)
        .map_err(|error| format!("step=browse-reply observed={error:?}"))?;
    if outcome != tree::ContainerViewOutcome::Opened {
        return Ok((outcome, None));
    }
    let delta = client
        .session_mut()
        .read_gated_delta(
            tree::STATE_DOMAIN_CONTAINER_VIEWS,
            tree::DELTA_TYPE_CONTAINER_VIEWS_V1,
        )
        .await?;
    let view = view_of(&delta.payload)?
        .views
        .into_iter()
        .find(|view| view.browse.is_some());
    Ok((outcome, view))
}

async fn view_intent(
    client: &mut DevClientSession,
    intent: tree::ContainerViewIntent,
) -> TestResult<(tree::ContainerViewOutcome, Vec<tree::ContainerView>)> {
    let payload = tree::encode_container_view_intent(&intent)
        .map_err(|error| format!("step=view-encode observed={error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            tree::CAPABILITY_CONTAINER_TREE_V1,
            tree::COMMAND_TYPE_CONTAINER_VIEW_INTENT,
            &payload,
        )
        .await?;
    let outcome = tree::decode_container_view_result(&reply.payload)
        .map_err(|error| format!("step=view-reply observed={error:?}"))?;
    let delta = client
        .session_mut()
        .read_gated_delta(
            tree::STATE_DOMAIN_CONTAINER_VIEWS,
            tree::DELTA_TYPE_CONTAINER_VIEWS_V1,
        )
        .await?;
    Ok((outcome, view_of(&delta.payload)?.views))
}

fn here(client: &DevClientSession) -> WorldTilePosition {
    let at = client.world_spatial().actor_position;
    WorldTilePosition {
        x: at.x,
        y: at.y,
        floor: -at.floor,
    }
}

fn ground(client: &DevClientSession) -> WorldTilePosition {
    let at = client.world_spatial().actor_position;
    WorldTilePosition {
        x: at.x,
        y: at.y,
        floor: at.floor,
    }
}

/// Drops the first backpack entry on the actor's own tile and returns its definition and count.
async fn drop_first(
    client: &mut DevClientSession,
    prefer: Option<std::num::NonZeroU32>,
) -> TestResult<(std::num::NonZeroU32, std::num::NonZeroU32)> {
    let item = client
        .inventory()
        .and_then(|inventory| {
            prefer
                .and_then(|want| {
                    inventory
                        .entries
                        .iter()
                        .find(|item| item.item_definition_ref == want)
                })
                .or(inventory.entries.first())
        })
        .copied()
        .ok_or("step=drop observed=no-backpack-item;needed:player-acquired-item")?;
    let at = ground(client);
    let before = quantity(client, item.item_definition_ref)?;
    client.take_events();
    let moved = client
        .move_item(&ItemMoveIntent {
            source: item.handle,
            destination: ItemMoveDestination::Ground(at),
        })
        .await?;
    if moved.outcome != ItemMoveOutcome::Moved {
        return Err(format!("step=drop observed={:?};needed=Moved", moved.outcome).into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while quantity(client, item.item_definition_ref)? >= before {
        client.take_events();
        if tokio::time::Instant::now() >= deadline {
            return Err("step=drop observed=no-inventory-debit".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    Ok((item.item_definition_ref, item.count))
}

/// Moves a browse entry into the main backpack and waits for the inventory credit.
async fn take_entry(
    client: &mut DevClientSession,
    entry: &oteryn_protocol_oteryn::item_view::ItemEntry,
) -> TestResult<()> {
    let before = quantity(client, entry.item_definition_ref)?;
    client.take_events();
    let moved = client
        .move_item(&ItemMoveIntent {
            source: entry.handle,
            destination: ItemMoveDestination::MainBackpack,
        })
        .await?;
    if moved.outcome != ItemMoveOutcome::Moved {
        return Err(format!("step=take observed={:?};needed=Moved", moved.outcome).into());
    }
    wait_quantity(
        client,
        entry.item_definition_ref,
        before + u64::from(entry.count.get()),
    )
    .await?;
    Ok(())
}

fn has(
    view: &tree::ContainerView,
    definition: std::num::NonZeroU32,
) -> Option<oteryn_protocol_oteryn::item_view::ItemEntry> {
    view.entries
        .iter()
        .find(|entry| entry.item_definition_ref == definition)
        .copied()
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let world = open_world(clients).await?;
    let mut client = connect_world(clients, 0xf3).await?;
    let bag = reference(2853)?;
    acquire(&mut client, world, 2853, 1)
        .await
        .map_err(|error| {
            format!("step=buy-bag observed={error};needed:player-gold-and-reachable-seller")
        })?;
    let at = here(&client);
    drop_first(&mut client, Some(bag)).await?;
    let (outcome, view) = browse_open(&mut client, at).await?;
    let browse_view =
        view.ok_or_else(|| format!("step=browse observed={outcome:?};needed=Opened-with-view"))?;
    let entry = has(&browse_view, bag).ok_or("step=browse observed=bag-not-listed")?;
    let (opened, views) = view_intent(
        &mut client,
        tree::ContainerViewIntent::Open {
            handle: entry.handle,
            replace_view: Some(browse_view.view_id),
        },
    )
    .await?;
    let inner = views
        .iter()
        .find(|view| view.container_handle == entry.handle)
        .ok_or_else(|| format!("step=open-bag observed={opened:?};needed=bag-view"))?;
    if opened != tree::ContainerViewOutcome::Opened
        || inner.parent_handle != Some(browse_view.container_handle)
    {
        return Err(format!(
            "step=open-bag observed={opened:?}-parent:{:?};needed=Opened-with-browse-parent",
            inner.parent_handle
        )
        .into());
    }
    let inner_id = inner.view_id;
    let (up, views) = view_intent(
        &mut client,
        tree::ContainerViewIntent::Up { view_id: inner_id },
    )
    .await?;
    let restored = views
        .iter()
        .find(|view| view.browse.is_some_and(|page| page.position == at));
    if up != tree::ContainerViewOutcome::Opened || restored.is_none() {
        return Err(format!(
            "step=up observed={up:?}-views:{};needed=Opened-browse-view",
            views.len()
        )
        .into());
    }
    let entry = restored
        .and_then(|view| has(view, bag))
        .ok_or("step=up observed=bag-not-listed-in-restored-view")?;
    take_entry(&mut client, &entry).await?;
    let restored = quantities(&client)?;
    logout(&mut client).await?;
    let mut fresh = connect_world(clients, 0xf1).await?;
    let durable = quantities(&fresh)?;
    logout(&mut fresh).await?;
    if durable != restored {
        return Err("step=fresh-admission observed=inventory-custody-lost".into());
    }
    Ok("up-returned-to-browse-view-of-the-tile".to_owned())
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

    #[test]
    fn container_reply_accepts_empty_views_and_rejects_malformed_payload() {
        assert!(view_of(&[]).is_ok_and(|views| views.views.is_empty()));
        assert!(view_of(&[255]).is_err());
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
