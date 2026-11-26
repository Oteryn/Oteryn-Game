//! E2E M09 — fishing and skinning/gathering yield actions.
//!
//! Concrete player scenario derived from the M09 matrix/re-audit row and the
//! production item-craft, fishing-rule, and durable-craft owners:
//! 1. Admit the node-boot Character, walk to a reachable shop NPC, say `hi` and
//!    `trade`, BUY a fishing rod (3483) and worms (3492), and assert dialogue,
//!    trade-window, BUY reply, and inventory deltas.
//! 2. Walk beside authored water and send native USE-WITH (rod on water). Assert
//!    every reply. A miss is `NothingToUse` and changes neither worm nor fish;
//!    continue real attempts until `Committed`, then assert one worm consumed,
//!    one fish (3578) minted, and the rod retained.
//! 3. Send USE-WITH without bait and assert the refusal is `NothingToUse` with
//!    unchanged fish/rod custody (the negative path).
//! 4. Assert custody of the fixture obsidian knife (5908), kill a reachable
//!    creature through native ATTACK, and send
//!    USE-WITH on its served corpse handle. Assert a committed attempt transforms
//!    the corpse and that any yielded item arrives through inventory deltas; a
//!    second use of the spent corpse must be refused without another yield.
//! 5. Logout, freshly admit the same Character, and assert the new game session
//!    retains the rod, consumed-worm count, fish, and any skinning yield.
//!
//! A runtime gap is never preselected: the journey performs fishing and its
//! refusal first, followed by the live corpse hunt and skinning.
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
    "skinning-replies-and-deltas",
    "spent-corpse-refusal",
    "fresh-admission-durable-custody",
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
            "e2e_all scenario=M09.full status={} seconds={:.3}",
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
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result = verdict::indexed_timeout(Duration::from_secs(600), journey(&mut row, clients))
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

pub(super) struct FishingInputs {
    npcs: oteryn_world_bundle::npc::Table,
    spawns: oteryn_world_bundle::spawn::Table,
    water: Vec<MapObject>,
}

impl FishingInputs {
    pub(super) fn load(bytes: &[u8]) -> TestResult<Self> {
        let mut water = Vec::new();
        let fishing_palette = std::sync::OnceLock::new();
        let metadata = oteryn_world_bundle::bundle::visit(bytes, |manifest, sector| {
            let is_fishing = fishing_palette.get_or_init(|| {
                manifest
                    .palette
                    .iter()
                    .map(|entry| {
                        crate::map::palette_appearance(&entry.key).is_some_and(|appearance| {
                            oteryn_gameplay_items::fishing::is_water(&format!(
                                "oteryn:item.tibia.i{appearance}"
                            ))
                        })
                    })
                    .collect::<Vec<_>>()
            });
            for tile in sector.tiles {
                for (ordinal, item) in tile.items.iter().filter(|item| item.depth == 0).enumerate()
                {
                    if is_fishing[item.palette as usize] {
                        water.push(MapObject {
                            at: TilePos {
                                x: tile.x,
                                y: tile.y,
                                floor: sector.floor,
                            },
                            ordinal: u8::try_from(ordinal).map_err(|_| {
                                oteryn_world_bundle::Error::Limit("M09 ordinal".into())
                            })?,
                        });
                    }
                }
            }
            Ok(())
        })?;
        water.sort_by_key(|object| (object.at, object.ordinal));
        Ok(Self {
            npcs: metadata.npcs,
            spawns: metadata.spawns,
            water,
        })
    }
}

pub(super) async fn water_approach(
    client: &mut DevClientSession,
    world: &BundleWorld,
    inputs: &FishingInputs,
    used_water: Option<TilePos>,
) -> TestResult<(WorldObjectTarget, TilePos)> {
    let start = tile(client.world_spatial().actor_position)?;
    let map = world.clone();
    let water = inputs.water.clone();
    let (path, water) = route_work(client, move || {
        let mut banks = std::collections::BTreeMap::new();
        for object in water {
            if used_water == Some(object.at)
                || object.at.floor.abs_diff(start.floor) > 2
                || object.at.x.abs_diff(start.x) > 161
                || object.at.y.abs_diff(start.y) > 161
            {
                continue;
            }
            for dx in -1..=1 {
                for dy in -1..=1 {
                    if let (Some(x), Some(y)) = (
                        object.at.x.checked_add_signed(dx),
                        object.at.y.checked_add_signed(dy),
                    ) {
                        banks
                            .entry(TilePos {
                                x,
                                y,
                                floor: object.at.floor,
                            })
                            .or_insert(object);
                    }
                }
            }
        }
        let path = route(&map, start, |at| banks.contains_key(&at))
            .ok_or("step 2 no reachable authored fishing water")?;
        let bank = path
            .iter()
            .copied()
            .try_fold(start, |at, step| destination(&map, at, step))
            .ok_or("step 2 planned bank route has no compiled landing")?;
        let water = *banks
            .get(&bank)
            .ok_or("step 2 planned bank lost its water target")?;
        Ok((path, water))
    })
    .await?;
    evidence(&format!(
        "M09_ROUTE fishing from={start:?} water={:?} steps={}",
        water.at,
        path.len()
    ));
    walk(client, world, &path).await.map_err(|error| {
        format!(
            "M09 fishing last={:?} {error}",
            client.world_spatial().actor_position
        )
    })?;
    Ok((target(world, water)?, water.at))
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
    let command = client.next_command_id();
    evidence(&format!(
        "M09_CAST command={command} position={:?} last_sequence={}",
        client.world_spatial().actor_position,
        client.last_server_sequence()
    ));
    let reply = client.session_mut().world_context_use(&payload).await?;
    let disposition = decode_use_result(&reply.payload)
        .map_err(|error| format!("M09 USE-WITH reply status={:?}: {error:?}", reply.status))?;
    evidence(&format!(
        "M09_CAST_RESULT command={command} status={:?} disposition={disposition:?}",
        reply.status
    ));
    Ok((reply.status, disposition))
}

pub(super) async fn shop_path(
    client: &mut DevClientSession,
    world: &BundleWorld,
    vendor: &oteryn_world_bundle::npc::Npc,
) -> TestResult<Vec<StepDirection>> {
    npc_approach_path(client, world, vendor)
        .await?
        .ok_or_else(|| format!("no reachable approach to {}", vendor.key).into())
}

pub(super) async fn shop_actor(
    client: &mut DevClientSession,
    vendor: &oteryn_world_bundle::npc::Npc,
    name: &str,
) -> TestResult<oteryn_dev_client::EntityRef> {
    use oteryn_protocol_oteryn::npc_service::TALK_RANGE_TILES;
    // Sam and Lubo wander every 2 s within two tiles of their authored placement.
    // Lubo's reachable approach ends three tiles out, so wait for a live in-range actor.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    let basis = spatial_basis::current(client)?;
    loop {
        let player = client.world_spatial().actor_position;
        if let Some(snapshot) = client.world_entities() {
            let mut near = snapshot
                .iter()
                .filter(|entity| {
                    let at = entity.position;
                    entity.kind == oteryn_dev_client::EntityKind::Npc
                        && at.floor == player.floor
                        && at.x.abs_diff(player.x).max(at.y.abs_diff(player.y)) <= TALK_RANGE_TILES
                        && vendor.placements.iter().any(|p| {
                            at.floor == basis.floor(p.floor)
                                && at
                                    .x
                                    .abs_diff(i32::from(p.x))
                                    .max(at.y.abs_diff(i32::from(p.y)))
                                    <= 2
                        })
                })
                .peekable();
            if let Some(first) = near.next() {
                if near.peek().is_none() {
                    return Ok(first.entity);
                }
                let mut named = std::iter::once(first).chain(near).filter(|entity| {
                    entity
                        .display_name
                        .as_ref()
                        .is_some_and(|label| label.as_str().is_ok_and(|label| label == name))
                });
                if let (Some(actor), None) = (named.next(), named.next()) {
                    return Ok(actor.entity);
                }
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("no unique served {name} in talk range at {player:?}").into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
    }
}

pub(super) async fn buy_fishing_gear(
    client: &mut DevClientSession,
    world: &BundleWorld,
    index: &crate::content::item_ref::ItemDefinitionIndex,
    inputs: &FishingInputs,
    bait_count: u32,
) -> TestResult<(NonZeroU32, NonZeroU32)> {
    use oteryn_dev_client::client_runtime::npc_shop as shop;
    use oteryn_protocol_oteryn::npc_service as npc;

    let vendor = inputs
        .npcs
        .npcs
        .iter()
        .find(|npc| npc.key == "oteryn:npc.lubo")
        .ok_or("step 1 authored fishing vendor Lubo missing")?;
    let path = shop_path(client, world, vendor).await?;
    let from = tile(client.world_spatial().actor_position)?;
    let landing = path
        .iter()
        .copied()
        .try_fold(from, |at, step| destination(world, at, step));
    evidence(&format!(
        "M09_ROUTE vendor=lubo from={from:?} steps={} destination={landing:?}",
        path.len()
    ));
    walk(client, world, &path).await.map_err(|error| {
        format!(
            "M09 vendor Lubo last={:?}: {error}",
            client.world_spatial().actor_position
        )
    })?;
    let actor = shop_actor(client, vendor, "Lubo").await?;
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
            return Err(format!("step 1 NPC Lubo {text} reply={reply:?}").into());
        }
        if text == "hi" {
            let delta = client
                .session_mut()
                .read_gated_delta(npc::STATE_DOMAIN_NPC_CONVERSATION, 1)
                .await?;
            if npc::decode_npc_conversation(&delta.payload)
                .map_err(|e| format!("step 1 dialogue frame: {e:?}"))?
                .open
                .is_none()
            {
                return Err("step 1 greeting delta did not open dialogue".into());
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
        .map_err(|e| format!("step 1 trade frame: {e:?}"))?;
    let catalogue_revision = window.open.ok_or("step 1 trade closed")?.1;

    let rod = reference(index, 3483)?;
    let worms = reference(index, 3492)?;
    for (wanted, quantity, expected_price) in [(rod, 1, 150), (worms, bait_count, 1)] {
        let before = units(client, wanted);
        if before != 0 {
            return Err(format!(
                "step 1 BUY {wanted} not run: inventory already contains {before}"
            )
            .into());
        }
        let (offer_index, offer) = window
            .offers
            .iter()
            .enumerate()
            .find(|(_, offer)| {
                offer.side == npc::NpcTradeSide::Buy && offer.item_definition_ref == wanted.get()
            })
            .ok_or_else(|| format!("step 1 BUY offer absent for {wanted}"))?;
        if offer.count != 1 || offer.unit_price != expected_price {
            return Err(format!(
                "step 1 BUY {wanted} expected one unit per lot at {expected_price}, got {} at {}",
                offer.count, offer.unit_price
            )
            .into());
        }
        let reply = shop::buy(
            client.session_mut(),
            &npc::NpcTradeIntent {
                npc_actor: actor,
                catalogue_revision,
                offer_index: u32::try_from(offer_index)?,
                side: offer.side,
                quantity,
                expected_unit_price: offer.unit_price,
            },
        )
        .await?;
        if reply != npc::NpcIntentDisposition::Ok {
            return Err(format!(
                "step 1 BUY {wanted} reply={reply:?} price={expected_price} quantity={quantity}"
            )
            .into());
        }
        settle(client).await?;
        if !purchase_observed(before, units(client, wanted), u64::from(quantity)) {
            return Err(format!(
                "step 1 BUY delta missing for {wanted}: before={before} after={}",
                units(client, wanted)
            )
            .into());
        }
        evidence(&format!(
            "M09_PURCHASE item={wanted} units={quantity} price={expected_price} vendor=lubo"
        ));
        verdict::Harness::global().progress("npc-purchase");
    }
    Ok((rod, worms))
}

pub(super) async fn wait_for_catch_delta(
    client: &mut DevClientSession,
    worm_ref: NonZeroU32,
    fish_ref: NonZeroU32,
    worms_before: u64,
    fish_before: u64,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while units(client, worm_ref).checked_add(1) != Some(worms_before)
        || fish_before.checked_add(1) != Some(units(client, fish_ref))
    {
        if tokio::time::Instant::now() >= deadline {
            break;
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
    Ok(())
}

pub(super) async fn clear_spare_bait(
    client: &mut DevClientSession,
    worm_ref: NonZeroU32,
) -> TestResult {
    // Move the leftover stack through the player inventory path before testing
    // the no-bait refusal against an unspent fishing tile.
    while let Some(bait) = carried(client, worm_ref) {
        let before = units(client, worm_ref);
        let at = client.world_spatial().actor_position;
        let moved = client
            .session_mut()
            .move_item_counted(oteryn_dev_client::client_runtime::inventory::MoveRequest {
                source: bait.handle,
                destination: oteryn_dev_client::ItemMoveDestination::Ground(
                    oteryn_protocol_oteryn::item_view::WorldTilePosition {
                        x: at.x,
                        y: at.y,
                        floor: at.floor,
                    },
                ),
                count: None,
            })
            .await?;
        settle(client).await?;
        if moved.outcome != oteryn_dev_client::ItemMoveOutcome::Moved
            || units(client, worm_ref) != before - u64::from(bait.count.get())
        {
            return Err(format!(
                "step=clear-spare-bait observed={moved:?} before={before} after={}",
                units(client, worm_ref)
            )
            .into());
        }
    }
    if units(client, worm_ref) != 0 {
        return Err("step=clear-spare-bait observed=remaining-bait-outside-backpack".into());
    }
    Ok(())
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let bytes = fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let inputs = FishingInputs::load(&bytes)?;
    let pin = serde_json::from_slice(&fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes.clone(),
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let index = provisioning::item_index()?;
    let identity =
        character_prep::dedicated_character(clients, "NODE_BOOT_M09_FULL_ACCOUNT").await?;
    let clients = &identity.clients(clients);
    character_prep::declare_seeded(
        "M09.full",
        "151 carried gold and one obsidian knife",
        "rod and bait purchases and live corpse skinning",
    );
    let mut client =
        character_prep::funded_character(clients, 0xb9, 151, &[(3031, 100), (3031, 1), (5908, 1)])
            .await?;
    let knife_ref = reference(&index, 5908)?;
    if units(&client, knife_ref) != 1 || carried(&client, knife_ref).is_none() {
        return Err(
            "step 0 served inventory must contain exactly one carried obsidian knife".into(),
        );
    }
    let (rod_ref, worm_ref) = buy_fishing_gear(&mut client, world, &index, &inputs, 1).await?;
    let fish_ref = reference(&index, 3578)?;
    row.pass(
        "player-acquired-rod-and-bait",
        (units(&client, rod_ref), units(&client, worm_ref)),
    );

    let (water, water_tile) = water_approach(&mut client, world, &inputs, None).await?;
    let rod = carried(&client, rod_ref).ok_or("step 2 rod not directly carried")?;
    let before_fish = units(&client, fish_ref);
    let mut committed = None;
    for attempt in 1..=40 {
        let worms = units(&client, worm_ref);
        let fish = units(&client, fish_ref);
        let reply = use_on(&mut client, rod.handle, MapUseTarget::Object(water.clone())).await?;
        if reply == (CommandStatus::Accepted, UseDisposition::Committed) {
            wait_for_catch_delta(&mut client, worm_ref, fish_ref, worms, fish).await?;
        } else {
            settle(&mut client).await?;
        }
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

    clear_spare_bait(&mut client, worm_ref).await?;
    let (unspent_water, _) = water_approach(&mut client, world, &inputs, Some(water_tile)).await?;
    let fish = units(&client, fish_ref);
    let refusal = use_on(&mut client, rod.handle, MapUseTarget::Object(unspent_water)).await?;
    settle(&mut client).await?;
    if refusal != (CommandStatus::Accepted, UseDisposition::NothingToUse)
        || units(&client, fish_ref) != fish
        || units(&client, rod_ref) != 1
    {
        return Err(format!("step 3 no-bait refusal mismatch: reply={refusal:?}").into());
    }
    row.pass("no-bait-refusal", refusal);

    let knife =
        carried(&client, knife_ref).ok_or("step 4 seeded knife missing from native inventory")?;
    let corpse = hunt_corpse(&mut client, world, &inputs.spawns).await?;
    let corpse_handle = match corpse.detail {
        oteryn_dev_client::EntityDetail::Object {
            item_handle: Some(handle),
            ..
        } => handle,
        _ => return Err("step 4 served corpse has no item handle".into()),
    };
    let corpse_definition =
        object_definition(corpse).ok_or("step 4 served corpse has no object definition")?;
    let fresh_definition = reference(&index, 5969)?.get();
    let spent = reference(&index, 4012)?.get();
    if corpse_definition != fresh_definition {
        return Err(format!("step=skinning-hunt observed=corpse-definition-{corpse_definition} expected={fresh_definition}").into());
    }
    let leather = reference(&index, 5878)?;
    let before_other = non_leather_inventory(&client, leather)?;
    let before_skin = inventory_signature(&client)?;
    let skin_reply = use_on(&mut client, knife.handle, MapUseTarget::Item(corpse_handle)).await?;
    settle(&mut client).await?;
    let after_skin = inventory_signature(&client)?;
    let skin_error = if skin_reply == (CommandStatus::Accepted, UseDisposition::Committed) {
        let spent_definition = client
            .world_entities()
            .and_then(|entities| entities.iter().find(|entry| entry.entity == corpse.entity))
            .copied()
            .and_then(object_definition)
            .ok_or_else(|| {
                format!(
                    "step 4 committed skinning removed the observed corpse: reply={skin_reply:?}"
                )
            })?;
        if spent_definition != spent {
            return Err(format!(
                "step 4 committed skinning returned the wrong spent corpse: reply={skin_reply:?}; definition={spent_definition}; expected={spent}"
            )
            .into());
        }
        if !skin_yield_matches(&before_skin, &after_skin, leather.get())
            || non_leather_inventory(&client, leather)? != before_other
        {
            return Err(format!("step=skinning-yield expected=one-leather-and-unchanged-other-custody before={before_skin:?} after={after_skin:?}").into());
        }
        row.pass(
            "skinning-replies-and-deltas",
            (
                &skin_reply,
                corpse_definition,
                spent_definition,
                &before_skin,
                &after_skin,
            ),
        );
        let second = use_on(&mut client, knife.handle, MapUseTarget::Item(corpse_handle)).await?;
        settle(&mut client).await?;
        if second.0 == CommandStatus::Accepted
            && second.1 != UseDisposition::Committed
            && inventory_signature(&client)? == after_skin
            && client
                .world_entities()
                .and_then(|entities| entities.iter().find(|entry| entry.entity == corpse.entity))
                .copied()
                .and_then(object_definition)
                == Some(spent_definition)
        {
            row.pass("spent-corpse-refusal", (second, spent_definition));
            None
        } else {
            Some(format!(
                "step 4 spent-corpse refusal mismatch: reply={second:?}"
            ))
        }
    } else {
        Some(format!(
            "step 4 native skinning attempt observed reply={skin_reply:?}; server needs this reachable corpse race to expose an admitted skinning rule"
        ))
    };

    let old_session = client.join_snapshot().game_session_id;
    let durable = inventory_signature(&client)?;
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step 5 logout refused".into());
    }
    drop(client);
    let fresh = connect_character(clients, 0xba, 0).await?;
    if fresh.join_snapshot().game_session_id == old_session
        || inventory_signature(&fresh)? != durable
    {
        return Err(
            "step 5 fresh admission changed durable M09 custody or reused the session".into(),
        );
    }
    row.pass("fresh-admission-durable-custody", durable);
    if let Some(error) = skin_error {
        return Err(error.into());
    }
    Ok(())
}

fn inventory_signature(client: &DevClientSession) -> TestResult<Vec<(u32, u32, u32)>> {
    let inventory = client.inventory().ok_or("M09 native inventory absent")?;
    let mut signature: Vec<_> = inventory
        .entries
        .iter()
        .chain(inventory.main_backpack.iter())
        .chain(inventory.equipment.iter().map(|entry| &entry.item))
        .map(|entry| {
            (
                entry.item_definition_ref.get(),
                entry.count.get(),
                entry.sub_type,
            )
        })
        .collect();
    signature.sort_unstable();
    Ok(signature)
}

fn object_definition(entity: oteryn_dev_client::WorldSpatialEntity) -> Option<u32> {
    match entity.detail {
        oteryn_dev_client::EntityDetail::Object {
            item_definition_ref,
            ..
        } => Some(item_definition_ref),
        _ => None,
    }
}

async fn hunt_corpse(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult<oteryn_dev_client::WorldSpatialEntity> {
    use oteryn_protocol_oteryn::attack as combat;
    let positions: Vec<_> = spawns
        .sources
        .iter()
        .flat_map(|source| source.points.iter().map(move |point| (source.floor, point)))
        .filter(|(_, point)| {
            spawns
                .creatures
                .get(point.creature as usize)
                .is_some_and(|creature| creature.key == "oteryn:creature.minotaur")
        })
        .map(|(floor, point)| TilePos {
            x: point.x,
            y: point.y,
            floor,
        })
        .collect();
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        !world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| tile.protection_zone())
            && positions.iter().any(|position| {
                position.floor == at.floor
                    && position.x.abs_diff(at.x).max(position.y.abs_diff(at.y)) <= 1
            })
    })
    .ok_or("step=skinning-hunt missing=reachable-minotaur-spawn-for-corpse-5969 owner=crates/gameplay/items/src/fishing.rs:111")?;
    walk(client, world, &path).await?;
    let at = client.world_spatial().actor_position;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entry| {
                entry.kind == oteryn_dev_client::EntityKind::Creature
                    && entry
                        .display_name
                        .as_ref()
                        .and_then(|name| name.as_str().ok())
                        == Some("Minotaur")
                    && entry.position.floor == at.floor
                    && entry
                        .position
                        .x
                        .abs_diff(at.x)
                        .max(entry.position.y.abs_diff(at.y))
                        <= 1
            })
        })
        .copied()
        .ok_or("step=skinning-hunt observed=no-live-minotaur-in-melee-range")?;
    let previous_corpses: Vec<_> = client
        .world_entities()
        .into_iter()
        .flat_map(|entities| entities.iter())
        .filter(|entry| entry.kind == oteryn_dev_client::EntityKind::Corpse)
        .map(|entry| entry.entity)
        .collect();
    let mut last_position = target.position;
    let reply = client
        .session_mut()
        .attack_target(Some(&target.entity))
        .await?;
    if reply.disposition != combat::AttackIntentDisposition::Ok {
        return Err(format!("step 4 ATTACK reply={reply:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(45);
    loop {
        settle(client).await?;
        let Some(live) = client
            .world_entities()
            .and_then(|entities| entities.iter().find(|entry| entry.entity == target.entity))
        else {
            break;
        };
        last_position = live.position;
        if tokio::time::Instant::now() >= deadline {
            client.session_mut().attack_target(None).await?;
            return Err("step 4 ATTACK timeout: creature remained alive for 45s".into());
        }
    }
    client.session_mut().attack_target(None).await?;
    client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entry| {
                entry.kind == oteryn_dev_client::EntityKind::Corpse
                    && entry.position == last_position
                    && !previous_corpses.contains(&entry.entity)
            })
        })
        .copied()
        .ok_or_else(|| "step 4 kill published no corpse item handle".into())
}

fn purchase_observed(before: u64, after: u64, expected_credit: u64) -> bool {
    expected_credit > 0 && before.checked_add(expected_credit) == Some(after)
}

fn non_leather_inventory(
    client: &DevClientSession,
    leather: NonZeroU32,
) -> TestResult<oteryn_protocol_oteryn::item_view::CharacterInventory> {
    let mut inventory = client
        .inventory()
        .cloned()
        .ok_or("step=skinning-yield missing=inventory")?;
    inventory
        .entries
        .retain(|item| item.item_definition_ref != leather);
    Ok(inventory)
}

fn skin_yield_matches(before: &[(u32, u32, u32)], after: &[(u32, u32, u32)], leather: u32) -> bool {
    let count = |items: &[(u32, u32, u32)]| {
        items
            .iter()
            .filter(|item| item.0 == leather)
            .map(|item| u64::from(item.1))
            .sum::<u64>()
    };
    count(before).checked_add(1) == Some(count(after))
        && before
            .iter()
            .filter(|item| item.0 != leather)
            .eq(after.iter().filter(|item| item.0 != leather))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn review_second_bait_purchase_requires_new_custody() {
        assert!(!purchase_observed(1, 1, 1));
        assert!(purchase_observed(1, 2, 1));
        assert!(!purchase_observed(1, 3, 1));
    }

    #[test]
    fn review_second_skinning_requires_one_leather_and_preserves_other_items() {
        let before = [(41, 1, 0), (73, 2, 0)];
        assert!(!skin_yield_matches(&before, &before, 52));
        assert!(skin_yield_matches(
            &before,
            &[(41, 1, 0), (52, 1, 0), (73, 2, 0)],
            52
        ));
        assert!(!skin_yield_matches(&before, &[(52, 1, 0), (73, 2, 0)], 52));
        assert!(!skin_yield_matches(
            &before,
            &[(41, 1, 0), (52, 1, 0), (73, 3, 0)],
            52
        ));
    }

    #[test]
    fn minotaur_corpse_has_the_obsidian_knife_recipe() {
        use oteryn_gameplay_items::fishing::{GatheringTarget, gathering_rule};

        assert!(matches!(
            gathering_rule(5908, 5969),
            Some((GatheringTarget::Corpse { spent_item: 4012 }, yield_item))
                if yield_item.item == 5878
        ));
    }

    #[test]
    fn row_requires_action_refusal_skinning_and_fresh_admission() {
        let mut row = Row::new();
        for &action in &ACTIONS[..ACTIONS.len() - 1] {
            row.pass(action, "fixture");
        }
        assert!(!ACTIONS.iter().all(|action| row.passed.contains(action)));
        row.pass("fresh-admission-durable-custody", "fixture");
        assert!(ACTIONS.iter().all(|action| row.passed.contains(action)));
    }
}
