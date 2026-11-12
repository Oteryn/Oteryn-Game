//! Client replay capture M09 — fishing and skinning/gathering yield actions.
//!
//! Concrete player scenario derived from the M09 matrix/re-audit row and the shared
//! inventory-delta seam used by fishing and skinning/gathering yields:
//! 1. Admit the node-boot Character and BUY a fishing rod (3483) and worms (3492).
//! 2. Walk beside authored water and send native USE-WITH until `Committed`; assert one worm
//!    consumed, one fish (3578) minted, and the rod retained, then capture the applied payload.
//! 3. Drain the bait and assert USE-WITH returns `NothingToUse` without changing fish or rod
//!    custody. The full M09 journey separately covers the skinning producer of this same payload.
use super::*;
use oteryn_dev_client::{DevClientSession, ItemEntry};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, decode_use_result,
    encode_item_use_intent,
};
use std::collections::BTreeSet;
use std::fs;
use std::num::NonZeroU32;

const ACTIONS: &[&str] = &[
    "player-acquired-rod-and-bait",
    "fishing-replies-and-deltas",
    "no-bait-refusal",
];

struct Row {
    passed: BTreeSet<&'static str>,
    started: std::time::Instant,
}

impl Row {
    fn new() -> Self {
        Self {
            passed: BTreeSet::new(),
            started: std::time::Instant::now(),
        }
    }
    fn pass(&mut self, action: &'static str, observed: impl std::fmt::Debug) {
        if ACTIONS.contains(&action) && self.passed.insert(action) {
            evidence(&format!("M09_CHECK {action} PASS observed={observed:?}"));
        }
    }
    fn finish(self, result: TestResult) {
        for action in ACTIONS {
            if !self.passed.contains(action) {
                evidence(&format!("M09_CHECK {action} FAIL assertion-not-completed"));
            }
        }
        if let Err(error) = &result {
            evidence(&format!("M09_GAP {error}"));
        }
        evidence(&format!(
            "e2e_all scenario=M09.client status={} seconds={:.3}",
            if result.is_ok() && ACTIONS.iter().all(|a| self.passed.contains(a)) {
                "PASS"
            } else {
                "FAIL"
            },
            self.started.elapsed().as_secs_f64()
        ));
    }
}

#[derive(Clone, Copy, Debug)]
struct MapObject {
    at: TilePos,
    ordinal: u8,
    appearance: u16,
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result = tokio::time::timeout(Duration::from_secs(600), journey(&mut row, clients))
        .await
        .unwrap_or_else(|_| {
            Err("step deadline: M09 journey exceeded 600s; last server reply unavailable".into())
        });
    row.finish(result);
}

fn reference(
    index: &crate::content::item_ref::ItemDefinitionIndex,
    id: u32,
) -> TestResult<NonZeroU32> {
    provisioning::item_reference(index, id)
}

fn units(client: &DevClientSession, wanted: NonZeroU32) -> u64 {
    client.inventory().map_or(0, |inventory| {
        inventory
            .entries
            .iter()
            .chain(inventory.main_backpack.iter())
            .chain(inventory.equipment.iter().map(|entry| &entry.item))
            .filter(|entry| entry.item_definition_ref == wanted)
            .map(|entry| u64::from(entry.count.get()))
            .sum()
    })
}

fn carried(client: &DevClientSession, wanted: NonZeroU32) -> Option<ItemEntry> {
    client
        .inventory()?
        .entries
        .iter()
        .find(|entry| entry.item_definition_ref == wanted)
        .copied()
}

async fn settle(client: &mut DevClientSession) -> TestResult {
    client.service_liveness(Duration::from_millis(50)).await?;
    client.take_events();
    Ok(())
}

fn objects(bytes: &[u8]) -> TestResult<Vec<MapObject>> {
    let mut found = Vec::new();
    oteryn_world_bundle::bundle::visit(bytes, |manifest, sector| {
        for tile in sector.tiles {
            for (ordinal, item) in tile.items.iter().filter(|item| item.depth == 0).enumerate() {
                if let Some(appearance) =
                    crate::map::palette_appearance(&manifest.palette[item.palette as usize].key)
                {
                    found.push(MapObject {
                        at: TilePos {
                            x: tile.x,
                            y: tile.y,
                            floor: sector.floor,
                        },
                        ordinal: u8::try_from(ordinal)
                            .map_err(|_| oteryn_world_bundle::Error::Limit("M09 ordinal".into()))?,
                        appearance,
                    });
                }
            }
        }
        Ok(())
    })?;
    found.sort_by_key(|object| (object.at, object.ordinal));
    Ok(found)
}

fn target(world: &BundleWorld, object: MapObject) -> TestResult<WorldObjectTarget> {
    let key =
        crate::map::view::placement_key(object.at, object.ordinal).ok_or("M09 placement key")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget {
        placement,
        expected_revision: 0,
    })
}

async fn use_on(
    client: &mut DevClientSession,
    source: oteryn_protocol_oteryn::item_view::ItemHandle,
    target: MapUseTarget,
) -> TestResult<(CommandStatus, UseDisposition)> {
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(source),
        use_with: None,
        use_with_map_item: Some(target),
        use_with_position: None,
    })
    .map_err(|error| format!("M09 USE-WITH encode: {error:?}"))?;
    let reply = client
        .session_mut()
        .world_context_use(&payload)
        .await?;
    let disposition = decode_use_result(&reply.payload)
        .map_err(|error| format!("M09 USE-WITH reply status={:?}: {error:?}", reply.status))?;
    Ok((reply.status, disposition))
}

async fn buy(
    client: &mut DevClientSession,
    world: &BundleWorld,
    index: &crate::content::item_ref::ItemDefinitionIndex,
    key: &str,
    count: u32,
) -> TestResult<NonZeroU32> {
    use oteryn_dev_client::client_runtime::npc_shop as shop;
    use oteryn_protocol_oteryn::npc_service as npc;
    let revision = index
        .revision(key)
        .ok_or_else(|| format!("step 1 content definition absent: {key}"))?;
    let wanted = index
        .definition_ref("Item", key, revision)
        .ok_or("step 1 definition reference absent")?;
    if units(client, wanted) >= u64::from(count) {
        return Err(format!(
            "step 1 BUY {key} was not run: observed existing inventory count={}",
            units(client, wanted)
        )
        .into());
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let offers: serde_json::Value = serde_json::from_slice(&fs::read(
        root.join("content/services/trade/trade-00000-00323.json"),
    )?)?;
    let metadata = oteryn_world_bundle::bundle::visit(
        &fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?,
        |_, _| Ok(()),
    )?;
    for record in offers["records"]
        .as_array()
        .ok_or("step 1 trade records absent")?
    {
        let declaration = &record["declaration"];
        if !declaration["offers"].as_array().is_some_and(|rows| {
            rows.iter()
                .any(|offer| offer["item"]["key"] == key && offer["direction"] == "SellToPlayer")
        }) {
            continue;
        }
        let Some(name) = declaration["identity"]["key"]
            .as_str()
            .and_then(|value| value.strip_prefix("oteryn:service.trade."))
        else {
            continue;
        };
        let Some(vendor) = metadata
            .npcs
            .npcs
            .iter()
            .find(|npc| npc.key == format!("oteryn:npc.{name}"))
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
                entities.iter().find(|entry| {
                    entry.kind == oteryn_dev_client::EntityKind::Npc
                        && vendor.placements.iter().any(|p| {
                            entry.position
                                == ActorPosition {
                                    x: i32::from(p.x),
                                    y: i32::from(p.y),
                                    floor: -i16::from(p.floor),
                                }
                        })
                })
            })
            .map(|entry| entry.entity)
            .ok_or("step 1 vendor actor missing after walk")?;
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
                return Err(format!("step 1 NPC {name} {text} reply={reply:?}").into());
            }
            let domain = if text == "hi" {
                npc::STATE_DOMAIN_NPC_CONVERSATION
            } else {
                npc::STATE_DOMAIN_NPC_TRADE_WINDOW
            };
            let delta = client.session_mut().read_gated_delta(domain, 1).await?;
            if text == "hi" {
                if npc::decode_npc_conversation(&delta.payload)
                    .map_err(|e| format!("step 1 dialogue frame: {e:?}"))?
                    .open
                    .is_none()
                {
                    return Err("step 1 greeting delta did not open dialogue".into());
                }
                continue;
            }
            let window = npc::decode_npc_trade_window(&delta.payload)
                .map_err(|e| format!("step 1 trade frame: {e:?}"))?;
            let (offer_index, offer) = window
                .offers
                .iter()
                .enumerate()
                .find(|(_, offer)| {
                    offer.side == npc::NpcTradeSide::Buy
                        && offer.item_definition_ref == wanted.get()
                })
                .ok_or("step 1 BUY offer absent")?;
            let missing = u64::from(count).saturating_sub(units(client, wanted));
            let quantity = u32::try_from(missing.div_ceil(u64::from(offer.count)))?;
            let before = units(client, wanted);
            let reply = shop::buy(
                client.session_mut(),
                &npc::NpcTradeIntent {
                    npc_actor: actor,
                    catalogue_revision: window.open.ok_or("step 1 trade closed")?.1,
                    offer_index: u32::try_from(offer_index)?,
                    side: offer.side,
                    quantity,
                    expected_unit_price: offer.unit_price,
                },
            )
            .await?;
            if reply != npc::NpcIntentDisposition::Ok {
                return Err(format!(
                    "step 1 BUY {key} reply={reply:?}; before={before}; price={}",
                    offer.unit_price
                )
                .into());
            }
            settle(client).await?;
            if units(client, wanted) < u64::from(count) {
                return Err(format!(
                    "step 1 BUY delta missing for {key}: before={before} after={}",
                    units(client, wanted)
                )
                .into());
            }
        }
        return Ok(wanted);
    }
    Err(format!("step 1 no reachable served NPC offer for {key}").into())
}

fn capture_inventory(client: &mut DevClientSession) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    let inventory = client.take_events().into_iter().rev().find_map(|event| match event {
        oteryn_dev_client::SessionEvent::Inventory(delta) => Some(delta.value),
        _ => None,
    });
    let Some(inventory) = inventory else {
        evidence("M09_CLIENT_CAPTURE write-error=no inventory delta followed committed fishing");
        return;
    };
    let payload = match oteryn_protocol_oteryn::item_view::encode_character_inventory(&inventory) {
        Ok(payload) => payload,
        Err(error) => {
            evidence(&format!("M09_CLIENT_CAPTURE write-error=encode {error:?}"));
            return;
        }
    };
    let expected = inventory
        .entries
        .iter()
        .map(|entry| format!("{} {}\n", entry.item_definition_ref, entry.count))
        .collect::<String>();
    let root = std::path::Path::new(&root);
    for result in [
        std::fs::write(root.join("m09-inventory.bin"), payload),
        std::fs::write(root.join("m09-expected.txt"), expected),
    ] {
        if let Err(error) = result {
            evidence(&format!("M09_CLIENT_CAPTURE write-error={error}"));
        }
    }
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let bytes = fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes.clone(),
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let index = provisioning::item_index()?;
    let mut client = connect_world(clients, 0xb9).await?;
    let rod_ref = buy(&mut client, world, &index, "oteryn:item.tibia.i3483", 1).await?;
    let worm_ref = buy(&mut client, world, &index, "oteryn:item.tibia.i3492", 40).await?;
    let fish_ref = reference(&index, 3578)?;
    row.pass(
        "player-acquired-rod-and-bait",
        (units(&client, rod_ref), units(&client, worm_ref)),
    );

    let start = tile(client.world_spatial().actor_position)?;
    let water = objects(&bytes)?
        .into_iter()
        .filter(|object| {
            oteryn_gameplay_items::fishing::is_water(&format!(
                "oteryn:item.tibia.i{}",
                object.appearance
            ))
        })
        .filter_map(|object| {
            route(world, start, |at| {
                at.floor == object.at.floor
                    && at.x.abs_diff(object.at.x).max(at.y.abs_diff(object.at.y)) <= 1
            })
            .map(|path| (path, object))
        })
        .min_by_key(|(path, object)| (path.len(), object.at))
        .ok_or("step 2 no reachable authored fishing water")?;
    walk(&mut client, world, &water.0).await?;
    let rod = carried(&client, rod_ref).ok_or("step 2 rod not directly carried")?;
    let before_fish = units(&client, fish_ref);
    let mut committed = None;
    for attempt in 1..=40 {
        let worms = units(&client, worm_ref);
        let fish = units(&client, fish_ref);
        let reply = use_on(
            &mut client,
            rod.handle,
            MapUseTarget::Object(target(world, water.1)?),
        )
        .await?;
        settle(&mut client).await?;
        match reply {
            (CommandStatus::Accepted, UseDisposition::NothingToUse) if units(&client, worm_ref) == worms && units(&client, fish_ref) == fish => {}
            (CommandStatus::Accepted, UseDisposition::Committed) if units(&client, worm_ref) + 1 == worms && units(&client, fish_ref) == fish + 1 => { committed = Some((attempt, reply)); break; }
            observed => return Err(format!("step 2 fishing reply/delta mismatch: observed={observed:?}; worms={worms}->{}; fish={fish}->{}", units(&client, worm_ref), units(&client, fish_ref)).into()),
        }
    }
    let committed = committed.ok_or_else(|| {
        format!(
            "step 2 fishing: 40 native attempts produced no Committed reply; fish={}->{}",
            before_fish,
            units(&client, fish_ref)
        )
    })?;
    if units(&client, rod_ref) != 1 {
        return Err("step 2 committed fishing consumed rod".into());
    }
    row.pass("fishing-replies-and-deltas", committed);
    capture_inventory(&mut client);

    while units(&client, worm_ref) > 0 {
        let reply = use_on(
            &mut client,
            rod.handle,
            MapUseTarget::Object(target(world, water.1)?),
        )
        .await?;
        settle(&mut client).await?;
        if reply.0 != CommandStatus::Accepted {
            return Err(format!("step 3 bait-drain reply={reply:?}").into());
        }
    }
    let fish = units(&client, fish_ref);
    let refusal = use_on(
        &mut client,
        rod.handle,
        MapUseTarget::Object(target(world, water.1)?),
    )
    .await?;
    settle(&mut client).await?;
    if refusal != (CommandStatus::Accepted, UseDisposition::NothingToUse)
        || units(&client, fish_ref) != fish
        || units(&client, rod_ref) != 1
    {
        return Err(format!("step 3 no-bait refusal mismatch: reply={refusal:?}").into());
    }
    row.pass("no-bait-refusal", refusal);

    Ok(())
}
