//! C16a client capture: real rune USE replies publish raw domain-17 map payloads.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use std::collections::BTreeSet;

fn rune_quantity(client: &DevClientSession, reference: std::num::NonZeroU32) -> TestResult<u64> {
    Ok(client
        .inventory()
        .ok_or("native inventory absent")?
        .entries
        .iter()
        .filter(|entry| entry.item_definition_ref == reference)
        .map(|entry| u64::from(entry.count.get()))
        .sum())
}
fn rune_handle(
    client: &DevClientSession,
    reference: std::num::NonZeroU32,
) -> TestResult<oteryn_protocol_oteryn::item_view::ItemHandle> {
    client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|entry| entry.item_definition_ref == reference)
        })
        .map(|entry| entry.handle)
        .ok_or_else(|| format!("no carried rune handle for definition {reference}").into())
}

// Content selects a real seller and navigation target. Neither source data nor an
// accepted trade receipt proves acquisition: require the subsequent inventory frame.
async fn acquire_rune(
    client: &mut DevClientSession,
    world: &BundleWorld,
    id: u32,
) -> TestResult<(std::num::NonZeroU32, u64)> {
    use oteryn_dev_client::client_runtime::npc_shop as runtime;
    use oteryn_protocol_oteryn::npc_service as wire;
    let index = provisioning::item_index()?;
    let reference = provisioning::item_reference(&index, id)?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let trades: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/services/trade/trade-00000-00323.json"),
    )?)?;
    let item_key = format!("oteryn:item.tibia.i{id}");
    let sellers: BTreeSet<_> = trades["records"]
        .as_array()
        .ok_or("trade records absent")?
        .iter()
        .filter_map(|record| {
            let declaration = &record["declaration"];
            declaration["offers"]
                .as_array()?
                .iter()
                .any(|offer| {
                    offer["direction"] == "SellToPlayer" && offer["item"]["key"] == item_key
                })
                .then(|| declaration["identity"]["key"].as_str())
                .flatten()
                .and_then(|key| key.strip_prefix("oteryn:service.trade."))
                .map(|name| format!("oteryn:npc.{name}"))
        })
        .collect();
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let here = tile(client.world_spatial().actor_position)?;
    let selected = metadata
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
        .min_by_key(|(_, path)| path.len());
    let Some((npc, path)) = selected else {
        let balances = runtime::balances(client.session_mut()).await?;
        return Err(format!("rune {id} acquisition blocked: no player-walk route within the admitted town window to a compiled-world seller; eligible_sellers={sellers:?}; observed_balances={balances:?}; carried_quantity={}; no item mint or remote vendor interaction; source:crates/gameplay/dispatch/src/gameplay_transport/npc_trade_command.rs:1", rune_quantity(client, reference)?).into());
    };
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
        .ok_or("rune vendor not visible at compiled placement")?;
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
            return Err(format!("rune {id} vendor {} {text} refused: {reply:?}", npc.key).into());
        }
    }
    let delta = client
        .session_mut()
        .read_gated_delta(wire::STATE_DOMAIN_NPC_TRADE_WINDOW, 1)
        .await?;
    let window = wire::decode_npc_trade_window(&delta.payload)
        .map_err(|error| format!("rune shop decode: {error:?}"))?;
    let (offer_index, offer) = window
        .offers
        .iter()
        .enumerate()
        .find(|(_, offer)| {
            offer.side == wire::NpcTradeSide::Buy && offer.item_definition_ref == reference.get()
        })
        .ok_or_else(|| format!("rune {id} not offered by live vendor {}", npc.key))?;
    let before = rune_quantity(client, reference)?;
    let balances = runtime::balances(client.session_mut()).await?;
    client.take_events();
    let reply = runtime::buy(
        client.session_mut(),
        &wire::NpcTradeIntent {
            npc_actor: actor,
            catalogue_revision: window.open.ok_or("rune shop closed")?.1,
            offer_index: u32::try_from(offer_index)?,
            side: offer.side,
            quantity: 2,
            expected_unit_price: offer.unit_price,
        },
    )
    .await?;
    if reply != wire::NpcIntentDisposition::Ok {
        return Err(format!("rune {id} player purchase refused by {}: {reply:?}; quantity=2 unit_price={} observed_balances={balances:?} carried_quantity={before}; no earned funds or rune were granted by this journey; source:crates/gameplay/dispatch/src/gameplay_transport/npc_trade_command.rs:1", npc.key, offer.unit_price).into());
    }
    let expected = before
        .checked_add(u64::from(offer.count) * 2)
        .ok_or("rune quantity overflow")?;
    let sequence = inventory_quantity(client, reference, expected).await?;
    Ok((reference, sequence))
}

async fn inventory_quantity(
    client: &mut DevClientSession,
    reference: std::num::NonZeroU32,
    expected: u64,
) -> TestResult<u64> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        for event in client.take_events() {
            if let SessionEvent::Inventory(delta) = event
                && delta
                    .value
                    .entries
                    .iter()
                    .filter(|entry| entry.item_definition_ref == reference)
                    .map(|entry| u64::from(entry.count.get()))
                    .sum::<u64>()
                    == expected
            {
                return Ok(delta.server_sequence);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("rune inventory frame missing: definition={reference} expected={expected} observed={}", rune_quantity(client, reference)?).into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }
}

async fn rune_use(
    client: &mut DevClientSession,
    reference: std::num::NonZeroU32,
    creature: Option<oteryn_dev_client::EntityRef>,
    position: Option<ActorPosition>,
) -> TestResult<(oteryn_protocol_oteryn::world_object::UseDisposition, u64)> {
    use oteryn_protocol_oteryn::{actor_spell::SpellTargetPosition, world_object as wire};
    if !client
        .selected_capabilities()
        .contains(&wire::CAPABILITY_RUNE_USE_V1)
    {
        return Err("RUNE_USE_V1 not negotiated on admitted native client".into());
    }
    let payload = wire::encode_item_use_intent(&wire::UseIntent {
        target: wire::UseTarget::Item(rune_handle(client, reference)?),
        use_with: creature,
        use_with_map_item: None,
        use_with_position: position.map(|at| SpellTargetPosition {
            x: at.x,
            y: at.y,
            floor: at.floor,
        }),
    })
    .map_err(|error| format!("rune USE encode: {error:?}"))?;
    let reply = client.session_mut().world_context_use(&payload).await?;
    let (payload, _) = oteryn_protocol_oteryn::refusal_detail::split_refusal_detail(&reply.payload)
        .map_err(|error| format!("rune refusal decode: {error:?}"))?;
    let disposition =
        wire::decode_use_result(payload).map_err(|error| format!("rune USE decode: {error:?}"))?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!(
            "native rune command status={:?} disposition={disposition:?}",
            reply.status
        )
        .into());
    }
    Ok((disposition, reply.server_sequence))
}

fn rune_corridor(world: &BundleWorld, from: TilePos) -> Option<Vec<TilePos>> {
    for direction in [
        StepDirection::North,
        StepDirection::East,
        StepDirection::South,
        StepDirection::West,
    ] {
        let mut cells = vec![from];
        let mut at = from;
        for _ in 0..2 {
            let Some(next) = destination(world, at, direction) else {
                break;
            };
            if next.floor != from.floor
                || world.ground_speed(next) != world.ground_speed(from)
                || world
                    .base()
                    .tile(next.x, next.y, next.floor)
                    .is_none_or(|tile| tile.protection_zone())
            {
                break;
            }
            cells.push(next);
            at = next;
        }
        if cells.len() == 3
            && world
                .base()
                .tile(from.x, from.y, from.floor)
                .is_some_and(|tile| !tile.protection_zone())
        {
            return Some(cells);
        }
    }
    None
}
fn actor_position(at: TilePos) -> ActorPosition {
    ActorPosition {
        x: i32::from(at.x),
        y: i32::from(at.y),
        floor: -i16::from(at.floor),
    }
}

fn capture_write(path: Option<&Path>, name: &str, bytes: impl AsRef<[u8]>) -> TestResult {
    if let Some(path) = path {
        std::fs::write(path.join(name), bytes)
            .map_err(|error| verdict::context(format!("C16a capture file={name}"), error))?;
    }
    Ok(())
}

fn barrier_transition(
    target: ActorPosition,
    expected: &[std::num::NonZeroU32],
    present: bool,
    tiles: &[oteryn_protocol_oteryn::world_map::MapTile],
) -> bool {
    use oteryn_protocol_oteryn::world_map::MapDefinition;
    tiles.iter().find(|tile| tile.position == target).is_some_and(|tile| {
        tile.items.iter().any(|item| matches!(item.definition,
            MapDefinition::Item(reference) if expected.contains(&reference))) == present
    })
}

async fn barrier_frame(
    client: &mut DevClientSession,
    target: ActorPosition,
    expected: &[std::num::NonZeroU32],
    present: bool,
    after: u64,
    wait: Duration,
) -> TestResult<oteryn_dev_client::AppliedDelta<oteryn_protocol_oteryn::world_map::WorldMapViewDelta>> {
    let deadline = tokio::time::Instant::now() + wait;
    verdict::Harness::global().bounded_wait(deadline, "barrier observation");
    loop {
        if let Some(delta) = client.take_events().into_iter().find_map(|event| match event {
            SessionEvent::WorldMap(delta)
                if delta.server_sequence > after
                    && barrier_transition(target, expected, present, &delta.value.tiles) => Some(delta),
            _ => None,
        }) {
            return Ok(delta);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("C16a missing barrier transition at {target:?}: present={present} expected={expected:?}").into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>, world: &BundleWorld) {
    let directory = std::env::var_os("E2E_ALL_CAPTURE_DIR");
    let path = directory.as_deref().map(Path::new);
    let result = verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(async {
        use oteryn_protocol_oteryn::world_map::{MapDefinition, encode_world_map_delta};
        let definitions = provisioning::item_index()?;
        let mut client = connect_world(clients, 0xc1).await?;
        let snapshot = client
            .session_mut()
            .gated_snapshot(oteryn_protocol_oteryn::world_map::STATE_DOMAIN_WORLD_MAP_VIEW)
            .ok_or("C16a client map snapshot absent")?
            .clone();
        capture_write(path, "c16a-snapshot.bin", &snapshot.payload)?;
        capture_write(path, "c16a-snapshot-revision.txt", snapshot.revision.to_string())?;
        let mut expected = String::new();
        for (rune_id, stem, item_ids) in [(3180, "wall", [2128, 10181]), (3156, "growth", [2130, 10182])] {
            let possible = [
                provisioning::item_reference(&definitions, item_ids[0])?,
                provisioning::item_reference(&definitions, item_ids[1])?,
            ];
            let reference = acquire_rune(&mut client, world, rune_id).await?.0;
            let path_to_site = route(world, tile(client.world_spatial().actor_position)?, |at| {
                rune_corridor(world, at).is_some()
            })
            .ok_or("C16a client has no barrier corridor")?;
            walk(&mut client, world, &path_to_site).await?;
            client.take_events();
            let from = tile(client.world_spatial().actor_position)?;
            let cells = rune_corridor(world, from).ok_or("C16a client corridor absent")?;
            let target = actor_position(cells[1]);
            let map = client.session_mut().world_map_snapshot().ok_or("C16a pre-cast map absent")?;
            if !barrier_transition(target, &possible, false, &map.tiles) {
                return Err(format!("C16a target {target:?} missing or already contains the barrier").into());
            }
            let (reply, cast_sequence) = rune_use(&mut client, reference, None, Some(target)).await?;
            if reply != oteryn_protocol_oteryn::world_object::UseDisposition::Committed {
                return Err(format!("C16a client rune {rune_id} refused: {reply:?}").into());
            }
            let placed = barrier_frame(&mut client, target, &possible, true, cast_sequence, Duration::from_secs(8)).await?;
            let actual = placed.value.tiles.iter().find(|tile| tile.position == target)
                .and_then(|tile| tile.items.iter().find_map(|item| match item.definition {
                    MapDefinition::Item(reference) if possible.contains(&reference) => Some(reference),
                    _ => None,
                }))
                .ok_or("C16a barrier definition missing from matched placement")?;
            let expired = barrier_frame(&mut client, target, &possible, false, placed.server_sequence, Duration::from_secs(70)).await?;
            for (phase, delta) in [("placed", placed), ("expired", expired)] {
                let payload = encode_world_map_delta(&delta.value.header, &delta.value)
                    .map_err(|error| format!("C16a {phase} map encoding: {error:?}"))?;
                capture_write(path, &format!("c16a-{stem}-{phase}.bin"), payload)?;
                capture_write(path, &format!("c16a-{stem}-{phase}-meta.txt"), format!(
                    "{} {} {}", delta.base_revision, delta.new_revision, delta.server_sequence,
                ))?;
            }
            expected.push_str(&format!("{stem} {} {} {} {actual}\n", target.x, target.y, target.floor));
        }
        capture_write(path, "c16a-expected.txt", expected)
    })).await.unwrap_or_else(|_| Err("C16a barrier journey deadline".into()));
    match result {
        Ok(()) => evidence(
            "e2e_all scenario=C16a.client status=PASS evidence=raw-domain-17-placement-and-expiry",
        ),
        Err(error) => evidence(&format!(
            "e2e_all scenario=C16a.client status=FAIL evidence={error}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_protocol_oteryn::world_map::{MapDefinition, MapItem, MapOrigin, MapTile};

    #[test]
    fn barrier_requires_exact_item_at_target_and_its_removal() -> TestResult {
        let target = ActorPosition { x: 4, y: 5, floor: 7 };
        let barrier = std::num::NonZeroU32::new(123).ok_or("definition")?;
        let mut tile = MapTile { position: target, items: vec![MapItem {
            definition: MapDefinition::Item(std::num::NonZeroU32::MIN),
            count: 1, sub_type: 0, origin: MapOrigin::BaseOrdinal { ordinal: 0, object_revision: 0 },
            appearance_id: 1,
        }], more: false, ground_speed: 0 };
        assert!(!barrier_transition(target, &[barrier], true, std::slice::from_ref(&tile)));
        tile.items[0].definition = MapDefinition::Item(barrier);
        assert!(barrier_transition(target, &[barrier], true, std::slice::from_ref(&tile)));
        assert!(!barrier_transition(target, &[barrier], false, std::slice::from_ref(&tile)));
        let substitute = std::num::NonZeroU32::new(124).ok_or("definition")?;
        tile.items[0].definition = MapDefinition::Item(substitute);
        assert!(!barrier_transition(target, &[barrier, substitute], false, std::slice::from_ref(&tile)));
        tile.items.clear();
        tile.position.x += 1;
        assert!(!barrier_transition(target, &[barrier], false, std::slice::from_ref(&tile)));
        tile.position = target;
        assert!(barrier_transition(target, &[barrier], false, &[tile]));
        Ok(())
    }
}
