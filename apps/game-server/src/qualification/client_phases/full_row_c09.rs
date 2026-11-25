//! C09 full-row player scenario, derived from the matrix, Crystal re-audit, and the native
//! food item-use and periodic-vitals owners:
//! 1. Open the tavern door, buy meat from Frodo, and verify the exact stack increase.
//! 2. Cast an available mana-cost spell and require its Cast reply and mana debit. Observe
//!    native melee from a seeded rat before stepping to adjacent protection.
//! 3. Send USE for the bought meat handle; require `Committed`, an inventory delta, and an exact
//!    one-unit stack decrease.
//! 4. Immediately send USE for the remaining meat as the refusal path; require `Exhausted` and
//!    no inventory or vitals change.
//! 5. While fed, require positive server `ActorVitals` deltas for both health and mana.
//! 6. Logout (retrying only an observed `InFight` refusal), make a fresh admission, require a new
//!    session plus the durable one-unit meat debit, and require further positive HP or mana
//!    regeneration after admission, proving that the food-funded regeneration state survived.
//! 7. Eat up to the donor stomach cap, require Full on the next use, and preserve that unit.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use std::collections::BTreeSet;
use std::num::NonZeroU32;

const STEPS: &[&str] = &[
    "food-purchase",
    "damage-and-mana-debit",
    "food-stack-debit",
    "food-refusal",
    "fed-hp-mana-regeneration",
    "fresh-admission-regeneration",
    "full-stomach-refusal",
];

struct Row {
    passed: BTreeSet<&'static str>,
    invalid: bool,
    started: std::time::Instant,
}
impl Row {
    fn new() -> Self {
        Self {
            passed: BTreeSet::new(),
            invalid: false,
            started: std::time::Instant::now(),
        }
    }
    fn pass(&mut self, step: &'static str, observed: impl std::fmt::Debug) {
        if !STEPS.contains(&step) || !self.passed.insert(step) {
            self.invalid = true;
        }
        evidence(&format!("C09_CHECK {step} PASS observed={observed:?}"));
    }
    fn finish(self, result: TestResult) {
        for step in STEPS {
            if !self.passed.contains(step) {
                evidence(&format!("C09_CHECK {step} FAIL assertion-not-completed"));
            }
        }
        if let Err(error) = &result {
            evidence(&format!(
                "C09_GAP step={} observed={error}",
                STEPS
                    .iter()
                    .find(|s| !self.passed.contains(**s))
                    .unwrap_or(&"completion")
            ));
        }
        evidence(&format!(
            "e2e_all scenario=C09.full status={} seconds={:.3} evidence={}",
            if result.is_ok() && !self.invalid && self.passed.len() == STEPS.len() {
                "PASS"
            } else {
                "FAIL"
            },
            self.started.elapsed().as_secs_f64(),
            if result.is_ok() {
                "native-food-regeneration-lifecycle"
            } else {
                "runtime-step-failed-see-C09_GAP"
            }
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result = verdict::indexed_timeout(Duration::from_secs(300), journey(&mut row, clients))
        .await
        .unwrap_or_else(|_| Err("timeout waiting for required C09 server reply/delta".into()));
    row.finish(result);
}

fn quantity(client: &DevClientSession, reference: NonZeroU32) -> TestResult<u64> {
    Ok(client
        .inventory()
        .ok_or("inventory snapshot absent")?
        .entries
        .iter()
        .filter(|item| item.item_definition_ref == reference)
        .map(|item| u64::from(item.count.get()))
        .sum())
}

async fn wait_quantity(
    client: &mut DevClientSession,
    reference: NonZeroU32,
    expected: u64,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        for event in client.take_events() {
            if matches!(event, SessionEvent::Inventory(_))
                && quantity(client, reference)? == expected
            {
                return Ok(());
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "inventory delta timeout expected={expected} observed={}",
                quantity(client, reference)?
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }
}

// The pinned Frodo behavior wanders two tiles from his authored placement.
const FRODO_INTERIOR_RANGE: u32 = 2;
const FRODO_WANDER_RADIUS: u16 = 2;

fn meat_seller_at(
    entity: &oteryn_dev_client::WorldSpatialEntity,
    placements: &[oteryn_world_bundle::npc::Placement],
    player: ActorPosition,
) -> bool {
    entity.kind == oteryn_dev_client::EntityKind::Npc
        && entity
            .display_name
            .is_none_or(|name| name.as_str().is_ok_and(|name| name == "Frodo"))
        && entity.position.floor == player.floor
        && entity
            .position
            .x
            .abs_diff(player.x)
            .max(entity.position.y.abs_diff(player.y))
            <= oteryn_protocol_oteryn::npc_service::TALK_RANGE_TILES
        && placements.iter().any(|p| {
            entity.position.floor == -i16::from(p.floor)
                && entity
                    .position
                    .x
                    .abs_diff(i32::from(p.x))
                    .max(entity.position.y.abs_diff(i32::from(p.y)))
                    <= u32::from(FRODO_WANDER_RADIUS)
        })
}

#[test]
fn c09_acquires_frodo_after_he_wanders_from_placement() -> TestResult {
    use oteryn_dev_client::{EntityDetail, EntityKind, EntityRef, WorldSpatialEntity};
    use oteryn_protocol_oteryn::entity_names::EntityDisplayName;
    use oteryn_world_bundle::npc::{Direction, Placement};

    let placement = Placement {
        x: 32358,
        y: 32209,
        floor: -7,
        direction: Direction::North,
    };
    let player = ActorPosition {
        x: 32360,
        y: 32209,
        floor: 7,
    };
    let mut entity = WorldSpatialEntity {
        motion: None,
        display_name: Some(EntityDisplayName::new("Frodo").map_err(|_| "Frodo display name")?),
        kind: EntityKind::Npc,
        entity: EntityRef {
            identity: [7; 16],
            generation: 1,
        },
        position: ActorPosition {
            x: 32356,
            y: 32209,
            floor: 7,
        },
        detail: EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    };
    assert!(meat_seller_at(&entity, &[placement], player));
    entity.display_name = None;
    assert!(meat_seller_at(&entity, &[placement], player));
    entity.position.x -= 1;
    assert!(!meat_seller_at(&entity, &[placement], player));
    entity.position.x += 1;
    entity.display_name = Some(EntityDisplayName::new("Sam").map_err(|_| "Sam display name")?);
    assert!(!meat_seller_at(&entity, &[placement], player));
    Ok(())
}

fn visible_frodo(
    client: &DevClientSession,
    placements: &[oteryn_world_bundle::npc::Placement],
) -> Option<oteryn_dev_client::EntityRef> {
    let player = client.world_spatial().actor_position;
    let mut candidates = client
        .world_entities()?
        .iter()
        .filter(|entry| meat_seller_at(entry, placements, player));
    let first = candidates.next()?;
    candidates.next().is_none().then_some(first.entity)
}

async fn walk_frodo_range(
    client: &mut DevClientSession,
    world: &BundleWorld,
    in_range: impl Fn(TilePos) -> bool,
) -> TestResult {
    let mut blocked = std::collections::BTreeSet::new();
    'replan: loop {
        let from = tile(client.world_spatial().actor_position)?;
        let path = route_avoiding(world, from, &in_range, |at| !blocked.contains(&at))
            .ok_or_else(|| format!("no STEP route to Frodo within talk range from {from:?} avoiding {blocked:?}"))?;
        for direction in path {
            let before = client.world_spatial().actor_position;
            let expected = destination(world, tile(before)?, direction)
                .ok_or("Frodo route landing disappeared")?;
            let mut result = client.step_retrying(direction).await?;
            for attempt in 1..=6 {
                let Some(delay) = town_route_retry_delay(
                    result.disposition,
                    client.world_spatial().actor_position == before,
                    attempt,
                ) else {
                    break;
                };
                tokio::time::sleep(delay).await;
                result = client.step_retrying(direction).await?;
            }
            let after = client.world_spatial().actor_position;
            if result.status == CommandStatus::Accepted && result.disposition == StepDisposition::Blocked
                && after == before && blocked.len() < 8 && blocked.insert(expected)
            {
                evidence(&format!("C09_FRODO_ROUTE blocked={expected:?} from={before:?}"));
                continue 'replan;
            }
            if result.status != CommandStatus::Accepted || result.disposition != StepDisposition::Moved
                || result.world_spatial_delta.is_none() || tile(after)? != expected
            {
                return Err(format!("Frodo STEP {direction:?} from={before:?} expected={expected:?} observed={after:?}: {result:?}").into());
            }
            verdict::Harness::global().progress("walk");
            client.take_events();
        }
        return Ok(());
    }
}

async fn walk_to_frodo(
    client: &mut DevClientSession,
    world: &BundleWorld,
    here: TilePos,
    within_talk_range: impl Fn(TilePos) -> bool,
) -> TestResult<BundleWorld> {
    use crate::map::door::{AuthoredClosedDoor, DoorUse};
    use crate::map::view::{MapFacts, placement_of};
    use oteryn_protocol_oteryn::world_map::MapOrigin;

    let door = TilePos {
        x: 32357,
        y: 32206,
        floor: -7,
    };
    let oracle = world
        .fresh_channel_for_test()
        .map_err(|error| format!("tavern map: {error:?}"))?;
    let doors = oracle.facts().doors();
    let (key, ordinal) = doors
        .keys()
        .find_map(|key| {
            let (at, ordinal) = placement_of(key)?;
            (at == door
                && matches!(
                    doors.authored_closed(key),
                    Some(AuthoredClosedDoor::Plain | AuthoredClosedDoor::KeyUnlocked)
                )
                && oracle.facts().opening_door_enterable(oracle.base(), key))
            .then_some((key, ordinal))
        })
        .ok_or("Frodo's tavern door is absent or lacks an enterable open state")?;
    let path = route(world, here, |at| {
        at.floor == door.floor && at.x.abs_diff(door.x).saturating_add(at.y.abs_diff(door.y)) == 1
    })
    .ok_or("no STEP route to Frodo's tavern door")?;
    walk(client, world, &path).await?;

    let tile = oracle
        .base()
        .tile(door.x, door.y, door.floor)
        .ok_or("tavern door tile absent")?;
    let id = tile
        .ids()
        .iter()
        .zip(tile.depths())
        .filter_map(|(&id, &depth)| (depth == 0).then_some(id))
        .nth(usize::from(ordinal))
        .ok_or("tavern door ordinal absent")?;
    // Compute the expected face in an isolated map. The route uses this state only
    // after the native command and its map delta prove the live door opened.
    if doors.toggle(key, 0, false, 1, false)
        != Some(DoorUse::Toggled {
            revision: 1,
            open: true,
        })
    {
        return Err("authored tavern door does not open at level one".into());
    }
    let open = oracle
        .facts()
        .base_entry(door, ordinal, id)
        .ok_or("open tavern door face absent")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    client.service_liveness(Duration::from_millis(250)).await?;
    client.take_events();
    let mut used = client.use_object(&placement, 0).await?;
    for _ in 0..2 {
        if used.status != CommandStatus::Rejected || used.disposition != UseDisposition::Rejected {
            break;
        }
        // A rejected command can be retried with the same closed revision. A
        // commit or stale revision must never be retried as a door toggle.
        client.service_liveness(Duration::from_millis(250)).await?;
        used = client.use_object(&placement, 0).await?;
    }
    if used.status != CommandStatus::Accepted || used.disposition != UseDisposition::Committed {
        return Err(format!(
            "tavern door USE reply={used:?} at {:?} vitals={:?}",
            client.world_spatial().actor_position,
            client.actor_vitals()
        )
        .into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let position = ActorPosition {
        x: i32::from(door.x),
        y: i32::from(door.y),
        floor: -i16::from(door.floor),
    };
    loop {
        let opened = client.take_events().into_iter().any(|event| {
            matches!(event, SessionEvent::WorldMap(delta)
                if delta.server_sequence > used.result_server_sequence
                    && delta.value.tiles.iter().any(|tile| tile.position == position
                        && tile.items.iter().any(|item| item.definition == open.definition
                            && item.appearance_id == open.appearance_id
                            && item.origin == MapOrigin::BaseOrdinal { ordinal, object_revision: 1 })))
        });
        if opened {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(
                "committed tavern door USE did not publish the open face at revision 1".into(),
            );
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
    evidence(&format!(
        "C09_TAVERN_DOOR placement={door:?} revision=1 disposition=Committed"
    ));
    walk_frodo_range(client, &oracle, within_talk_range).await?;
    Ok(oracle)
}

pub(super) async fn buy_meat(
    client: &mut DevClientSession,
    world: &BundleWorld,
    reference: NonZeroU32,
    units: u32,
) -> TestResult {
    buy_meat_from_frodo(client, world, reference, units, false).await
}

async fn buy_meat_from_frodo(
    client: &mut DevClientSession,
    world: &BundleWorld,
    reference: NonZeroU32,
    units: u32,
    tavern_interior: bool,
) -> TestResult {
    use oteryn_dev_client::client_runtime::npc_shop as runtime;
    use oteryn_protocol_oteryn::npc_service as wire;
    let metadata = oteryn_world_bundle::bundle::visit(
        &std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?,
        |_, _| Ok(()),
    )?;
    let here = tile(client.world_spatial().actor_position)?;
    let npc = metadata
        .npcs
        .npcs
        .iter()
        .find(|npc| npc.key == "oteryn:npc.frodo")
        .ok_or("Frodo absent from compiled World bundle")?;
    let approach = if tavern_interior {
        // The interior landing is two tiles from Frodo's authored position.
        // Do not route onto the tile in front of the counter where he can wander.
        FRODO_INTERIOR_RANGE
    } else {
        wire::TALK_RANGE_TILES - u32::from(FRODO_WANDER_RADIUS)
    };
    let within_talk_range = |at: TilePos| {
        npc.placements.iter().any(|p| {
            p.floor == at.floor && u32::from(at.x.abs_diff(p.x).max(at.y.abs_diff(p.y))) <= approach
        })
    };
    if tavern_interior {
        walk_frodo_range(client, world, within_talk_range).await?;
    } else {
        let path = route(world, here, within_talk_range).ok_or("no walk route to Frodo within talk range")?;
        walk(client, world, &path).await?;
    }
    if tavern_interior {
        verdict::Harness::global().progress("observe-frodo");
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let actor = loop {
        if let Some(actor) = visible_frodo(client, &npc.placements) {
            break actor;
        }
        if tokio::time::Instant::now() >= deadline {
            let visible_npcs: Vec<_> = client
                .world_entities()
                .into_iter()
                .flat_map(|entities| entities.iter())
                .filter(|entry| entry.kind == oteryn_dev_client::EntityKind::Npc)
                .take(8)
                .map(|entry| (entry.position, entry.display_name))
                .collect();
            return Err(format!(
                "need=unique-visible-Frodo-within-talk-range;observed=none-or-ambiguous;visible-npcs={visible_npcs:?};position={:?};last-frame={}",
                client.world_spatial().actor_position, client.last_server_sequence()
            ).into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
    };
    for text in ["hi", "trade"] {
        if tavern_interior {
            verdict::Harness::global().progress(if text == "hi" { "greet-frodo" } else { "open-frodo-trade" });
        }
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
    if tavern_interior {
        verdict::Harness::global().progress("observe-frodo-offer");
    }
    let delta = client
        .session_mut()
        .read_gated_delta(wire::STATE_DOMAIN_NPC_TRADE_WINDOW, 1)
        .await?;
    let details_selected = client
        .selected_capabilities()
        .contains(&oteryn_protocol_oteryn::trade_details::CAPABILITY_TRADE_DETAILS_V1);
    let window = wire::decode_npc_trade_window_for(&delta.payload, details_selected)
        .map_err(|e| format!("trade window reply={e:?}"))?;
    let (seller, catalogue_revision) = window.open.ok_or("trade window closed")?;
    if seller != actor {
        return Err("trade window belongs to a different NPC".into());
    }
    let (index, offer) = window
        .offers
        .iter()
        .enumerate()
        .find(|(_, offer)| {
            offer.side == wire::NpcTradeSide::Buy && offer.item_definition_ref == reference.get()
        })
        .ok_or("seller omitted meat offer")?;
    let before = quantity(client, reference)?;
    client.take_events();
    if tavern_interior {
        verdict::Harness::global().progress("buy-frodo-meat");
    }
    let reply = runtime::buy(
        client.session_mut(),
        &wire::NpcTradeIntent {
            npc_actor: actor,
            catalogue_revision,
            offer_index: u32::try_from(index)?,
            side: offer.side,
            quantity: units,
            expected_unit_price: offer.unit_price,
        },
    )
    .await?;
    if reply != wire::NpcIntentDisposition::Ok {
        return Err(format!("buy reply={reply:?}").into());
    }
    wait_quantity(
        client,
        reference,
        before + u64::from(units) * u64::from(offer.count),
    )
    .await?;
    Ok(())
}

async fn damage_and_spend_mana(
    client: &mut DevClientSession,
    world: &BundleWorld,
) -> TestResult<(u32, u32, u32, u32)> {
    let from = tile(client.world_spatial().actor_position)?;
    let protected = |at: TilePos| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .map(|tile| tile.protection_zone())
    };
    let outside = route(world, from, |at| {
        let mut neighbours = [
            StepDirection::North,
            StepDirection::East,
            StepDirection::South,
            StepDirection::West,
        ]
        .into_iter()
        .filter_map(|step| destination(world, at, step))
        .filter(|next| next.floor == at.floor)
        .filter_map(protected);
        at.floor == from.floor
            && protected(at) == Some(false)
            && neighbours.clone().any(|protected| protected)
            && neighbours.any(|protected| !protected)
    })
    .ok_or("no native route to an open damage encounter tile")?;
    walk(client, world, &outside).await?;
    client.session_mut().attack_target(None).await?;
    let (mana, mana_after) = spend_mana(client).await?;
    let before = *client.actor_vitals().ok_or("pre-combat vitals absent")?;
    client.take_events();
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let spawn = request_spawn(session, "oteryn:creature.rat".into());
    tokio::pin!(spawn);
    let answer = loop {
        tokio::select! {
            answer = &mut spawn => break answer.map_err(|error| format!("C09 melee rat fixture: {error}"))?,
            result = client.service_liveness(Duration::from_millis(100)) => { result?; }
        }
    };
    let threat = parse_spawned_entity(&answer)?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    let mut damage = None;
    let mut observed = false;
    let health = loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        observed |= client.world_entities().is_some_and(|entities| {
            entities
                .get(&threat)
                .is_some_and(|entity| entity.kind == oteryn_dev_client::EntityKind::Creature)
        });
        damage = damage.or_else(|| {
            client
                .take_events()
                .into_iter()
                .find_map(|event| match event {
                    SessionEvent::ActorVitals(delta) if delta.value.health < before.health => {
                        Some(delta.value.health)
                    }
                    _ => None,
                })
        });
        if observed && let Some(health) = damage {
            break health;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "C09 melee rat visible={observed} damage={damage:?} from {}",
                before.health
            )
            .into());
        }
    };
    let here = tile(client.world_spatial().actor_position)?;
    let safe = route(world, here, |at| {
        at.floor == here.floor && protected(at) == Some(true)
    })
    .ok_or("no native retreat into protection after the C09 damage delta")?;
    walk(client, world, &safe).await?;
    Ok((before.health, health, mana, mana_after))
}

pub(super) async fn spend_mana(client: &mut DevClientSession) -> TestResult<(u32, u32)> {
    use oteryn_protocol_oteryn::actor_spell_catalog as catalog;
    let snapshot = client
        .session_mut()
        .gated_snapshot(catalog::STATE_DOMAIN_ACTOR_SPELL_CATALOG)
        .ok_or("spell catalogue absent")?;
    let entries =
        catalog::decode_catalog(&snapshot.payload).map_err(|e| format!("catalogue reply={e:?}"))?;
    let spell = ["utevo lux", "exura infir"]
        .into_iter()
        .find_map(|words| {
            entries.iter().find(|entry| {
                entry.available
                    && entry.words == words
                    && entry.mana > 0
                    && entry.rune_item.is_none()
            })
        })
        .ok_or("need=available-Light-or-Magic-Patch-for-mana-deficit")?;
    let mana = client.actor_vitals().ok_or("pre-cast vitals absent")?.mana;
    let cast = client
        .cast_spell(
            NonZeroU32::new(spell.cast_index).ok_or("zero cast index")?,
            SpellTarget::None,
            false,
        )
        .await?;
    if cast.status != CommandStatus::Accepted || cast.disposition != SpellCastDisposition::Cast {
        return Err(format!("spell reply={cast:?}").into());
    }
    let mana_after = client.actor_vitals().ok_or("post-cast vitals absent")?.mana;
    if mana_after >= mana {
        return Err(format!("Cast reply missing mana debit {mana}->{mana_after}").into());
    }
    Ok((mana, mana_after))
}

async fn wait_regeneration(
    client: &mut DevClientSession,
    health: u32,
    mana: u32,
    require_both: bool,
) -> TestResult<(u32, u32)> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(25);
    let (mut best_health, mut best_mana) = (health, mana);
    loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        for event in client.take_events() {
            if let SessionEvent::ActorVitals(delta) = event {
                best_health = best_health.max(delta.value.health);
                best_mana = best_mana.max(delta.value.mana);
            }
        }
        if (best_health > health && best_mana > mana)
            || (!require_both && (best_health > health || best_mana > mana))
        {
            return Ok((best_health, best_mana));
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "regeneration timeout hp={health}->{best_health} mana={mana}->{best_mana}"
            )
            .into());
        }
    }
}

async fn accepted_logout(client: &mut DevClientSession) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(70);
    loop {
        let reply = client.session_mut().logout().await?;
        match reply.result {
            Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) => return Ok(()),
            Some(oteryn_protocol_oteryn::logout::LogoutResult::InFight { .. }) => {}
            _ => return Err(format!("logout reply={reply:?}").into()),
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("logout remained refused; last={reply:?}").into());
        }
        client.service_liveness(Duration::from_millis(250)).await?;
        client.take_events();
    }
}

async fn food_remaining(
    client: &mut DevClientSession,
    after_result: Option<u64>,
) -> TestResult<i32> {
    use oteryn_protocol_oteryn::rest_status as wire;
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_REST_STATUS_V1,
            wire::COMMAND_TYPE_REST_STATUS_QUERY,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("rest status refused: {reply:?}").into());
    }
    if after_result.is_some_and(|sequence| {
        !super::full_row_c02::frame_follows_result(reply.server_sequence, sequence)
    }) {
        return Err(
            format!("rest status did not follow food refusal {after_result:?}: {reply:?}").into(),
        );
    }
    Ok(wire::RestStatus::decode(&reply.payload)
        .ok_or("missing rest status payload")?
        .food_remaining_ms)
}

async fn prove_full_stomach(client: &mut DevClientSession, meat: NonZeroU32) -> TestResult<u64> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(25);
    loop {
        let before = quantity(client, meat)?;
        let handle = client
            .inventory()
            .and_then(|view| {
                view.entries
                    .iter()
                    .find(|entry| entry.item_definition_ref == meat)
            })
            .map(|entry| entry.handle)
            .ok_or("need=meat-left-for-full-stomach-refusal")?;
        let used = client.use_item(handle).await?;
        if used.status != CommandStatus::Accepted {
            return Err(format!("food command not accepted: {used:?}").into());
        }
        match used.disposition {
            UseDisposition::Full => {
                food_remaining(client, Some(used.result_server_sequence)).await?;
                let after = quantity(client, meat)?;
                if after != before {
                    return Err(format!("Full refusal changed meat count {before}->{after}").into());
                }
                return Ok(before);
            }
            UseDisposition::Committed => wait_quantity(client, meat, before - 1).await?,
            UseDisposition::Exhausted => {
                food_remaining(client, Some(used.result_server_sequence)).await?;
                let after = quantity(client, meat)?;
                if after != before {
                    return Err(
                        format!("Exhausted refusal changed meat count {before}->{after}").into(),
                    );
                }
                client.service_liveness(Duration::from_millis(250)).await?;
                client.take_events();
            }
            _ => return Err(format!("need=Full-without-debit;observed={used:?}").into()),
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("need=Full-without-debit;observed=food-cap-probe-deadline-25s".into());
        }
    }
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let meat = provisioning::item_reference(&provisioning::item_index()?, 3577)?;
    character_prep::declare_seeded(
        "C09",
        "character,melee-rat",
        "dedicated Platform bootstrap and nearby native melee without a condition effect",
    );
    let identity =
        character_prep::lookup_row_character(clients, "01934f10-7c00-7000-8000-000000000020")
            .await?;
    let clients = &identity.clients(clients);
    let mut client = character_prep::connect_row_character(clients).await?;
    let frodo = metadata
        .npcs
        .npcs
        .iter()
        .find(|npc| npc.key == "oteryn:npc.frodo")
        .ok_or("Frodo absent from compiled World bundle")?;
    let here = tile(client.world_spatial().actor_position)?;
    let tavern_open = walk_to_frodo(&mut client, world, here, |at| {
        frodo
            .placements
            .iter()
            .any(|p| {
                p.floor == at.floor
                    && u32::from(at.x.abs_diff(p.x).max(at.y.abs_diff(p.y))) <= FRODO_INTERIOR_RANGE
            })
    })
    .await?;
    // Seven meats can fill the 20-minute stomach. Keep an additional unit
    // available to exercise the refusal after allowing the regeneration ticks.
    const MEATS: u32 = 8;
    // The pinned Frodo trade charges five gold per meat.
    super::full_row_c02::earn_gold(
        &mut client,
        &tavern_open,
        &metadata.spawns,
        5 * u64::from(MEATS),
    )
    .await
    .map_err(|error| format!("fund-Frodo-trade: {error}"))?;
    let initial = quantity(&client, meat)?;
    buy_meat_from_frodo(&mut client, &tavern_open, meat, MEATS, true).await?;
    if quantity(&client, meat)? != initial + u64::from(MEATS) {
        return Err(format!(
            "purchase delta expected={MEATS} observed={}",
            quantity(&client, meat)?.saturating_sub(initial)
        )
        .into());
    }
    row.pass("food-purchase", (initial, initial + u64::from(MEATS)));
    row.pass(
        "damage-and-mana-debit",
        damage_and_spend_mana(&mut client, &tavern_open).await?,
    );
    let handle = client
        .inventory()
        .and_then(|i| {
            i.entries
                .iter()
                .find(|item| item.item_definition_ref == meat)
        })
        .map(|item| item.handle)
        .ok_or("bought meat handle absent")?;
    let before_use = quantity(&client, meat)?;
    client.take_events();
    let used = client.use_item(handle).await?;
    if used.disposition != UseDisposition::Committed {
        return Err(format!("food USE reply={used:?}").into());
    }
    wait_quantity(&mut client, meat, before_use - 1).await?;
    row.pass(
        "food-stack-debit",
        (used.disposition, before_use, before_use - 1),
    );
    let refused_inventory = quantity(&client, meat)?;
    let refused_vitals = client.actor_vitals().copied();
    let refused = client.use_item(handle).await?;
    food_remaining(&mut client, Some(refused.result_server_sequence)).await?;
    if refused.status != CommandStatus::Accepted
        || refused.disposition != UseDisposition::Exhausted
        || quantity(&client, meat)? != refused_inventory
        || client.actor_vitals().copied() != refused_vitals
    {
        return Err(format!("food refusal reply={refused:?} inventory={refused_inventory}->{} vitals={refused_vitals:?}->{:?}", quantity(&client, meat)?, client.actor_vitals()).into());
    }
    row.pass("food-refusal", refused.disposition);
    let fed = food_remaining(&mut client, None).await?;
    if fed <= 0 {
        return Err(
            format!("committed meat did not start served food regeneration: {fed}ms").into(),
        );
    }
    let start = *client.actor_vitals().ok_or("fed vitals absent")?;
    let regenerated = wait_regeneration(&mut client, start.health, start.mana, true).await?;
    row.pass(
        "fed-hp-mana-regeneration",
        (start.health, regenerated.0, start.mana, regenerated.1),
    );
    let durable_quantity = quantity(&client, meat)?;
    let old_session = client.join_snapshot().game_session_id;
    accepted_logout(&mut client).await?;
    drop(client);
    let mut client = character_prep::connect_row_character(clients).await?;
    let fed_after = food_remaining(&mut client, None).await?;
    if fed_after <= 0 {
        return Err(format!("fresh admission lost timed food: remaining={fed_after}ms").into());
    }
    spend_mana(&mut client).await?;
    let admitted = *client
        .actor_vitals()
        .ok_or("fresh-admission vitals absent")?;
    if client.join_snapshot().game_session_id == old_session
        || quantity(&client, meat)? != durable_quantity
    {
        return Err(format!("fresh admission session/stack mismatch old={old_session:?} new={:?} meat={durable_quantity}->{}", client.join_snapshot().game_session_id, quantity(&client, meat)?).into());
    }
    let after = wait_regeneration(&mut client, admitted.health, admitted.mana, false).await?;
    row.pass(
        "fresh-admission-regeneration",
        (
            durable_quantity,
            fed_after,
            admitted.health,
            after.0,
            admitted.mana,
            after.1,
        ),
    );
    let unspent_meat = prove_full_stomach(&mut client, meat).await?;
    accepted_logout(&mut client).await?;
    row.pass("full-stomach-refusal", unspent_meat);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pass_requires_each_step_once() {
        let mut row = Row::new();
        for step in STEPS {
            row.pass(step, ());
        }
        assert!(!row.invalid && row.passed.len() == STEPS.len());
        row.pass(STEPS[0], ());
        assert!(row.invalid);
    }
}
