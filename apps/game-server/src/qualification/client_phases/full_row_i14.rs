//! I14 player scenario, derived from the matrix, the 2026-10-28 Crystal re-audit, and the
//! production item-use owner (`durability/item_use.rs`):
//! 1. Walk to an authored NPC seller, say `hi` and `trade`, buy two health potions (266) and
//!    two meats (3577), and assert the accepted talk/buy replies plus exact inventory deltas.
//! 2. USE one server-issued meat handle; assert `Committed` and an exact one-unit inventory
//!    debit (the server-owned 180-second food condition is not exposed as a client state frame).
//! 3. USE one server-issued health-potion handle; assert `Committed`, one potion debit, one
//!    empty-flask (285) credit, and a vitals frame whose health does not decrease.
//! 4. Immediately USE the remaining potion as the refusal path; assert `Exhausted` and no
//!    inventory or vitals mutation.
//! 5. Logout, obtain a fresh admission, and assert a new game-session id plus the durable
//!    potion/meat/flask quantities and vitals observed after the committed uses.
//! 6. Through the native hotkey encoder, attempt an uncarried consumable variant (268); assert
//!    `NothingToUse` and no state delta. This exercises the audited missing-profile path rather
//!    than turning a source inspection into a verdict.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use std::collections::{BTreeMap, BTreeSet};

const REQUIRED: &[&str] = &[
    "npc-purchase",
    "food-consume",
    "potion-consume-flask",
    "potion-exhaust-refusal",
    "fresh-admission",
    "missing-variant-refusal",
];

struct Row {
    passed: BTreeSet<&'static str>,
    started: std::time::Instant,
    invalid: bool,
}

impl Row {
    fn new() -> Self {
        Self { passed: BTreeSet::new(), started: std::time::Instant::now(), invalid: false }
    }
    fn pass(&mut self, step: &'static str, observation: impl std::fmt::Display) {
        if !REQUIRED.contains(&step) || !self.passed.insert(step) { self.invalid = true; }
        evidence(&format!("I14_CHECK {step} PASS {observation}"));
    }
    fn finish(self, result: TestResult) {
        if let Err(error) = &result { evidence(&format!("I14_GAP step={} observed={error}", next_step(&self.passed))); }
        for step in REQUIRED { if !self.passed.contains(step) { evidence(&format!("I14_CHECK {step} FAIL not-observed")); } }
        evidence(&format!(
            "e2e_all scenario=I14.full status={} seconds={:.3} evidence={}",
            if result.is_ok() && !self.invalid && self.passed.len() == REQUIRED.len() { "PASS" } else { "FAIL" },
            self.started.elapsed().as_secs_f64(),
            if result.is_ok() { "native-lifecycle-observed" } else { "runtime-step-failed-see-I14_GAP" },
        ));
    }
}

fn next_step(passed: &BTreeSet<&str>) -> &'static str {
    REQUIRED.iter().copied().find(|step| !passed.contains(step)).unwrap_or("completion")
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result = tokio::time::timeout(Duration::from_secs(180), journey(&mut row, clients)).await;
    row.finish(match result { Ok(result) => result, Err(_) => Err("timeout waiting for server reply/delta".into()) });
}

fn quantities(client: &DevClientSession) -> TestResult<BTreeMap<u32, u64>> {
    let mut values = BTreeMap::new();
    for item in &client.inventory().ok_or("inventory snapshot absent")?.entries {
        *values.entry(item.item_definition_ref.get()).or_default() += u64::from(item.count.get());
    }
    Ok(values)
}

fn quantity(client: &DevClientSession, reference: std::num::NonZeroU32) -> TestResult<u64> {
    Ok(*quantities(client)?.get(&reference.get()).unwrap_or(&0))
}

async fn wait_quantity(client: &mut DevClientSession, reference: std::num::NonZeroU32, expected: u64) -> TestResult<u64> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        for event in client.take_events() {
            if let SessionEvent::Inventory(delta) = event && quantity(client, reference)? == expected { return Ok(delta.server_sequence); }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("inventory delta timeout definition={reference} expected={expected} observed={}", quantity(client, reference)?).into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }
}

async fn acquire(client: &mut DevClientSession, world: &BundleWorld, id: u32, amount: u32) -> TestResult<u64> {
    use oteryn_dev_client::client_runtime::npc_shop as runtime;
    use oteryn_protocol_oteryn::npc_service as wire;
    let index = provisioning::item_index()?;
    let reference = provisioning::item_reference(&index, id)?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let trades: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join("content/services/trade/trade-00000-00323.json"))?)?;
    let key = format!("oteryn:item.tibia.i{id}");
    let sellers: BTreeSet<_> = trades["records"].as_array().ok_or("trade records absent")?.iter().filter_map(|record| {
        let declaration = &record["declaration"];
        declaration["offers"].as_array()?.iter().any(|offer| offer["direction"] == "SellToPlayer" && offer["item"]["key"] == key)
            .then(|| declaration["identity"]["key"].as_str()).flatten()?.strip_prefix("oteryn:service.trade.").map(|name| format!("oteryn:npc.{name}"))
    }).collect();
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let here = tile(client.world_spatial().actor_position)?;
    let (npc, path) = metadata.npcs.npcs.iter().filter(|npc| sellers.contains(&npc.key)).filter_map(|npc| {
        route(world, here, |at| npc.placements.iter().any(|p| p.floor == at.floor && at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)) <= 3)).map(|path| (npc, path))
    }).min_by_key(|(_, path)| path.len()).ok_or_else(|| format!("no walk route to seller for item {id}; sellers={sellers:?}"))?;
    walk(client, world, &path).await?;
    let actor = client.world_entities().and_then(|entities| entities.iter().find(|entity| entity.kind == oteryn_dev_client::EntityKind::Npc && npc.placements.iter().any(|p| entity.position.x == i32::from(p.x) && entity.position.y == i32::from(p.y) && entity.position.floor == -i16::from(p.floor)))).map(|entry| entry.entity).ok_or("seller not visible after walk")?;
    for text in ["hi", "trade"] {
        let reply = runtime::talk(client.session_mut(), &wire::NpcTalkIntent { npc_actor: actor, text: text.into() }).await?;
        if reply != wire::NpcIntentDisposition::Ok { return Err(format!("NPC {text} reply={reply:?}").into()); }
    }
    let delta = client.session_mut().read_gated_delta(wire::STATE_DOMAIN_NPC_TRADE_WINDOW, 1).await?;
    let window = wire::decode_npc_trade_window(&delta.payload).map_err(|error| format!("trade window decode: {error:?}"))?;
    let (offer_index, offer) = window.offers.iter().enumerate().find(|(_, offer)| offer.side == wire::NpcTradeSide::Buy && offer.item_definition_ref == reference.get()).ok_or("live seller omitted requested offer")?;
    let before = quantity(client, reference)?;
    client.take_events();
    let reply = runtime::buy(client.session_mut(), &wire::NpcTradeIntent { npc_actor: actor, catalogue_revision: window.open.ok_or("trade window closed")?.1, offer_index: u32::try_from(offer_index)?, side: offer.side, quantity: amount, expected_unit_price: offer.unit_price }).await?;
    if reply != wire::NpcIntentDisposition::Ok { return Err(format!("buy item={id} reply={reply:?} price={} quantity={amount}", offer.unit_price).into()); }
    let expected = before + u64::from(offer.count) * u64::from(amount);
    wait_quantity(client, reference, expected).await
}

async fn use_one(client: &mut DevClientSession, reference: std::num::NonZeroU32) -> TestResult<(UseDisposition, u64)> {
    let before = quantity(client, reference)?;
    let handle = client.inventory().and_then(|inventory| inventory.entries.iter().find(|item| item.item_definition_ref == reference)).map(|item| item.handle).ok_or("purchased item handle absent")?;
    client.take_events();
    let reply = client.use_item(handle).await?;
    if reply.disposition == UseDisposition::Committed { wait_quantity(client, reference, before - 1).await?; }
    Ok((reply.disposition, before))
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) { return Err("logout refused".into()); }
    Ok(())
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(bytes, &pin, WorldId::decode(&clients.world)?, ChannelId::decode(&clients.channel)?)?;
    let index = provisioning::item_index()?;
    let potion = provisioning::item_reference(&index, 266)?;
    let meat = provisioning::item_reference(&index, 3577)?;
    let flask = provisioning::item_reference(&index, 285)?;
    let missing = provisioning::item_reference(&index, 268)?;
    let mut client = connect_world(clients, 0xb4).await?;
    let purchase_meat = acquire(&mut client, world, 3577, 2).await?;
    let purchase_potion = acquire(&mut client, world, 266, 2).await?;
    row.pass("npc-purchase", format!("meat_sequence={purchase_meat} potion_sequence={purchase_potion}"));
    let (food_reply, _) = use_one(&mut client, meat).await?;
    if food_reply != UseDisposition::Committed { return Err(format!("food USE reply={food_reply:?}").into()); }
    row.pass("food-consume", format!("reply={food_reply:?} remaining={}", quantity(&client, meat)?));
    let flask_before = quantity(&client, flask)?;
    let health_before = client.actor_vitals().copied().ok_or("pre-potion vitals absent")?.health;
    let (potion_reply, _) = use_one(&mut client, potion).await?;
    if potion_reply != UseDisposition::Committed || quantity(&client, flask)? != flask_before + 1 || client.actor_vitals().is_none_or(|v| v.health < health_before) { return Err(format!("potion USE reply={potion_reply:?} flask_before={flask_before} flask_after={}", quantity(&client, flask)?).into()); }
    row.pass("potion-consume-flask", format!("reply={potion_reply:?} flask_delta=1"));
    let before_refusal = quantities(&client)?;
    let vitals_before_refusal = client.actor_vitals().copied();
    let (refusal, _) = use_one(&mut client, potion).await?;
    if refusal != UseDisposition::Exhausted || quantities(&client)? != before_refusal || client.actor_vitals().copied() != vitals_before_refusal { return Err(format!("immediate potion refusal reply={refusal:?} state_changed={}", quantities(&client)? != before_refusal).into()); }
    row.pass("potion-exhaust-refusal", format!("reply={refusal:?} no-delta"));
    let durable = quantities(&client)?;
    let vitals = client.actor_vitals().copied();
    let session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    client = connect_world(clients, 0xb4).await?;
    if client.join_snapshot().game_session_id == session || quantities(&client)? != durable || client.actor_vitals().copied() != vitals { return Err("fresh admission changed durable consumable state/vitals".into()); }
    row.pass("fresh-admission", "new-session exact-inventory-and-vitals");
    // `use_item` requires a server-issued handle. The absence itself is established by the
    // inventory frame; encode the hotkey command directly, as required for an uncarried variant.
    let intent = oteryn_protocol_oteryn::world_object::UseIntent { target: oteryn_protocol_oteryn::world_object::UseTarget::ItemByDefinition(missing), use_with: None, use_with_map_item: None, use_with_position: None };
    let payload = oteryn_protocol_oteryn::world_object::encode_item_use_intent(&intent).map_err(|error| format!("variant encoder: {error:?}"))?;
    let before_missing = quantities(&client)?;
    let reply = client.session_mut().world_context_use(&payload).await?;
    let disposition = oteryn_protocol_oteryn::world_object::decode_use_result(&reply.payload).map_err(|error| format!("variant reply decode status={:?}: {error:?}", reply.status))?;
    if disposition != UseDisposition::NothingToUse || quantities(&client)? != before_missing { return Err(format!("missing variant USE status={:?} reply={disposition:?}", reply.status).into()); }
    row.pass("missing-variant-refusal", format!("status={:?} reply={disposition:?} no-delta", reply.status));
    logout(&mut client).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn next_step_is_first_unobserved_action() {
        let mut passed = BTreeSet::new();
        assert_eq!(next_step(&passed), "npc-purchase");
        passed.insert("npc-purchase");
        assert_eq!(next_step(&passed), "food-consume");
    }
}
