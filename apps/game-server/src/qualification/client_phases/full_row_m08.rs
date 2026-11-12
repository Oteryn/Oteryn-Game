//! M08 full-row player scenario, derived from the matrix/re-audit and the live
//! `item_craft::terrain` owner:
//! 1. Admit a node-boot character, walk to an ordinary NPC, say `hi` and
//!    `trade`, buy a shovel, machete, scythe, sickle, crowbar and pick, and
//!    assert each dialogue reply, trade-window delta, BUY reply and inventory delta.
//! 2. Walk adjacent to authored terrain for each tool and send native
//!    ITEM_USE_V1/USE_WITH commands.  Assert Accepted+Committed and the map-view
//!    delta's exact transformed appearance (shovel 593->594, machete 3696->3695,
//!    scythe 3653->3651, sickle 5463->5462, crowbar 4848->3134, pick 7200->7236).
//! 3. Send a stale-revision repeat against a transformed placement; assert the
//!    refusal reply and no second transformation delta.
//! 4. Log out, obtain a fresh admission for the same character, walk back to the
//!    durable pick hole and assert the map snapshot still carries appearance 7236.
//!
//! All commands below travel through the native dev client.  Missing money,
//! vendor content, reachable authored terrain, or a server frame is reported
//! only after the preceding real player actions have been attempted.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind, SessionEvent};
use oteryn_protocol_oteryn::npc_service as npc;
use oteryn_protocol_oteryn::world_map::{MapOrigin, MapTile};
use oteryn_protocol_oteryn::world_object::*;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug)]
struct Object {
    at: TilePos,
    ordinal: u8,
    appearance: u16,
}

const CASES: [(&str, u32, u16, u16); 6] = [
    ("shovel", 3457, 593, 594),
    ("machete", 3308, 3696, 3695),
    ("scythe", 3453, 3653, 3651),
    ("sickle", 3293, 5463, 5462),
    ("crowbar", 3304, 4848, 3134),
    ("pick", 3456, 7200, 7236),
];

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(300), journey(clients)).await;
    let result = match result {
        Ok(result) => result,
        Err(_) => Err("step=journey observed=timeout-after-300s need=responsive-server".into()),
    };
    if let Err(error) = &result {
        evidence(&format!("M08_GAP {error}"));
    }
    evidence(&format!(
        "e2e_all scenario=M08.full status={} seconds={:.3}{}",
        if result.is_ok() { "PASS" } else { "FAIL" },
        started.elapsed().as_secs_f64(),
        result
            .err()
            .map(|error| format!(" evidence={}", clean(&error.to_string())))
            .unwrap_or_default()
    ));
}

fn clean(value: &str) -> String {
    value.replace([' ', '\n', '\r'], "-")
}

fn objects(bytes: &[u8]) -> TestResult<Vec<Object>> {
    let mut result = Vec::new();
    oteryn_world_bundle::bundle::visit(bytes, |manifest, sector| {
        for tile in sector.tiles {
            for (ordinal, item) in tile.items.iter().filter(|item| item.depth == 0).enumerate() {
                if let Some(appearance) =
                    crate::map::palette_appearance(&manifest.palette[item.palette as usize].key)
                {
                    result.push(Object {
                        at: TilePos {
                            x: tile.x,
                            y: tile.y,
                            floor: sector.floor,
                        },
                        ordinal: u8::try_from(ordinal)
                            .map_err(|_| oteryn_world_bundle::Error::Limit("M08 ordinal".into()))?,
                        appearance,
                    });
                }
            }
        }
        Ok(())
    })?;
    result.sort_by_key(|object| (object.at, object.ordinal));
    Ok(result)
}

fn distance(a: TilePos, b: TilePos) -> u16 {
    if a.floor == b.floor {
        a.x.abs_diff(b.x).max(a.y.abs_diff(b.y))
    } else {
        u16::MAX
    }
}

async fn approach(
    client: &mut DevClientSession,
    world: &BundleWorld,
    objects: &[Object],
    appearance: u16,
) -> TestResult<Object> {
    let from = tile(client.world_spatial().actor_position)?;
    for &object in objects
        .iter()
        .filter(|object| object.appearance == appearance)
    {
        if let Some(path) = route(world, from, |at| distance(at, object.at) == 1) {
            walk(client, world, &path).await?;
            return Ok(object);
        }
    }
    Err(format!("step=approach-{appearance} observed=no-reachable-authored-target need=compiled-world-target").into())
}

fn inventory_item(
    client: &DevClientSession,
    reference: std::num::NonZeroU32,
) -> Option<oteryn_dev_client::ItemEntry> {
    client
        .inventory()?
        .entries
        .iter()
        .find(|entry| entry.item_definition_ref == reference)
        .copied()
}

async fn buy(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    id: u32,
) -> TestResult<oteryn_dev_client::ItemEntry> {
    use oteryn_dev_client::client_runtime::npc_shop as shop;
    let definitions = provisioning::item_index()?;
    let reference = provisioning::item_reference(&definitions, id)?;
    if inventory_item(client, reference).is_some() {
        return Err(format!(
            "step=1-buy-{id} observed=reply-not-sent-item-already-present need=live-buy-reply-and-inventory-delta"
        )
        .into());
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let records: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/services/trade/trade-00000-00323.json"),
    )?)?;
    let key = format!("oteryn:item.tibia.i{id}");
    for record in records["records"]
        .as_array()
        .ok_or("step=trade-catalogue observed=missing-records")?
    {
        if !record["declaration"]["offers"]
            .as_array()
            .is_some_and(|offers| {
                offers.iter().any(|offer| {
                    offer["direction"] == "SellToPlayer" && offer["item"]["key"] == key
                })
            })
        {
            continue;
        }
        let Some(slug) = record["declaration"]["identity"]["key"]
            .as_str()
            .and_then(|value| value.strip_prefix("oteryn:service.trade."))
        else {
            continue;
        };
        let Some(vendor) = npcs
            .npcs
            .iter()
            .find(|entry| entry.key == format!("oteryn:npc.{slug}"))
        else {
            continue;
        };
        let Some(path) = route(world, tile(client.world_spatial().actor_position)?, |at| {
            vendor
                .placements
                .iter()
                .any(|p| p.floor == at.floor && p.x.abs_diff(at.x).max(p.y.abs_diff(at.y)) <= 3)
        }) else {
            continue;
        };
        walk(client, world, &path).await?;
        let actor = client
            .world_entities()
            .and_then(|entities| {
                entities.iter().find(|entity| {
                    entity.kind == EntityKind::Npc
                        && vendor.placements.iter().any(|p| {
                            entity.position
                                == ActorPosition {
                                    x: i32::from(p.x),
                                    y: i32::from(p.y),
                                    floor: -i16::from(p.floor),
                                }
                        })
                })
            })
            .map(|entry| entry.entity)
            .ok_or_else(|| format!("step=vendor-{slug} observed=actor-not-visible"))?;
        for text in ["hi", "trade"] {
            let reply = shop::talk(
                client.session_mut(),
                &npc::NpcTalkIntent {
                    npc_actor: actor,
                    text: text.into(),
                },
            )
            .await?;
            if reply != npc::NpcIntentDisposition::Ok {
                return Err(format!(
                    "step=vendor-{slug}-{text} observed={reply:?} need=accepted-dialogue"
                )
                .into());
            }
            let domain = if text == "hi" {
                npc::STATE_DOMAIN_NPC_CONVERSATION
            } else {
                npc::STATE_DOMAIN_NPC_TRADE_WINDOW
            };
            let delta = client.session_mut().read_gated_delta(domain, 1).await?;
            if text == "hi" {
                if npc::decode_npc_conversation(&delta.payload)
                    .map_err(|e| format!("conversation:{e:?}"))?
                    .open
                    .is_none()
                {
                    return Err(format!("step=vendor-{slug}-hi observed=missing-open-delta").into());
                }
            } else {
                let window = npc::decode_npc_trade_window(&delta.payload)
                    .map_err(|e| format!("trade-window:{e:?}"))?;
                let (index, offer) = window
                    .offers
                    .iter()
                    .enumerate()
                    .find(|(_, offer)| {
                        offer.side == npc::NpcTradeSide::Buy
                            && offer.item_definition_ref == reference.get()
                    })
                    .ok_or_else(|| format!("step=buy-{id} observed=offer-missing vendor={slug}"))?;
                let reply = shop::buy(
                    client.session_mut(),
                    &npc::NpcTradeIntent {
                        npc_actor: actor,
                        catalogue_revision: window.open.ok_or("trade-window-closed")?.1,
                        offer_index: u32::try_from(index)?,
                        side: offer.side,
                        quantity: 1,
                        expected_unit_price: offer.unit_price,
                    },
                )
                .await?;
                if reply != npc::NpcIntentDisposition::Ok {
                    return Err(format!("step=buy-{id} observed={reply:?} vendor={slug} price={} need=player-earnable-funds", offer.unit_price).into());
                }
                let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
                while inventory_item(client, reference).is_none()
                    && tokio::time::Instant::now() < deadline
                {
                    client.service_liveness(Duration::from_millis(25)).await?;
                }
                let item = inventory_item(client, reference)
                    .ok_or_else(|| format!("step=buy-{id} observed=missing-inventory-delta"))?;
                evidence(&format!(
                    "M08_CHECK acquire-{id} PASS vendor={slug} reply=Ok inventory=present"
                ));
                return Ok(item);
            }
        }
    }
    Err(
        format!("step=buy-{id} observed=no-reachable-live-vendor need=world-vendor-for-{key}")
            .into(),
    )
}

fn target(world: &BundleWorld, object: Object, revision: u64) -> TestResult<WorldObjectTarget> {
    let key =
        crate::map::view::placement_key(object.at, object.ordinal).ok_or("M08 placement key")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget {
        placement,
        expected_revision: revision,
    })
}

fn transformed(events: &[SessionEvent], object: Object, after: u16) -> bool {
    events.iter().any(|event| match event {
        SessionEvent::WorldMap(delta) => delta
            .value
            .tiles
            .iter()
            .any(|tile| tile_has(tile, object, after)),
        SessionEvent::WorldMapSnapshot(delta) => delta
            .value
            .tiles
            .iter()
            .any(|tile| tile_has(tile, object, after)),
        _ => false,
    })
}

fn tile_has(tile: &MapTile, object: Object, appearance: u16) -> bool {
    tile_at(tile, object.at)
        && tile.items.iter().any(|item| item.appearance_id == appearance && matches!(item.origin, MapOrigin::BaseOrdinal { ordinal, .. } if ordinal == object.ordinal))
}

fn contains_placement_tile_delta(events: &[SessionEvent], object: Object) -> bool {
    events.iter().any(|event| match event {
        SessionEvent::WorldMap(delta) => delta
            .value
            .tiles
            .iter()
            .any(|tile| tile_at(tile, object.at)),
        SessionEvent::WorldMapSnapshot(delta) => {
            delta.value.tiles.iter().any(|tile| tile_at(tile, object.at))
        }
        _ => false,
    })
}

fn tile_at(tile: &MapTile, position: TilePos) -> bool {
    tile.position
        == ActorPosition {
            x: i32::from(position.x),
            y: i32::from(position.y),
            floor: -i16::from(position.floor),
        }
}

async fn use_tool(
    client: &mut DevClientSession,
    world: &BundleWorld,
    item: oteryn_dev_client::ItemEntry,
    object: Object,
    after: u16,
) -> TestResult {
    client.take_events();
    let intent = UseIntent {
        target: UseTarget::Item(item.handle),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(target(world, object, 0)?)),
        use_with_position: None,
    };
    let payload = encode_item_use_intent(&intent).map_err(|e| format!("tool-intent:{e:?}"))?;
    let reply = client
        .session_mut()
        .world_context_use(&payload)
        .await?;
    let disposition =
        decode_use_result(&reply.payload).map_err(|e| format!("tool-result:{e:?}"))?;
    if reply.status != CommandStatus::Accepted || disposition != UseDisposition::Committed {
        return Err(format!(
            "step=use-{} observed=status:{:?},reply:{disposition:?} need=Committed",
            object.appearance, reply.status
        )
        .into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let events = client.take_events();
        if transformed(&events, object, after) {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "step=delta-{} observed=timeout-missing-appearance-{after} need=world-map-delta",
                object.appearance
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
    evidence(&format!(
        "M08_CHECK tool-{} PASS reply=Committed transformed={after}",
        object.appearance
    ));
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes.clone(),
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let objects = objects(&bytes)?;
    let mut client = connect_world(clients, 0xd8).await?;
    let mut completed = BTreeSet::new();
    let mut pick = None;
    for (name, id, before, after) in CASES {
        let item = buy(&mut client, world, &metadata.npcs, id).await?;
        let object = approach(&mut client, world, &objects, before).await?;
        use_tool(&mut client, world, item, object, after).await?;
        completed.insert(name);
        if name == "pick" {
            pick = Some(object);
        }
    }
    let pick = pick.ok_or("step=pick observed=not-attempted")?;
    let pick_item = inventory_item(
        &client,
        provisioning::item_reference(&provisioning::item_index()?, 3456)?,
    )
    .ok_or("pick missing")?;
    let stale = UseIntent {
        target: UseTarget::Item(pick_item.handle),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(target(world, pick, 0)?)),
        use_with_position: None,
    };
    client.take_events();
    let reply = client
        .session_mut()
        .world_context_use(&encode_item_use_intent(&stale).map_err(|e| format!("stale-intent:{e:?}"))?)
        .await?;
    let refusal = decode_use_result(&reply.payload).map_err(|e| format!("stale-result:{e:?}"))?;
    if reply.status != CommandStatus::Accepted || refusal == UseDisposition::Committed {
        return Err(format!(
            "step=stale-refusal observed=status:{:?},reply:{refusal:?} need=non-committed-refusal",
            reply.status
        )
        .into());
    }
    client.service_liveness(Duration::from_millis(250)).await?;
    let second_events = client.take_events();
    let snapshot = client
        .session_mut()
        .world_map_snapshot()
        .ok_or_else(|| {
            format!(
                "step=3-stale-no-second-delta observed=reply:{refusal:?},map-snapshot-missing need=appearance-7236-and-no-object-delta"
            )
        })?;
    let durable_state = snapshot.tiles.iter().any(|tile| tile_has(tile, pick, 7236));
    let second_delta = contains_placement_tile_delta(&second_events, pick);
    if second_delta || !durable_state {
        return Err(format!(
            "step=3-stale-no-second-delta observed=reply:{refusal:?},object-delta:{second_delta},appearance-7236:{durable_state} need=no-object-delta-and-appearance-7236"
        )
        .into());
    }
    evidence(&format!(
        "M08_CHECK stale-refusal PASS reply={refusal:?} object_delta=false appearance=7236"
    ));
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused need=accepted".into());
    }
    drop(client);
    let mut fresh = connect_world(clients, 0xd9).await?;
    let _ = approach(&mut fresh, world, &[pick], 7200).await?;
    let snapshot = fresh
        .session_mut()
        .world_map_snapshot()
        .ok_or("step=relog observed=missing-map-snapshot")?;
    if !snapshot.tiles.iter().any(|tile| tile_has(tile, pick, 7236)) {
        return Err(
            "step=relog-persistence observed=appearance-7236-missing need=durable-terrain-reload"
                .into(),
        );
    }
    if completed.len() != CASES.len() {
        return Err("step=complete observed=missing-tool-case".into());
    }
    evidence("M08_CHECK relog-persistence PASS appearance=7236");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scenario_has_every_required_tool_once() {
        assert_eq!(
            CASES
                .iter()
                .map(|case| case.0)
                .collect::<BTreeSet<_>>()
                .len(),
            6
        );
    }
}
