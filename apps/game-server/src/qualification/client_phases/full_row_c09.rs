//! C09 full-row player scenario, derived from the matrix, Crystal re-audit, and the native
//! food item-use and periodic-vitals owners:
//! 1. Buy meat from the authored seller and verify the exact purchased stack increase.
//! 2. Walk to a compiled poison-spider spawn and use ordinary ATTACK/combat until health is
//!    below maximum; cast an available mana-cost spell and require its Cast reply and mana debit.
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
        let result: TestResult =
            result.and(Err(super::full_row_c02::PLATFORM_ADMISSION_GAP.into()));
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
const FRODO_WANDER_RADIUS: u16 = 2;

fn is_frodo(
    entity: &oteryn_dev_client::WorldSpatialEntity,
    placements: &[oteryn_world_bundle::npc::Placement],
) -> bool {
    entity.kind == oteryn_dev_client::EntityKind::Npc
        && entity.display_name.is_none_or(|name| name.as_str().is_ok_and(|name| name == "Frodo"))
        && placements.iter().any(|p| {
            entity.position.floor == -i16::from(p.floor)
                && entity.position.x.abs_diff(i32::from(p.x))
                    .max(entity.position.y.abs_diff(i32::from(p.y))) <= u32::from(FRODO_WANDER_RADIUS)
        })
}

fn meat_seller_at(
    entity: &oteryn_dev_client::WorldSpatialEntity,
    placements: &[oteryn_world_bundle::npc::Placement],
    player: ActorPosition,
) -> bool {
    is_frodo(entity, placements)
        && entity.position.floor == player.floor
        && entity.position.x.abs_diff(player.x).max(entity.position.y.abs_diff(player.y))
            <= oteryn_protocol_oteryn::npc_service::TALK_RANGE_TILES
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

pub(super) async fn buy_meat(
    client: &mut DevClientSession,
    world: &BundleWorld,
    reference: NonZeroU32,
    units: u32,
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
    let closest = wire::TALK_RANGE_TILES.saturating_sub(u32::from(FRODO_WANDER_RADIUS));
    let farthest = wire::TALK_RANGE_TILES + u32::from(FRODO_WANDER_RADIUS);
    let path = (closest..=farthest).find_map(|radius| {
        route(world, here, |at| {
            npc.placements.iter().any(|p| {
                p.floor == at.floor
                    && u32::from(at.x.abs_diff(p.x).max(at.y.abs_diff(p.y))) <= radius
            })
        })
    })
    .ok_or("no walk route to Frodo within talk range")?;
    walk(client, world, &path).await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let actor = loop {
        if let Some(actor) = visible_frodo(client, &npc.placements) {
            break actor;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "need=unique-visible-Frodo-within-talk-range;observed=none-or-ambiguous;position={:?};last-frame={}",
                client.world_spatial().actor_position, client.last_server_sequence()
            ).into());
        }
        let frodo = client.world_entities().and_then(|view| {
            let mut matches = view.iter().filter(|entry| is_frodo(entry, &npc.placements));
            let only = matches.next()?;
            matches.next().is_none().then_some(only.position)
        });
        if let Some(position) = frodo {
            let target = tile(position)?;
            let from = tile(client.world_spatial().actor_position)?;
            let approach = route_avoiding(world, from, |at| {
                at.floor == target.floor
                    && u32::from(at.x.abs_diff(target.x).max(at.y.abs_diff(target.y)))
                        <= wire::TALK_RANGE_TILES
            }, |at| at != target);
            if let Some(direction) = approach.and_then(|steps| steps.first().copied()) {
                walk(client, world, &[direction]).await?;
            }
        }
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
    };
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
    let details = client.selected_capabilities().contains(
        &oteryn_protocol_oteryn::trade_details::CAPABILITY_TRADE_DETAILS_V1,
    );
    let window = wire::decode_npc_trade_window_for(&delta.payload, details)
        .map_err(|e| format!("trade window reply={e:?}"))?;
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
    let reply = runtime::buy(
        client.session_mut(),
        &wire::NpcTradeIntent {
            npc_actor: actor,
            catalogue_revision: window.open.ok_or("trade window closed")?.1,
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
    .await
}

async fn damage_and_spend_mana(
    client: &mut DevClientSession,
    world: &BundleWorld,
) -> TestResult<(u32, u32, u32, u32)> {
    let metadata = oteryn_world_bundle::bundle::visit(
        &std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?,
        |_, _| Ok(()),
    )?;
    travel_to_spawn(
        client,
        world,
        &metadata.spawns,
        "oteryn:creature.poison_spider",
    )
    .await?;
    client.session_mut().attack_target(None).await?;
    let (mana, mana_after) = spend_mana(client).await?;
    let before = *client.actor_vitals().ok_or("pre-combat vitals absent")?;
    client.take_events();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    let health = loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        let changed = client
            .take_events()
            .into_iter()
            .find_map(|event| match event {
                SessionEvent::ActorVitals(delta) if delta.value.health < before.health => {
                    Some(delta.value.health)
                }
                _ => None,
            });
        if let Some(value) = changed {
            break value;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("ATTACK produced no damage delta from {}", before.health).into());
        }
    };
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
    let mut client = connect_world(clients, 0xc9).await?;
    // Seven meats can fill the 20-minute stomach. Keep an additional unit
    // available to exercise the refusal after allowing the regeneration ticks.
    const MEATS: u32 = 8;
    // The pinned Frodo trade charges five gold per meat.
    super::full_row_c02::earn_gold(&mut client, world, &metadata.spawns, 5 * u64::from(MEATS))
        .await
        .map_err(|error| format!("fund-Frodo-trade: {error}"))?;
    let initial = quantity(&client, meat)?;
    buy_meat(&mut client, world, meat, MEATS).await?;
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
        damage_and_spend_mana(&mut client, world).await?,
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
    let mut client = connect_world(clients, 0xd9).await?;
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
