//! C16a client capture: real rune USE replies publish raw domain-17 map payloads.
use super::*;
use super::{
    row_connect_world as connect_world,
    row_finish as row_logout,
};
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

fn capture_write(path: &Path, name: &str, bytes: impl AsRef<[u8]>) -> TestResult {
    std::fs::write(path.join(name), bytes)?;
    Ok(())
}

struct CapturedMapDelta {
    server_sequence: u64,
    base_revision: u64,
    new_revision: u64,
    payload: Vec<u8>,
}

async fn capture_map_delta(client: &mut DevClientSession) -> TestResult<CapturedMapDelta> {
    let delta = client
        .session_mut()
        .read_gated_delta(
            oteryn_protocol_oteryn::world_map::STATE_DOMAIN_WORLD_MAP_VIEW,
            1,
        )
        .await?;
    Ok(CapturedMapDelta {
        server_sequence: delta.server_sequence,
        base_revision: delta.base_revision,
        new_revision: delta.new_revision,
        payload: delta.payload,
    })
}

pub(super) async fn run_packet(clients: &SeamClients<'_>, world: &BundleWorld) {
    let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    let result: TestResult = row_complete(async {
        let path = Path::new(&directory);
        let mut client = connect_world(clients, 0xc1).await?;
        let row_deadline = tokio::time::Instant::now() + Duration::from_secs(300);
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
            let snapshot = client
                .session_mut()
                .gated_snapshot(oteryn_protocol_oteryn::world_map::STATE_DOMAIN_WORLD_MAP_VIEW)
                .ok_or("C16a client map snapshot absent")?
                .clone();
            capture_write(path, "c16a-snapshot.bin", &snapshot.payload)?;
            capture_write(
                path,
                "c16a-snapshot-revision.txt",
                snapshot.revision.to_string(),
            )?;
            let snapshot_map =
                oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(&snapshot.payload)
                    .map_err(|error| format!("C16a client snapshot decode: {error:?}"))?;
            let mut expected = String::new();
            for (rune_id, stem) in [(3180, "wall"), (3156, "growth")] {
                let reference = acquire_rune(&mut client, world, rune_id).await?.0;
                let path_to_site =
                    route(world, tile(client.world_spatial().actor_position)?, |at| {
                        rune_corridor(world, at).is_some()
                    })
                    .ok_or("C16a client has no barrier corridor")?;
                walk(&mut client, world, &path_to_site).await?;
                client.take_events();
                let from = tile(client.world_spatial().actor_position)?;
                let cells = rune_corridor(world, from).ok_or("C16a client corridor absent")?;
                let target = actor_position(cells[1]);
                let (reply, _) = rune_use(&mut client, reference, None, Some(target)).await?;
                if reply != oteryn_protocol_oteryn::world_object::UseDisposition::Committed {
                    return Err(format!("C16a client rune {rune_id} refused: {reply:?}").into());
                }
                let placed = capture_map_delta(&mut client).await?;
                capture_write(path, &format!("c16a-{stem}-placed.bin"), &placed.payload)?;
                capture_write(
                    path,
                    &format!("c16a-{stem}-placed-meta.txt"),
                    format!(
                        "{} {} {}",
                        placed.base_revision, placed.new_revision, placed.server_sequence
                    ),
                )?;
                let visible = oteryn_protocol_oteryn::world_map::decode_world_map_delta(
                    &snapshot_map.header,
                    &placed.payload,
                )
                .map_err(|error| format!("C16a client placement decode: {error:?}"))?;
                let visible_reference = visible
                    .tiles
                    .iter()
                    .find(|tile| tile.position == target)
                    .and_then(|tile| {
                        tile.items
                            .iter()
                            .rev()
                            .find_map(|item| match item.definition {
                                oteryn_protocol_oteryn::world_map::MapDefinition::Item(
                                    reference,
                                ) => Some(reference),
                                oteryn_protocol_oteryn::world_map::MapDefinition::Terrain(_) => {
                                    None
                                }
                            })
                    })
                    .ok_or("C16a client placement omitted barrier item")?;
                expected.push_str(&format!(
                    "{stem} {} {} {} {}\n",
                    target.x, target.y, target.floor, visible_reference
                ));
                let expired = capture_map_delta(&mut client).await?;
                capture_write(path, &format!("c16a-{stem}-expired.bin"), &expired.payload)?;
                capture_write(
                    path,
                    &format!("c16a-{stem}-expired-meta.txt"),
                    format!(
                        "{} {} {}",
                        expired.base_revision, expired.new_revision, expired.server_sequence
                    ),
                )?;
            }
            capture_write(path, "c16a-expected.txt", expected)?;
            Ok(())
        })
    .await
    .unwrap_or_else(|error| Err(error.into()));
        tokio::time::timeout(Duration::from_secs(10), row_logout(&mut client)).await??;
        row_result
    }).await.and_then(|result| result);
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

    #[test]
    fn actor_coordinates_keep_floor_sign() {
        assert_eq!(
            actor_position(TilePos {
                x: u16::MAX,
                y: 0,
                floor: 15
            }),
            ActorPosition {
                x: 65535,
                y: 0,
                floor: -15
            }
        );
    }
}
