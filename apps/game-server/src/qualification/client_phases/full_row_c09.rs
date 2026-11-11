//! C09 full-row player scenario, derived from the matrix, Crystal re-audit, and the native
//! food item-use and periodic-vitals owners:
//! 1. Walk to an authored food seller, send NPC `hi` and `trade`, buy two meats, and require
//!    accepted talk/buy replies plus an exact two-unit inventory increase.
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
    let result = row_complete(journey(&mut row, clients))
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

async fn buy_meat(
    client: &mut DevClientSession,
    world: &BundleWorld,
    reference: NonZeroU32,
) -> TestResult {
    use oteryn_dev_client::client_runtime::npc_shop as runtime;
    use oteryn_protocol_oteryn::npc_service as wire;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let trades: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/services/trade/trade-00000-00323.json"),
    )?)?;
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
                    offer["direction"] == "SellToPlayer"
                        && offer["item"]["key"] == "oteryn:item.tibia.i3577"
                })
                .then(|| declaration["identity"]["key"].as_str())
                .flatten()?
                .strip_prefix("oteryn:service.trade.")
                .map(|name| format!("oteryn:npc.{name}"))
        })
        .collect();
    let metadata = oteryn_world_bundle::bundle::visit(
        &std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?,
        |_, _| Ok(()),
    )?;
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
        .ok_or("no walk route to authored meat seller")?;
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
        .ok_or("meat seller not visible after walk")?;
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
            quantity: 2,
            expected_unit_price: offer.unit_price,
        },
    )
    .await?;
    if reply != wire::NpcIntentDisposition::Ok {
        return Err(format!("buy reply={reply:?}").into());
    }
    wait_quantity(client, reference, before + 2 * u64::from(offer.count)).await
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
    let before = *client.actor_vitals().ok_or("pre-combat vitals absent")?;
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
    client.session_mut().attack_target(None).await?;
    use oteryn_protocol_oteryn::actor_spell_catalog as catalog;
    let snapshot = client
        .session_mut()
        .gated_snapshot(catalog::STATE_DOMAIN_ACTOR_SPELL_CATALOG)
        .ok_or("spell catalogue absent")?;
    let spell = catalog::decode_catalog(&snapshot.payload)
        .map_err(|e| format!("catalogue reply={e:?}"))?
        .into_iter()
        .find(|entry| entry.available && entry.mana > 0 && entry.rune_item.is_none())
        .ok_or("no available mana-cost player spell")?;
    let mana = client.actor_vitals().ok_or("pre-cast vitals absent")?.mana;
    let cast = client
        .cast_spell(
            NonZeroU32::new(spell.cast_index).ok_or("zero cast index")?,
            SpellTarget::None,
            false,
        )
        .await?;
    if cast.disposition != SpellCastDisposition::Cast {
        return Err(format!("spell reply={cast:?}").into());
    }
    let mana_after = client.actor_vitals().ok_or("post-cast vitals absent")?.mana;
    if mana_after >= mana {
        return Err(format!("Cast reply missing mana debit {mana}->{mana_after}").into());
    }
    Ok((before.health, health, mana, mana_after))
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

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(300);
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let meat = provisioning::item_reference(&provisioning::item_index()?, 3577)?;
    let mut client = connect_world(clients, 0xc9).await?;
    let mut row_needs_logout_0 = true;
    let row_result: TestResult<_> = async {
    let initial = quantity(&client, meat)?;
    buy_meat(&mut client, world, meat).await?;
    if quantity(&client, meat)? != initial + 2 {
        return Err(format!(
            "purchase delta expected=2 observed={}",
            quantity(&client, meat)?.saturating_sub(initial)
        )
        .into());
    }
    row.pass("food-purchase", (initial, initial + 2));
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
    if refused.disposition != UseDisposition::Exhausted
        || quantity(&client, meat)? != refused_inventory
        || client.actor_vitals().copied() != refused_vitals
    {
        return Err(format!("food refusal reply={refused:?} inventory={refused_inventory}->{} vitals={refused_vitals:?}->{:?}", quantity(&client, meat)?, client.actor_vitals()).into());
    }
    row.pass("food-refusal", refused.disposition);
    let start = *client.actor_vitals().ok_or("fed vitals absent")?;
    let regenerated = wait_regeneration(&mut client, start.health, start.mana, true).await?;
    row.pass(
        "fed-hp-mana-regeneration",
        (start.health, regenerated.0, start.mana, regenerated.1),
    );
    let durable_quantity = quantity(&client, meat)?;
    let old_session = client.join_snapshot().game_session_id;
    accepted_logout(&mut client).await?; row_needs_logout_0 = false;
    if row_needs_logout_0 { row_logout(&mut client).await?; row_needs_logout_0 = false; }
    let mut client = connect_world(clients, 0xd9).await?;
    let mut row_needs_logout_1 = true;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
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
            admitted.health,
            after.0,
            admitted.mana,
            after.1,
        ),
    );
    accepted_logout(&mut client).await?; row_needs_logout_1 = false;
    Ok(())

    }).await.unwrap_or_else(|error| Err(error.into()));
    if row_needs_logout_1 { row_logout(&mut client).await?; }
    row_result

    }.await;
    if row_needs_logout_0 {
        row_logout(&mut client).await?;
    }
    row_result
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
