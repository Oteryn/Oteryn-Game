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
//! 4. Obtain an obsidian knife (5908) only from an offered NPC trade or ordinary
//!    creature loot, kill a reachable creature through native ATTACK, and send
//!    USE-WITH on its served corpse handle. Assert a committed attempt transforms
//!    the corpse and that any yielded item arrives through inventory deltas; a
//!    second use of the spent corpse must be refused without another yield.
//! 5. Logout, freshly admit the same Character, and assert the new game session
//!    retains the rod, consumed-worm count, fish, and any skinning yield.
//!
//! A runtime gap is never preselected: the journey performs fishing and its
//! refusal first. If legitimate knife/corpse acquisition is unavailable, the
//! observed player-path failure identifies the exact unfinished step.
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
    appearance: u16,
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

    let mut stalled = 0;
    while units(&client, worm_ref) > 0 {
        let worms = units(&client, worm_ref);
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
        stalled = bait_stall(stalled, worms, units(&client, worm_ref))?;
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

    // The production row also owns corpse skinning. Acquisition remains a real
    // player BUY; if the served world offers no legitimate knife route, report
    // that observed step rather than manufacturing one.
    let knife_ref = buy(&mut client, world, &index, "oteryn:item.tibia.i5908", 1).await
        .map_err(|error| { verdict::record_error(verdict::IndexedError::as_error(&error)); verdict::context(format!("step 4 skinning prerequisite after completed fishing/refusal: observed={error}; server needs a reachable player acquisition route for oteryn:item.tibia.i5908"), error) })?;
    let knife =
        carried(&client, knife_ref).ok_or("step 4 bought knife missing from native inventory")?;
    let mut verified_rolls = 0;
    let mut yielded = false;
    for attempt in 1..=SKIN_ATTEMPTS {
        let corpse = hunt_corpse(&mut client, world, &bytes).await?;
        let corpse_handle = match corpse.detail {
            oteryn_dev_client::EntityDetail::Object {
                item_handle: Some(handle),
                ..
            }
            | oteryn_dev_client::EntityDetail::ObjectWithAppearance {
                item_handle: Some(handle),
                ..
            } => handle,
            _ => return Err("step 4 served corpse has no item handle".into()),
        };
        let corpse_definition =
            object_definition(corpse).ok_or("step 4 served corpse has no object definition")?;
        let source_id = index
            .definition_at(NonZeroU32::new(corpse_definition).ok_or("zero corpse definition")?)
            .and_then(|(key, _)| key.strip_prefix("oteryn:item.tibia.i"))
            .and_then(|id| id.parse::<u16>().ok());
        let before_skin = inventory_signature(&client)?;
        let skin_reply = use_on(&mut client, knife.handle, MapUseTarget::Item(corpse_handle)).await?;
        settle(&mut client).await?;
        let after_skin = inventory_signature(&client)?;
        if skin_reply != (CommandStatus::Accepted, UseDisposition::Committed) {
            return Err(format!("step 4 native skinning attempt observed reply={skin_reply:?}").into());
        }
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
        let (kind, reward) = source_id
            .and_then(|id| oteryn_gameplay_items::fishing::gathering_rule(5908, id))
            .ok_or("step 4 committed skinning has no authored corpse yield rule")?;
        let oteryn_gameplay_items::fishing::GatheringTarget::Corpse { spent_item } = kind else {
            return Err("step 4 skinning rule is not a corpse rule".into());
        };
        let reward_ref = provisioning::item_reference(&index, u32::from(reward.item))?;
        let spent_ref = provisioning::item_reference(&index, u32::from(spent_item))?;
        let tally = |signature: &[(u32, u32, u32)]| {
            signature
                .iter()
                .filter(|entry| entry.0 == reward_ref.get())
                .map(|entry| u64::from(entry.1))
                .sum::<u64>()
        };
        let roll_yielded = match skin_attempt(corpse_definition, spent_definition, tally(&before_skin), tally(&after_skin)) {
            Some(true) if spent_definition == spent_ref.get() => true,
            Some(false) if spent_definition == spent_ref.get() && before_skin == after_skin => false,
            _ => return Err(format!(
                "step 4 skinning state mismatch; corpse={corpse_definition}->{spent_definition} yield={} {}->{}",
                reward.item, tally(&before_skin), tally(&after_skin)
            ).into()),
        };
        let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
        let second = tokio::time::timeout_at(deadline, async {
            loop {
                let handle = client.world_entities()
                    .and_then(|entities| entities.get(&corpse.entity))
                    .filter(|entity| object_definition(**entity) == Some(spent_definition))
                    .and_then(|entity| match entity.detail {
                        oteryn_dev_client::EntityDetail::Object { item_handle, .. }
                        | oteryn_dev_client::EntityDetail::ObjectWithAppearance { item_handle, .. } => item_handle,
                        _ => None,
                    }).ok_or("step 4 spent corpse has no current item handle")?;
                let reply = use_on(&mut client, knife.handle, MapUseTarget::Item(handle)).await?;
                if reply.1 != UseDisposition::Exhausted {
                    break Ok::<_, Box<dyn std::error::Error>>(reply);
                }
                client.service_liveness(Duration::from_millis(200)).await?;
                client.take_events();
            }
        }).await??;
        settle(&mut client).await?;
        if spent_corpse_reply(second)
            && inventory_signature(&client)? == after_skin
            && client
                .world_entities()
                .and_then(|entities| entities.iter().find(|entry| entry.entity == corpse.entity))
                .copied()
                .and_then(object_definition)
                == Some(spent_definition)
        {
            verified_rolls += 1;
            yielded |= roll_yielded;
            evidence(&format!("M09_CHECK skinning attempt={attempt} yielded={roll_yielded} corpse={corpse_definition}->{spent_definition} replay={second:?}"));
            if skinning_series_complete(verified_rolls, yielded) {
                row.pass("skinning-replies-and-deltas", (verified_rolls, yielded, corpse_definition, spent_definition));
                row.pass("spent-corpse-refusal", (second, spent_definition));
                break;
            }
        } else {
            return Err(format!("step 4 spent-corpse refusal mismatch: reply={second:?}").into());
        }
    }

    let old_session = client.join_snapshot().game_session_id;
    let durable = inventory_signature(&client)?;
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step 5 logout refused".into());
    }
    drop(client);
    let fresh = connect_world(clients, 0xba).await?;
    if fresh.join_snapshot().game_session_id == old_session
        || inventory_signature(&fresh)? != durable
    {
        return Err(
            "step 5 fresh admission changed durable M09 custody or reused the session".into(),
        );
    }
    row.pass("fresh-admission-durable-custody", durable);
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
        }
        | oteryn_dev_client::EntityDetail::ObjectWithAppearance {
            item_definition_ref,
            ..
        } => Some(item_definition_ref),
        _ => None,
    }
}

async fn hunt_corpse(
    client: &mut DevClientSession,
    world: &BundleWorld,
    bytes: &[u8],
) -> TestResult<oteryn_dev_client::WorldSpatialEntity> {
    use oteryn_protocol_oteryn::attack as combat;
    let metadata = oteryn_world_bundle::bundle::visit(bytes, |_, _| Ok(()))?;
    let positions: Vec<_> = metadata
        .spawns
        .sources
        .iter()
        .flat_map(|source| source.points.iter().map(move |point| (source.floor, point)))
        .filter(|(_, point)| {
            metadata
                .spawns
                .creatures
                .get(point.creature as usize)
                .is_some_and(|creature| creature.key == "oteryn:creature.minotaur")
        })
        .map(|(floor, point)| (TilePos {
            x: point.x,
            y: point.y,
            floor,
        }, point.respawn_ms))
        .collect();
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        !world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| tile.protection_zone())
            && positions.iter().any(|(position, _)| {
                position.floor == at.floor
                    && position.x.abs_diff(at.x).max(position.y.abs_diff(at.y)) <= 1
            })
    })
    .ok_or("step 4 no reachable ordinary creature spawn")?;
    walk(client, world, &path).await?;
    let at = client.world_spatial().actor_position;
    let here = tile(at)?;
    let respawn_ms = positions.iter()
        .filter(|(position, _)| position.floor == here.floor && position.x.abs_diff(here.x).max(position.y.abs_diff(here.y)) <= 1)
        .map(|(_, respawn_ms)| *respawn_ms)
        .max()
        .ok_or("step 4 reached no adjacent authored spawn")?;
    let spawn_deadline = tokio::time::Instant::now() + spawn_wait(respawn_ms);
    let target = loop {
        let target = client
            .world_entities()
            .and_then(|entities| {
                entities.iter().find(|entry| {
                    entry.kind == oteryn_dev_client::EntityKind::Creature
                        && entry.display_name.is_some_and(|name| {
                            name.as_str()
                                .is_ok_and(|label| label.eq_ignore_ascii_case("Minotaur"))
                        })
                        && entry.position.floor == at.floor
                        && entry
                            .position
                            .x
                            .abs_diff(at.x)
                            .max(entry.position.y.abs_diff(at.y))
                            <= 1
                })
            })
            .copied();
        if let Some(target) = target { break target; }
        if tokio::time::Instant::now() >= spawn_deadline {
            return Err("step 4 combat endpoint has no served Minotaur".into());
        }
        settle(client).await?;
    };
    let previous_corpses: BTreeSet<_> = client
        .world_entities()
        .into_iter()
        .flat_map(|entities| entities.iter())
        .filter(|entity| entity.kind == oteryn_dev_client::EntityKind::Corpse)
        .map(|entity| entity.entity)
        .collect();
    let reply = client
        .session_mut()
        .attack_target(Some(&target.entity))
        .await?;
    if reply.disposition != combat::AttackIntentDisposition::Ok {
        return Err(format!("step 4 ATTACK reply={reply:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(45);
    let mut last_position = target.position;
    loop {
        settle(client).await?;
        if let Some(current) = client
            .world_entities()
            .and_then(|entities| entities.get(&target.entity))
        {
            last_position = current.position;
        }
        if !client
            .world_entities()
            .is_some_and(|entities| entities.iter().any(|entry| entry.entity == target.entity))
        {
            break;
        }
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

#[cfg(test)]
mod tests {
    use super::*;
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

fn bait_stall(stalled: u32, before: u64, after: u64) -> TestResult<u32> {
    if after < before {
        return Ok(0);
    }
    if stalled >= 39 {
        return Err(format!(
            "step 3 bait-drain: 40 consecutive casts without progress; bait={after}"
        )
        .into());
    }
    Ok(stalled + 1)
}

#[test]
fn bait_drain_bounds_consecutive_casts_without_progress() -> TestResult {
    let mut stalled = 0;
    for _ in 0..39 {
        stalled = bait_stall(stalled, 1, 1)?;
    }
    assert!(bait_stall(stalled, 1, 1).is_err());
    assert_eq!(bait_stall(stalled, 1, 0)?, 0);
    Ok(())
}

#[cfg(test)]
fn skinning_effect(original: u32, spent: u32, before_yield: u64, after_yield: u64) -> bool {
    original != spent && before_yield.checked_add(1) == Some(after_yield)
}

#[test]
fn skinning_requires_the_expected_material_credit() {
    assert!(!skinning_effect(10, 11, 3, 3));
    assert!(!skinning_effect(10, 10, 3, 4));
    assert!(skinning_effect(10, 11, 3, 4));
}

fn skin_attempt(original: u32, spent: u32, before: u64, after: u64) -> Option<bool> {
    if original == spent { return None; }
    if before.checked_add(1) == Some(after) { return Some(true); }
    (before == after).then_some(false)
}
#[test]
fn round2_spent_corpse_without_yield_is_a_retryable_roll() {
    assert_eq!(skin_attempt(10, 11, 3, 3), Some(false));
    assert_eq!(skin_attempt(10, 11, 3, 4), Some(true));
    assert_eq!(skin_attempt(10, 10, 3, 3), None);
    assert_eq!(skin_attempt(10, 11, 3, 5), None);
}

fn spawn_wait(respawn_ms: u32) -> Duration { Duration::from_millis(u64::from(respawn_ms)) + Duration::from_secs(10) }

#[test]
fn round3_minotaur_wait_covers_the_authored_respawn() {
    assert!(spawn_wait(90_000) > Duration::from_secs(90));
}

const SKIN_ATTEMPTS: usize = 4;

fn skinning_series_complete(verified_rolls: usize, yielded: bool) -> bool {
    verified_rolls > 0 && (yielded || verified_rolls == SKIN_ATTEMPTS)
}

#[test]
fn round3_four_verified_skinning_misses_complete_the_series() {
    assert!(!skinning_series_complete(0, false));
    assert!(!skinning_series_complete(3, false));
    assert!(skinning_series_complete(4, false));
    assert!(skinning_series_complete(1, true));
}

fn spent_corpse_reply(reply: (CommandStatus, UseDisposition)) -> bool {
    reply == (CommandStatus::Accepted, UseDisposition::NothingToUse)
}

#[test]
fn round4_spent_corpse_needs_semantic_refusal() {
    assert!(spent_corpse_reply((CommandStatus::Accepted, UseDisposition::NothingToUse)));
    for unrelated in [UseDisposition::StaleState, UseDisposition::Exhausted, UseDisposition::Committed] {
        assert!(!spent_corpse_reply((CommandStatus::Accepted, unrelated)));
    }
}
