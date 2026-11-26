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
//! 4. Obtain a fresh admission immediately after each transformation and check its map
//!    snapshot against the expected appearance before travelling to the next target.
//!
//! All commands below travel through the native dev client.  Missing money,
//! vendor content, reachable authored terrain, or a server frame is reported
//! only after the preceding real player actions have been attempted.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::npc_service as npc;
use oteryn_protocol_oteryn::world_map::{MapOrigin, MapTile};
use oteryn_protocol_oteryn::world_object::*;
#[cfg(test)]
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

// Catalogue prices for the exact authored sellers used by this journey.
const PURCHASES: [(&str, &[(u32, u32)]); 2] = [
    ("sam", &[(3293, 7)]),
    (
        "lubo",
        &[(3457, 50), (3308, 35), (3453, 50), (3304, 260), (3456, 50)],
    ),
];

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::observe(journey(clients)).await;
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

fn matches_terrain_action(appearance: u16, action: Option<u16>) -> bool {
    appearance != 4848 || action == Some(40041)
}

fn objects(bytes: &[u8]) -> TestResult<Vec<Object>> {
    let mut result = Vec::new();
    oteryn_world_bundle::bundle::visit(bytes, |manifest, sector| {
        for tile in sector.tiles {
            for (ordinal, item) in tile.items.iter().filter(|item| item.depth == 0).enumerate() {
                if let Some(appearance) =
                    crate::map::palette_appearance(&manifest.palette[item.palette as usize].key)
                {
                    // The Crystal crowbar recipe is for the Ape City cask,
                    // identified by action 40041 rather than appearance alone.
                    if !matches_terrain_action(appearance, item.attrs.action) {
                        continue;
                    }
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
    let targets: Vec<_> = objects
        .iter()
        .copied()
        .filter(|object| object.appearance == appearance)
        .collect();
    let candidates = targets.clone();
    let path = route_live(client, world, move |at| {
        candidates.iter().any(|object| distance(at, object.at) == 1)
    })
    .await?
    .ok_or_else(|| format!("step=approach-{appearance} observed=no-reachable-authored-target need=compiled-world-target"))?;
    walk_logged(client, world, &path, &format!("terrain-{appearance}")).await?;
    let at = tile(client.world_spatial().actor_position)?;
    targets
        .into_iter()
        .find(|object| distance(at, object.at) == 1)
        .ok_or_else(|| {
            format!("step=approach-{appearance} observed=target-not-adjacent at={at:?}").into()
        })
}

/// Keep the in-flight STEP and its compiled landing in the evidence even if the
/// outer row deadline cancels the walk before the server replies.
async fn walk_logged(
    client: &mut DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
    purpose: &str,
) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let goal = path
        .iter()
        .copied()
        .try_fold(from, |at, step| destination(world, at, step));
    evidence(&format!(
        "M08_ROUTE purpose={purpose} from={from:?} goal={goal:?} steps={}",
        path.len()
    ));
    for (index, &step) in path.iter().enumerate() {
        let from = tile(client.world_spatial().actor_position)?;
        let expected = destination(world, from, step);
        evidence(&format!(
            "M08_STEP purpose={purpose} index={index} from={from:?} step={step:?} expected={expected:?}"
        ));
        walk(client, world, &[step]).await.map_err(|error| {
            format!(
                "M08_ROUTE purpose={purpose} last={:?} {error}",
                client.world_spatial().actor_position
            )
        })?;
    }
    Ok(())
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

async fn stage_starting_tools(
    client: &mut DevClientSession,
    definitions: &crate::content::item_ref::ItemDefinitionIndex,
) -> TestResult {
    let at = client.world_spatial().actor_position;
    let ground = oteryn_protocol_oteryn::item_view::WorldTilePosition {
        x: at.x,
        y: at.y,
        floor: at.floor,
    };
    for (_, id, _, _) in CASES {
        let reference = provisioning::item_reference(definitions, id)?;
        if let Some(item) = inventory_item(client, reference) {
            let moved = client
                .session_mut()
                .move_item_counted(oteryn_dev_client::client_runtime::inventory::MoveRequest {
                    source: item.handle,
                    destination: oteryn_dev_client::ItemMoveDestination::Ground(ground),
                    count: None,
                })
                .await?;
            if moved.outcome != oteryn_dev_client::ItemMoveOutcome::Moved {
                return Err(
                    format!("M08 staging starter tool {id} at {at:?} refused: {moved:?}").into(),
                );
            }
            let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
            while inventory_item(client, reference).is_some()
                && tokio::time::Instant::now() < deadline
            {
                client.service_liveness(Duration::from_millis(25)).await?;
            }
            if inventory_item(client, reference).is_some() {
                return Err(format!(
                    "M08 starter tool {id} remained in served inventory after move"
                )
                .into());
            }
            evidence(&format!(
                "M08_CHECK stage-starter-{id} PASS position={at:?} reply=Moved inventory=0"
            ));
        }
    }
    Ok(())
}

async fn buy_tools(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    definitions: &crate::content::item_ref::ItemDefinitionIndex,
    seller: &str,
    purchases: &[(u32, u32)],
) -> TestResult {
    use oteryn_dev_client::client_runtime::npc_shop as shop;
    let vendor = npcs
        .npcs
        .iter()
        .find(|entry| entry.key == format!("oteryn:npc.{seller}"))
        .ok_or_else(|| format!("step=vendor-{seller} observed=placement-missing"))?;
    let path = full_row_m09::shop_path(client, world, vendor).await?;
    walk_logged(client, world, &path, seller).await?;
    let name = match seller {
        "lubo" => "Lubo",
        "sam" => "Sam",
        _ => return Err(format!("step=vendor-{seller} observed=not-a-tool-seller").into()),
    };
    let actor = full_row_m09::shop_actor(client, vendor, name).await?;

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
                "step=vendor-{seller}-{text} observed={reply:?} need=accepted-dialogue"
            )
            .into());
        }
        if text == "hi" {
            let delta = client
                .session_mut()
                .read_gated_delta(npc::STATE_DOMAIN_NPC_CONVERSATION, 1)
                .await?;
            if npc::decode_npc_conversation(&delta.payload)
                .map_err(|e| format!("conversation:{e:?}"))?
                .open
                .is_none()
            {
                return Err(format!("step=vendor-{seller}-hi observed=missing-open-delta").into());
            }
        }
    }
    let delta = client
        .session_mut()
        .read_gated_delta(npc::STATE_DOMAIN_NPC_TRADE_WINDOW, 1)
        .await?;
    let details = client
        .session_mut()
        .is_selected(oteryn_protocol_oteryn::trade_details::CAPABILITY_TRADE_DETAILS_V1);
    let window = npc::decode_npc_trade_window_for(&delta.payload, details)
        .map_err(|e| format!("trade-window:{e:?}"))?;
    let catalogue_revision = window.open.ok_or("trade-window-closed")?.1;
    for &(id, price) in purchases {
        let reference = provisioning::item_reference(definitions, id)?;
        if inventory_item(client, reference).is_some() {
            return Err(format!(
                "step=1-buy-{id} observed=reply-not-sent-item-already-present need=live-buy-reply-and-inventory-delta"
            ).into());
        }
        let (index, offer) = window
            .offers
            .iter()
            .enumerate()
            .find(|(_, offer)| {
                offer.side == npc::NpcTradeSide::Buy && offer.item_definition_ref == reference.get()
            })
            .ok_or_else(|| format!("step=buy-{id} observed=offer-missing vendor={seller}"))?;
        if offer.count != 1 || offer.unit_price != price {
            return Err(format!("step=buy-{id} vendor={seller} served-count={} served-price={} expected-count=1 expected-price={price}", offer.count, offer.unit_price).into());
        }
        let reply = shop::buy(
            client.session_mut(),
            &npc::NpcTradeIntent {
                npc_actor: actor,
                catalogue_revision,
                offer_index: u32::try_from(index)?,
                side: offer.side,
                quantity: 1,
                expected_unit_price: offer.unit_price,
            },
        )
        .await?;
        if reply != npc::NpcIntentDisposition::Ok {
            return Err(
                format!("step=buy-{id} observed={reply:?} vendor={seller} price={price}").into(),
            );
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while inventory_item(client, reference).is_none() && tokio::time::Instant::now() < deadline
        {
            client.service_liveness(Duration::from_millis(25)).await?;
        }
        let carried = inventory_item(client, reference)
            .ok_or_else(|| format!("step=buy-{id} observed=no-inventory-delta"))?;
        if carried.count.get() != 1 {
            return Err(format!(
                "step=buy-{id} observed=unexpected-served-count:{}",
                carried.count
            )
            .into());
        }
        evidence(&format!(
            "M08_CHECK acquire-{id} PASS vendor={seller} reply=Ok inventory=0->1"
        ));
        verdict::Harness::global().progress("npc-purchase");
    }
    Ok(())
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
        SessionEvent::WorldMapSnapshot(delta) => delta
            .value
            .tiles
            .iter()
            .any(|tile| tile_at(tile, object.at)),
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
    let reply = client.session_mut().world_context_use(&payload).await?;
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

async fn verify_after_relog(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    name: &str,
    object: Object,
    after: u16,
) -> TestResult {
    let session = client.join_snapshot().game_session_id;
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err(format!("step=relog-{name} observed=logout-refused").into());
    }
    *client = connect_character(clients, 0xd9, 0).await?;
    if client.join_snapshot().game_session_id == session {
        return Err(format!("step=relog-{name} observed=reused-session").into());
    }
    approach(client, world, &[object], object.appearance).await?;
    let snapshot = client
        .session_mut()
        .world_map_snapshot()
        .ok_or("step=relog observed=missing-map-snapshot")?;
    if !snapshot
        .tiles
        .iter()
        .any(|tile| tile_has(tile, object, after))
    {
        return Err(format!(
            "step=relog-{name} observed=transformed-placement-missing position={:?} expected={after}",
            object.at
        ).into());
    }
    evidence(&format!("M08_CHECK relog-{name} PASS appearance={after}"));
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
    let identity =
        character_prep::dedicated_character(clients, "NODE_BOOT_M08_FULL_ACCOUNT").await?;
    let clients = &identity.clients(clients);
    character_prep::declare_seeded("M08.full", "452 carried gold", "six live tool purchases");
    let definitions = provisioning::item_index()?;
    // The 44-step Sam leg and 205-step Lubo leg reached the shovel purchase
    // guard after 355.698 s. Measured purchase staging is bounded separately
    // from the original 300 s budget for transformations, stale use and relogs.
    let mut client = verdict::indexed_timeout(Duration::from_secs(460), async {
        let mut client =
            character_prep::funded_character(clients, 0xd8, 452, &[(3035, 4), (3031, 2)]).await?;
        stage_starting_tools(&mut client, &definitions).await?;
        for (seller, purchases) in PURCHASES {
            buy_tools(
                &mut client,
                world,
                &metadata.npcs,
                &definitions,
                seller,
                purchases,
            )
            .await?;
        }
        TestResult::Ok(client)
    })
    .await
    .map_err(|_| "step=tool-purchase-staging observed=timeout-after-460s")??;
    verdict::indexed_timeout(Duration::from_secs(300), async {
        exercise_tools(&mut client, clients, world, &objects, &definitions).await
    })
    .await
    .map_err(|_| "step=terrain-journey observed=timeout-after-300s")?
}

async fn exercise_tools(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    objects: &[Object],
    definitions: &crate::content::item_ref::ItemDefinitionIndex,
) -> TestResult {
    let mut pick = None;
    for (name, id, before, after) in CASES {
        let reference = provisioning::item_reference(definitions, id)?;
        let item = inventory_item(client, reference)
            .ok_or_else(|| format!("step={name} observed=purchased-tool-missing"))?;
        let object = approach(client, world, objects, before).await?;
        use_tool(client, world, item, object, after).await?;
        if name == "pick" {
            pick = Some((object, after));
        } else {
            // Several terrain bindings decay in minutes. Read the fresh
            // snapshot before routing to the next tool or terrain target.
            verify_after_relog(client, clients, world, name, object, after).await?;
        }
    }
    let (pick, pick_after) = pick.ok_or("step=pick observed=not-attempted")?;
    let pick_ref = provisioning::item_reference(definitions, 3456)?;
    let pick_item = inventory_item(client, pick_ref).ok_or("pick missing")?;
    let stale = UseIntent {
        target: UseTarget::Item(pick_item.handle),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(target(world, pick, 0)?)),
        use_with_position: None,
    };
    client.take_events();
    let reply = client
        .session_mut()
        .world_context_use(
            &encode_item_use_intent(&stale).map_err(|e| format!("stale-intent:{e:?}"))?,
        )
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
    verify_after_relog(client, clients, world, "pick", pick, pick_after).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crowbar_route_uses_the_ape_city_action_id() {
        assert!(!matches_terrain_action(4848, None));
        assert!(!matches_terrain_action(4848, Some(40042)));
        assert!(matches_terrain_action(4848, Some(40041)));
        assert!(matches_terrain_action(593, None));
    }

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
