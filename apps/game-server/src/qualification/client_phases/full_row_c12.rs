//! C12 full-row journey (Spell equipment augments), derived from the matrix, the re-audit and the
//! `item_spell_augments` owner. Every step is driven over the real client connection and every
//! effect is measured, never inferred from a mana debit or a slot.
//!
//! 1. `baseline-cast`: reach a reachable creature spawn whose content health is high enough that a
//!    spell hit is never health-capped, attack it, and CAST Greater Tiger Clash repeatedly without
//!    the item; require `Cast` and a mana debit each time and record the spell damage the server
//!    attributes to the cast (the presentation events sharing the cast's decision ordinal).
//! 2. `acquire-50271`: acquire Fists of Enlightenment (50271) in-journey through the NPC trade
//!    route the content declares (earned gold, walk, `hi`, `trade`, BUY); require the inventory
//!    delta. No route in content or no reachable seller fails with `needed=acquisition-route-for-50271`.
//! 3. `augmented-cast`: ITEM_MOVE the item to the right hand, CAST again and require the measured
//!    mean spell damage to exceed the baseline mean (the content augment is base damage).
//! 4. `unequip`: ITEM_MOVE it back to the backpack; require the inventory delta.
//! 5. `unequipped-cast`: CAST again with the item only carried; require the measured mean to fall
//!    below the augmented mean (a carried item grants no augment).
//! 6. `fresh-admission`: re-equip, LOGOUT, admit afresh, require the right-hand item to persist and
//!    CAST again; the pooled augmented mean must exceed the baseline mean.

use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind, SessionEvent};
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget};
use oteryn_protocol_oteryn::item_view as items;
use oteryn_protocol_oteryn::npc_service as npc;
use oteryn_protocol_oteryn::presentation::{PresentationEventKind, PresentationValueKind};
use std::num::NonZeroU32;

const ITEM_ID: u32 = 50_271;
const SPELL_WORDS: &str = "exori nia";
const SPELL_NAME: &str = "greater tiger clash";
const SAMPLES: usize = 3;
const STEPS: [&str; 6] = [
    "baseline-cast",
    "acquire-50271",
    "augmented-cast",
    "unequip",
    "unequipped-cast",
    "fresh-admission",
];

struct Row {
    started: std::time::Instant,
    passed: Vec<&'static str>,
    failure: Option<(&'static str, String)>,
}

impl Row {
    fn new() -> Self {
        Self { started: std::time::Instant::now(), passed: Vec::new(), failure: None }
    }

    fn require_attributed_augment(&mut self, samples: &[u64]) {
        // ValueText identifies a cast, but contains no augment contribution or pre-roll
        // magnitude. Random ordinary rolls (and the weapon's own attack value) can give
        // these same means. Preserve that observation as a gap, never a passed effect.
        self.failure.get_or_insert_with(|| ("augmented-cast", format!("needed=augment-attributed-damage-or-authoritative-unaugmented-bound;observed={samples:?};source=crates/protocol-oteryn/src/presentation.rs")));
    }

    fn current(&self) -> &'static str {
        STEPS.get(self.passed.len()).copied().unwrap_or("journey")
    }

    fn pass(&mut self, step: &'static str, observed: impl std::fmt::Debug) {
        if self.failure.is_none() && STEPS.get(self.passed.len()) == Some(&step) {
            self.passed.push(step);
            evidence(&format!("C12_CHECK {step} PASS observed={observed:?}"));
        }
    }

    fn finish(mut self, result: TestResult) {
        if let Err(error) = result {
            self.failure = Some((self.current(), error.to_string()));
        }
        let complete = self.failure.is_none() && self.passed.len() == STEPS.len();
        let detail = match &self.failure {
            _ if complete => String::new(),
            Some((step, observed)) => {
                let not_run: Vec<_> = STEPS.iter().skip(self.passed.len() + 1).copied().collect();
                evidence(&format!("C12_CHECK {step} FAIL observed={observed}"));
                format!(" evidence=step:{step};observed:{};not-run:{}", token(observed), not_run.join(","))
            }
            None => " evidence=journey-ended-without-every-step".to_owned(),
        };
        evidence(&format!(
            "e2e_all scenario=C12.full status={} seconds={:.3}{detail}",
            if complete { "PASS" } else { "FAIL" },
            self.started.elapsed().as_secs_f64()
        ));
    }
}

fn token(value: &str) -> String {
    value
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || "-_.:/<>=".contains(c) { c } else { '-' })
        .collect()
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result = match row_complete_with_timeout(Duration::from_secs(480), journey(&mut row, clients)).await {
        Ok(result) => result,
        Err(_) => Err("journey-timeout-480s-without-server-reply".into()),
    };
    row.finish(result);
}

fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_json(relative: &str) -> TestResult<serde_json::Value> {
    Ok(serde_json::from_slice(&std::fs::read(root().join(relative))?)?)
}

fn spell() -> TestResult<NonZeroU32> {
    let input = crate::content::native_gameplay::NativeGameplayInput::from_manifest(
        &root().join("content/spells.manifest.json"),
    )?;
    let catalog = crate::spell::executable_catalog::compile(&input.catalog.bytes, &input.catalog.sha256)?;
    let selections = crate::spell::executable_catalog::compile_source_selection(
        &input.source_selection.bytes,
        &input.source_selection.sha256,
        &catalog,
    )?;
    let book = catalog.into_spell_book_with_selections(&selections)?;
    let key = &book.spoken(SPELL_WORDS).ok_or("Greater Tiger Clash absent from the served spell book")?.spell.key;
    (1..=book.source_len())
        .filter_map(|index| u32::try_from(index).ok().and_then(NonZeroU32::new))
        .find(|index| book.indexed(*index).is_some_and(|candidate| &candidate.key == key))
        .ok_or_else(|| "Greater Tiger Clash has no served index".into())
}

/// The base-damage augment (hundredths of a percent) that content binds to this spell on the item.
fn content_augment() -> TestResult<u64> {
    let profiles = read_json("content/items/definitions/spell-native-profiles.json")?;
    let key = format!("oteryn:item.tibia.i{ITEM_ID}");
    profiles["records"]
        .as_array()
        .ok_or("item profile records absent")?
        .iter()
        .find(|record| record["production_definition"]["production_key"] == key)
        .and_then(|record| record["attributes"]["spell_augments"].as_array())
        .and_then(|augments| {
            augments.iter().find(|a| a["kind"] == "base_damage" && a["spell"] == SPELL_NAME)
        })
        .and_then(|augment| augment["value"].as_u64())
        .ok_or_else(|| format!("content binds no base_damage augment of {SPELL_NAME} to {key}").into())
}

/// Creature key to (health, armor-free) content health, for spawn selection.
fn creature_health() -> TestResult<std::collections::BTreeMap<String, u64>> {
    let mut health = std::collections::BTreeMap::new();
    for entry in std::fs::read_dir(root().join("content/creatures/definitions"))? {
        let path = entry?.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        if !(name.starts_with("creatures-") && name.ends_with(".json")) {
            continue;
        }
        let document: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
        for record in document["records"].as_array().into_iter().flatten() {
            if let (Some(key), Some(value)) = (
                record["definition"]["identity"]["key"].as_str(),
                record["authoring"]["profile"]["health"].as_u64(),
            ) {
                health.insert(key.to_owned(), value);
            }
        }
    }
    Ok(health)
}

fn item_ref() -> TestResult<NonZeroU32> {
    provisioning::item_reference(&provisioning::item_index()?, ITEM_ID)
}

fn equipped_right(client: &DevClientSession, definition: NonZeroU32) -> bool {
    client.inventory().is_some_and(|inventory| {
        inventory.equipment.iter().any(|entry| {
            entry.slot == items::EquipmentSlot::RightHand && entry.item.item_definition_ref == definition
        })
    })
}

fn carried_count(client: &DevClientSession, definition: NonZeroU32) -> u64 {
    client.inventory().map_or(0, |inventory| {
        inventory
            .entries
            .iter()
            .filter(|entry| entry.item_definition_ref == definition)
            .map(|entry| u64::from(entry.count.get()))
            .sum()
    })
}

async fn wait_inventory(
    client: &mut DevClientSession,
    what: &str,
    predicate: impl Fn(&DevClientSession) -> bool,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while !predicate(client) {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("no inventory delta for {what}").into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
    Ok(())
}

fn own_entity(client: &DevClientSession) -> TestResult<oteryn_dev_client::EntityRef> {
    let entities = client.world_entities().ok_or("own-entity-snapshot-missing")?;
    entities
        .iter()
        .find(|entry| entry.entity.identity == *entities.own_identity())
        .map(|entry| entry.entity)
        .ok_or_else(|| "own-entity-missing".into())
}

fn adjacent_creature(client: &DevClientSession) -> Option<oteryn_dev_client::EntityRef> {
    let at = client.world_spatial().actor_position;
    client.world_entities()?.iter().find(|entry| {
        entry.kind == EntityKind::Creature
            && entry.position.floor == at.floor
            && entry.position.x.abs_diff(at.x).max(entry.position.y.abs_diff(at.y)) <= 1
    }).map(|entry| entry.entity)
}

/// The spawn creature whose content health is the lowest one that a spell hit cannot cap, else the
/// healthiest reachable one; the measurement gate below still refuses any possibly capped sample.
fn pick_creature(
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    from: TilePos,
    health: &std::collections::BTreeMap<String, u64>,
) -> TestResult<(String, u64)> {
    let mut found: Vec<(String, u64)> = Vec::new();
    for creature in &spawns.creatures {
        let Some(hp) = health.get(&creature.key).copied() else { continue };
        let points: Vec<_> = spawns
            .sources
            .iter()
            .flat_map(|source| source.points.iter().map(move |point| (source.floor, point)))
            .filter(|(_, point)| spawns.creatures.get(point.creature as usize).is_some_and(|c| c.key == creature.key))
            .collect();
        if route(world, from, |at| {
            !world.base().tile(at.x, at.y, at.floor).is_none_or(|t| t.protection_zone())
                && points.iter().any(|(floor, p)| at.floor == *floor && at.x.abs_diff(p.x) <= 1 && at.y.abs_diff(p.y) <= 1)
        }).is_some() {
            found.push((creature.key.clone(), hp));
        }
    }
    found.sort_by_key(|(_, hp)| *hp);
    found
        .iter()
        .find(|(_, hp)| *hp >= 3_000)
        .or_else(|| found.last())
        .cloned()
        .ok_or_else(|| "no reachable creature spawn has content health".into())
}

struct Hunt<'a> {
    world: &'a BundleWorld,
    spawns: &'a oteryn_world_bundle::spawn::Table,
    creature: String,
    health: u64,
}

impl Hunt<'_> {
    async fn target(&self, client: &mut DevClientSession) -> TestResult<oteryn_dev_client::EntityRef> {
        for _ in 0..4 {
            if adjacent_creature(client).is_none() {
                travel_to_spawn(client, self.world, self.spawns, &self.creature).await?;
            }
            let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
            while adjacent_creature(client).is_none() && tokio::time::Instant::now() < deadline {
                client.service_liveness(Duration::from_millis(100)).await?;
                client.take_events();
            }
            if let Some(entity) = adjacent_creature(client) {
                let reply = client.session_mut().attack_target(Some(&entity)).await?;
                if reply.disposition == oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
                    return Ok(entity);
                }
                return Err(format!("ATTACK reply={reply:?}").into());
            }
        }
        Err(format!("no live {} adjacent after four spawn visits", self.creature).into())
    }

    /// One cast, retried only while the server reports `CoolingDown`; returns the damage the server
    /// attributes to that cast (events sharing the decision ordinal of the caster's SpellWords).
    async fn cast_damage(&self, client: &mut DevClientSession, spell: NonZeroU32) -> TestResult<u64> {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        loop {
            let victim = self.target(client).await?;
            let own = own_entity(client)?;
            let before = *client.actor_vitals().ok_or("pre-cast vitals absent")?;
            client.take_events();
            let outcome = client.cast_spell(spell, SpellTarget::AttackTarget, true).await?;
            if outcome.disposition == SpellCastDisposition::CoolingDown && tokio::time::Instant::now() < deadline {
                client.service_liveness(Duration::from_millis(250)).await?;
                continue;
            }
            if outcome.disposition != SpellCastDisposition::Cast || outcome.actor_vitals_delta.is_none() {
                return Err(format!("CAST reply={outcome:?}").into());
            }
            let mut damage = 0_u64;
            let window = tokio::time::Instant::now() + Duration::from_secs(3);
            while damage == 0 && tokio::time::Instant::now() < window {
                client.service_liveness(Duration::from_millis(50)).await?;
                damage = client.take_events().iter().map(|event| cast_damage_in(event, own, victim)).sum();
            }
            let after = *client.actor_vitals().ok_or("post-cast vitals absent")?;
            if damage == 0 {
                return Err(format!("CAST reply={outcome:?} produced no attributed spell damage frame; mana={}->{}", before.mana, after.mana).into());
            }
            if damage.saturating_mul(2) >= self.health {
                return Err(format!("damage={damage} may be capped by {} health={}; needed=creature-with-spell-proof-health", self.creature, self.health).into());
            }
            return Ok(damage);
        }
    }
}

/// Damage to `victim` in presentation batches that also carry `own`'s SpellWords in the same decision.
fn cast_damage_in(event: &SessionEvent, own: oteryn_dev_client::EntityRef, victim: oteryn_dev_client::EntityRef) -> u64 {
    let SessionEvent::Presentation(delta) = event else { return 0 };
    let decisions: Vec<u32> = delta
        .value
        .iter()
        .filter(|e| matches!(e.kind, PresentationEventKind::SpellWords { caster, .. } if caster == own))
        .map(|e| e.decision_ordinal)
        .collect();
    delta
        .value
        .iter()
        .filter(|e| decisions.contains(&e.decision_ordinal))
        .filter_map(|e| match e.kind {
            PresentationEventKind::ValueText { kind: PresentationValueKind::DamageDealt, target, primary, .. }
                if target == victim => Some(primary.value),
            _ => None,
        })
        .sum()
}

fn mean(samples: &[u64]) -> f64 {
    samples.iter().sum::<u64>() as f64 / samples.len().max(1) as f64
}

async fn samples(hunt: &Hunt<'_>, client: &mut DevClientSession, spell: NonZeroU32) -> TestResult<Vec<u64>> {
    let mut out = Vec::new();
    for _ in 0..SAMPLES {
        let before = client.actor_vitals().ok_or("pre-sample vitals absent")?.mana;
        out.push(hunt.cast_damage(client, spell).await?);
        if client.actor_vitals().ok_or("post-sample vitals absent")?.mana >= before {
            return Err(format!("Cast without a mana debit: mana {before} -> {}", client.actor_vitals().map_or(0, |v| v.mana)).into());
        }
    }
    Ok(out)
}

async fn player_gold(client: &mut DevClientSession) -> TestResult<u64> {
    use oteryn_protocol_oteryn::atelier::resources as money;
    let reply = client
        .session_mut()
        .gated_command(money::CAPABILITY_ATELIER_RESOURCES_V1, money::COMMAND_TYPE_ATELIER_RESOURCES_QUERY, &[])
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("native player balances refused:{reply:?}").into());
    }
    let balances = money::decode_reply(&reply.payload).map_err(|e| format!("native balances:{e:?}"))?;
    balances.carried_gold.checked_add(balances.bank_gold).ok_or_else(|| "native gold overflow".into())
}

/// Acquire item 50271 through the declared NPC trade (earning the gold in play) and return nothing
/// but the confirmed inventory delta.
async fn acquire(
    client: &mut DevClientSession,
    world: &BundleWorld,
    metadata: &oteryn_world_bundle::bundle::Visited,
    definition: NonZeroU32,
) -> TestResult<u64> {
    use oteryn_dev_client::client_runtime::npc_shop;
    let trades = read_json("content/services/trade/trade-00000-00323.json")?;
    let key = format!("oteryn:item.tibia.i{ITEM_ID}");
    let mut offers: Vec<(String, u64)> = Vec::new();
    for record in trades["records"].as_array().ok_or("trade records absent")? {
        let declaration = &record["declaration"];
        let Some(service) = declaration["identity"]["key"].as_str().and_then(|k| k.strip_prefix("oteryn:service.trade.")) else { continue };
        for offer in declaration["offers"].as_array().into_iter().flatten() {
            if let (true, Some(price)) = (
                offer["direction"] == "SellToPlayer" && offer["item"]["key"] == key,
                offer["unit_price"].as_u64(),
            ) {
                offers.push((format!("oteryn:npc.{service}"), price));
            }
        }
    }
    let here = tile(client.world_spatial().actor_position)?;
    let reachable = |npc_key: &str, from: TilePos| {
        metadata.npcs.npcs.iter().filter(|n| n.key == npc_key).find_map(|n| {
            route(world, from, |at| n.placements.iter().any(|p| p.floor == at.floor && at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)) <= 3)).map(|path| (n, path))
        })
    };
    let (seller, price) = offers
        .iter()
        .filter(|(npc_key, _)| reachable(npc_key, here).is_some())
        .min_by_key(|(_, price)| *price)
        .cloned()
        .ok_or_else(|| format!("needed=acquisition-route-for-{ITEM_ID};content-sellers={};reachable-sellers=0;loot-and-quest-routes=none-declared", offers.len()))?;
    let hunt_deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    let mut hunts = 0_u32;
    loop {
        let have = player_gold(client).await?;
        if have >= price {
            break;
        }
        if tokio::time::Instant::now() >= hunt_deadline {
            return Err(format!("needed=player-earned-gold>={price};have={have};route=npc-trade:{seller};ordinary-hunts={hunts}-in-120s").into());
        }
        travel_to_spawn(client, world, &metadata.spawns, "oteryn:creature.rat").await?;
        client_hunt::hunt(client).await?;
        hunts += 1;
    }
    let (npc_entry, path) = reachable(&seller, tile(client.world_spatial().actor_position)?)
        .ok_or("seller unreachable after the gold hunt")?;
    walk(client, world, &path).await?;
    let actor = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|e| {
                e.kind == EntityKind::Npc
                    && npc_entry.placements.iter().any(|p| e.position.x == i32::from(p.x) && e.position.y == i32::from(p.y) && e.position.floor == -i16::from(p.floor))
            })
        })
        .map(|e| e.entity)
        .ok_or("seller not visible as an NPC entity")?;
    for text in ["hi", "trade"] {
        let reply = npc_shop::talk(client.session_mut(), &npc::NpcTalkIntent { npc_actor: actor, text: text.into() }).await?;
        if reply != npc::NpcIntentDisposition::Ok {
            return Err(format!("NPC {text} reply={reply:?}").into());
        }
    }
    let delta = client.session_mut().read_gated_delta(npc::STATE_DOMAIN_NPC_TRADE_WINDOW, 1).await?;
    let window = npc::decode_npc_trade_window(&delta.payload).map_err(|e| format!("trade window:{e:?}"))?;
    let (index, offer) = window
        .offers
        .iter()
        .enumerate()
        .find(|(_, o)| o.side == npc::NpcTradeSide::Buy && o.item_definition_ref == definition.get())
        .ok_or("served trade window does not offer the item")?;
    let before = carried_count(client, definition);
    let reply = npc_shop::buy(
        client.session_mut(),
        &npc::NpcTradeIntent {
            npc_actor: actor,
            catalogue_revision: window.open.ok_or("trade window closed")?.1,
            offer_index: u32::try_from(index)?,
            side: offer.side,
            quantity: 1,
            expected_unit_price: offer.unit_price,
        },
    )
    .await?;
    if reply != npc::NpcIntentDisposition::Ok {
        return Err(format!("BUY reply={reply:?} price={}", offer.unit_price).into());
    }
    wait_inventory(client, "purchased item", |c| carried_count(c, definition) > before).await?;
    Ok(client.last_server_sequence())
}

async fn equip_right(client: &mut DevClientSession, definition: NonZeroU32) -> TestResult {
    let handle = client
        .inventory()
        .and_then(|inventory| inventory.entries.iter().find(|e| e.item_definition_ref == definition))
        .map(|e| e.handle)
        .ok_or("augment item not carried")?;
    let moved = client
        .move_item(&items::ItemMoveIntent { source: handle, destination: items::ItemMoveDestination::Equipment(items::EquipmentSlot::RightHand) })
        .await?;
    if moved.outcome != items::ItemMoveOutcome::Moved {
        return Err(format!("right-hand ITEM_MOVE reply={moved:?}").into());
    }
    wait_inventory(client, "right-hand equipment", |c| equipped_right(c, definition)).await
}

async fn logout(client: &mut RowClient) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(75);
    loop {
        let reply = client.row_logout().await?;
        if reply.result == Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("logout reply={:?}", reply.result).into());
        }
        client.service_liveness(Duration::from_millis(250)).await?;
    }
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let definition = item_ref()?;
    let spell = spell()?;
    let augment = content_augment()?;
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(bytes, &pin, WorldId::decode(&clients.world)?, ChannelId::decode(&clients.channel)?)?;
    let mut client = row_connect_world(clients, 0xc2).await?;
    let from = tile(client.world_spatial().actor_position)?;
    let (creature, health) = pick_creature(world, &metadata.spawns, from, &creature_health()?)?;
    let hunt = Hunt { world, spawns: &metadata.spawns, creature, health };

    let baseline = samples(&hunt, &mut client, spell).await?;
    row.pass("baseline-cast", (&baseline, mean(&baseline)));

    let acquired = acquire(&mut client, world, &metadata, definition).await?;
    row.pass("acquire-50271", acquired);

    equip_right(&mut client, definition).await?;
    let augmented = samples(&hunt, &mut client, spell).await?;
    if !equipped_right(&client, definition) || mean(&augmented) <= mean(&baseline) {
        return Err(format!(
            "no augment effect: baseline={baseline:?} (mean {:.1}) augmented={augmented:?} (mean {:.1}) content-augment={augment}/10000",
            mean(&baseline), mean(&augmented)
        ).into());
    }
    row.require_attributed_augment(&augmented);
    row.pass("augmented-cast", (&augmented, mean(&augmented) / mean(&baseline), augment));

    let equipped = client
        .inventory()
        .and_then(|i| i.equipment.iter().find(|e| e.slot == items::EquipmentSlot::RightHand && e.item.item_definition_ref == definition))
        .map(|e| e.item.handle)
        .ok_or("right-hand item absent before unequip")?;
    let moved = client
        .move_item(&items::ItemMoveIntent { source: equipped, destination: items::ItemMoveDestination::MainBackpack })
        .await?;
    if moved.outcome != items::ItemMoveOutcome::Moved {
        return Err(format!("unequip ITEM_MOVE reply={moved:?}").into());
    }
    wait_inventory(&mut client, "unequip", |c| !equipped_right(c, definition) && carried_count(c, definition) > 0).await?;
    row.pass("unequip", client.last_server_sequence());

    let carried = samples(&hunt, &mut client, spell).await?;
    if mean(&carried) >= mean(&augmented) {
        return Err(format!("carried item kept the augment: augmented={augmented:?} carried={carried:?}").into());
    }
    row.pass("unequipped-cast", (&carried, mean(&carried)));

    equip_right(&mut client, definition).await?;
    let old_session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);
    let mut fresh = row_connect_world(clients, 0xc3).await?;
    if fresh.join_snapshot().game_session_id == old_session || !equipped_right(&fresh, definition) {
        return Err("fresh admission lost the right-hand item".into());
    }
    let replay = samples(&hunt, &mut fresh, spell).await?;
    let pooled: Vec<u64> = augmented.iter().chain(&replay).copied().collect();
    if mean(&pooled) <= mean(&baseline) {
        return Err(format!("augment not reconstructed: baseline={baseline:?} pooled-augmented={pooled:?}").into());
    }
    row.pass("fresh-admission", (&replay, mean(&pooled) / mean(&baseline)));
    logout(&mut fresh).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_high_damage_rolls_do_not_prove_an_augment() {
        let mut row = Row::new();
        row.pass("baseline-cast", [90, 91, 92]);
        row.pass("acquire-50271", "real-inventory-delta");
        row.require_attributed_augment(&[108, 109, 110]);
        row.pass("augmented-cast", [108, 109, 110]);
        assert_eq!(row.passed.len(), 2);
        assert!(row.failure.is_some());
    }

    #[test]
    fn pass_requires_every_step_in_order() {
        let mut row = Row::new();
        row.pass("acquire-50271", "out of order");
        assert!(row.passed.is_empty());
        for step in STEPS {
            row.pass(step, "ok");
        }
        assert_eq!(row.passed.len(), STEPS.len());
    }

    #[test]
    fn content_binds_the_augment_to_the_item() -> TestResult {
        assert!(content_augment()? > 0);
        assert_ne!(item_ref()?.get(), 0);
        Ok(())
    }

    #[test]
    fn the_declared_trade_sells_the_item() -> TestResult {
        let trades = read_json("content/services/trade/trade-00000-00323.json")?;
        let key = format!("oteryn:item.tibia.i{ITEM_ID}");
        assert!(trades["records"].as_array().ok_or("records")?.iter().any(|record| {
            record["declaration"]["offers"].as_array().is_some_and(|offers| {
                offers.iter().any(|o| o["direction"] == "SellToPlayer" && o["item"]["key"] == key)
            })
        }));
        Ok(())
    }
}
