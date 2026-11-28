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
    "cask-keg-refill",
    "special-food",
];

// Tibia Fandom, Health Potion: npcprice = 50. The row buys two from a regular seller.
const HEALTH_POTION_PRICE: u64 = 50;
const MEAT_PRICE: u64 = 5;
const POTION_FUNDING: u32 = 100;
const POTION_PURCHASE_QUANTITY: u32 = 2;

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
    let result = verdict::indexed_timeout(Duration::from_secs(180), verdict::observe(journey(&mut row, clients))).await;
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
    wait_quantities(client, &[(reference, expected)]).await
}

async fn wait_quantities(client: &mut DevClientSession, expected: &[(std::num::NonZeroU32, u64)]) -> TestResult<u64> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        for event in client.take_events() {
            if let SessionEvent::Inventory(delta) = event
                && quantities_match(expected, |id| delta.value.entries.iter()
                    .filter(|item| item.item_definition_ref == id)
                    .map(|item| u64::from(item.count.get()))
                    .sum())
            {
                return Ok(delta.server_sequence);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("inventory delta timeout expected={expected:?} observed={:?}", quantities(client)?).into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }
}

pub(super) fn decode_trade_window(
    selected: &[u32],
    payload: &[u8],
) -> TestResult<oteryn_protocol_oteryn::npc_service::NpcTradeWindow> {
    let details = selected.contains(
        &oteryn_protocol_oteryn::trade_details::CAPABILITY_TRADE_DETAILS_V1,
    );
    oteryn_protocol_oteryn::npc_service::decode_npc_trade_window_for(payload, details)
        .map_err(|error| format!("trade window decode: {error:?}").into())
}

pub(super) async fn acquire(client: &mut DevClientSession, world: &BundleWorld, id: u32, amount: u32) -> TestResult<u64> {
    acquire_with_approach::<false>(client, world, id, amount).await
}

async fn acquire_with_approach<const I14: bool>(
    client: &mut DevClientSession,
    world: &BundleWorld,
    id: u32,
    amount: u32,
) -> TestResult<u64> {
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
    let talk_range = if I14 { wire::TALK_RANGE_TILES } else { 3 };
    let (npc, path) = metadata
        .npcs
        .npcs
        .iter()
        .filter(|npc| sellers.contains(&npc.key))
        .filter_map(|npc| {
            route(world, here, |at| {
                npc.placements.iter().any(|placement| seller_in_range(at, placement, talk_range))
            })
            .map(|path| (npc, path))
        })
        .min_by_key(|(_, path)| path.len())
        .ok_or_else(|| format!("no walk route to seller for item {id}; sellers={sellers:?}"))?;
    if I14 {
        for direction in path {
            let from = tile(client.world_spatial().actor_position)?;
            let expected = destination(world, from, direction)
                .ok_or("I14 seller route lost its planned landing")?;
            let reply = client.step(direction).await?;
            let actual = tile(client.world_spatial().actor_position)?;
            if reply.disposition != StepDisposition::Moved
                || reply.world_spatial_delta.is_none()
                || actual != expected
            {
                return Err(format!(
                    "I14 seller STEP {direction:?} from={from:?} planned={expected:?} actual={actual:?} reply={reply:?}"
                )
                .into());
            }
            verdict::Harness::global().progress("walk");
            client.take_events();
        }
    } else {
        walk(client, world, &path).await?;
    }
    let actor = client.world_entities().and_then(|entities| entities.iter().find(|entity| entity.kind == oteryn_dev_client::EntityKind::Npc && npc.placements.iter().any(|p| entity.position.x == i32::from(p.x) && entity.position.y == i32::from(p.y) && entity.position.floor == -i16::from(p.floor)))).map(|entry| entry.entity).ok_or("seller not visible after walk")?;
    for text in ["hi", "trade"] {
        let reply = runtime::talk(client.session_mut(), &wire::NpcTalkIntent { npc_actor: actor, text: text.into() }).await?;
        if reply != wire::NpcIntentDisposition::Ok { return Err(format!("NPC {text} reply={reply:?}").into()); }
    }
    let delta = client.session_mut().read_gated_delta(wire::STATE_DOMAIN_NPC_TRADE_WINDOW, 1).await?;
    let window = decode_trade_window(client.selected_capabilities(), &delta.payload)?;
    let (offer_index, offer) = window.offers.iter().enumerate().find(|(_, offer)| offer.side == wire::NpcTradeSide::Buy && offer.item_definition_ref == reference.get()).ok_or("live seller omitted requested offer")?;
    let before = quantity(client, reference)?;
    client.take_events();
    let reply = runtime::buy(client.session_mut(), &wire::NpcTradeIntent { npc_actor: actor, catalogue_revision: window.open.ok_or("trade window closed")?.1, offer_index: u32::try_from(offer_index)?, side: offer.side, quantity: amount, expected_unit_price: offer.unit_price }).await?;
    if reply != wire::NpcIntentDisposition::Ok { return Err(format!("buy item={id} reply={reply:?} price={} quantity={amount}", offer.unit_price).into()); }
    let expected = before + u64::from(offer.count) * u64::from(amount);
    wait_quantity(client, reference, expected).await
}

fn seller_in_range(
    at: TilePos,
    seller: &oteryn_world_bundle::npc::Placement,
    range: u32,
) -> bool {
    at.floor == seller.floor
        && u32::from(at.x.abs_diff(seller.x).max(at.y.abs_diff(seller.y))) <= range
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

fn check_debit(before: u64, after: u64, cost: u64) -> TestResult {
    if before.checked_sub(cost) != Some(after) {
        return Err(format!("I14 gold debit expected {cost}: before={before} after={after}").into());
    }
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
    let fixture = character_prep::dedicated_character(clients, "NODE_BOOT_I14_ACCOUNT").await?;
    let scope = fixture.clients(clients);
    let mut client = connect_character(&scope, 0xb4, 0).await?;
    let seed_session = client.join_snapshot().game_session_id;
    let starting_gold = full_e2ef7::player_gold(&mut client).await?;
    character_prep::declare_seeded("I14.full", "gold coin (3031) x100", "potion purchase funding");
    character_prep::grant_item(&client, 3031, POTION_FUNDING).await?;
    logout(&mut client).await?;
    drop(client);
    let mut client = connect_character(&scope, 0xb4, 0).await?;
    let funded_gold = full_e2ef7::player_gold(&mut client).await?;
    if client.join_snapshot().game_session_id == seed_session
        || starting_gold.checked_add(u64::from(POTION_FUNDING)) != Some(funded_gold)
    {
        return Err(format!("I14 gold fixture expected +{POTION_FUNDING}: before={starting_gold} after={funded_gold}").into());
    }
    let purchase_meat = acquire_with_approach::<true>(&mut client, world, 3577, 2).await?;
    let after_meat = full_e2ef7::player_gold(&mut client).await?;
    check_debit(funded_gold, after_meat, 2 * MEAT_PRICE)?;
    let potion_before = quantity(&client, potion)?;
    let purchase_potion = acquire_with_approach::<true>(&mut client, world, 266, POTION_PURCHASE_QUANTITY).await?;
    if quantity(&client, potion)? != potion_before + u64::from(POTION_PURCHASE_QUANTITY) {
        return Err("buy item=266 did not credit exactly two health potions".into());
    }
    let after_potion = full_e2ef7::player_gold(&mut client).await?;
    check_debit(after_meat, after_potion, HEALTH_POTION_PRICE * u64::from(POTION_PURCHASE_QUANTITY))?;
    row.pass("npc-purchase", format!("meat_sequence={purchase_meat} potion_sequence={purchase_potion} gold={funded_gold}->{after_meat}->{after_potion}"));
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
    client = connect_character(&scope, 0xb4, 0).await?;
    if client.join_snapshot().game_session_id == session || quantities(&client)? != durable || client.actor_vitals().copied() != vitals || full_e2ef7::player_gold(&mut client).await? != after_potion { return Err("fresh admission changed durable consumable state/vitals".into()); }
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
    let refill = refill_keg(&mut client, world, flask, potion).await;
    let released = logout(&mut client).await;
    refill?;
    row.pass("cask-keg-refill", "accepted-flask-debit-and-potion-credit");
    released?;
    Err("missing=ordinary-special-food-acquisition-and-effect-observation owner=crates/gameplay/durability/src/durability/item_use.rs".into())
}

async fn refill_keg(
    client: &mut DevClientSession,
    world: &BundleWorld,
    flask: std::num::NonZeroU32,
    potion: std::num::NonZeroU32,
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::{MapUseTarget, UseIntent, UseTarget, decode_use_result, encode_item_use_intent};

    let keg_id = 25903;
    acquire_with_approach::<true>(client, world, keg_id, 1).await.map_err(|error| {
        verdict::context(format!(
            "missing=ordinary-health-potion-keg-{keg_id}-acquisition observed={error} owner=crates/gameplay/dispatch/src/gameplay_transport/item_craft/dispenser.rs:6"
        ), error)
    })?;
    let keg = provisioning::item_reference(&provisioning::item_index()?, keg_id)?;
    let inventory = client.inventory().ok_or("keg-use inventory missing")?;
    let handle = |definition| inventory.entries.iter()
        .find(|item| item.item_definition_ref == definition).map(|item| item.handle);
    let intent = UseIntent {
        target: UseTarget::Item(handle(flask).ok_or("keg-use flask handle missing")?),
        use_with: None,
        use_with_position: None,
        use_with_map_item: Some(MapUseTarget::Item(handle(keg).ok_or("keg-use purchased keg handle missing")?)),
    };
    let flask_before = quantity(client, flask)?;
    let potion_before = quantity(client, potion)?;
    let payload = encode_item_use_intent(&intent).map_err(|error| format!("keg-use encode: {error:?}"))?;
    client.take_events();
    let reply = client.session_mut().world_context_use(&payload).await?;
    let disposition = decode_use_result(&reply.payload).map_err(|error| format!("keg-use decode: {error:?}"))?;
    if reply.status != CommandStatus::Accepted || disposition != UseDisposition::Committed {
        return Err(format!("keg-use status={:?} disposition={disposition:?}", reply.status).into());
    }
    wait_quantities(client, &[(flask, flask_before - 1), (potion, potion_before + 1)]).await?;
    Ok(())
}


fn quantities_match(expected: &[(std::num::NonZeroU32, u64)], count: impl Fn(std::num::NonZeroU32) -> u64) -> bool {
    expected.iter().all(|&(id, units)| count(id) == units)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn purchase_debit_rejects_no_op_overcharge_and_underflow() -> TestResult {
        check_debit(140, 40, 100)?;
        assert!(check_debit(140, 140, 100).is_err());
        assert!(check_debit(140, 39, 100).is_err());
        assert!(check_debit(99, 0, 100).is_err());
        Ok(())
    }

    #[test]
    fn review_second_refill_requires_both_legs_in_one_observation() -> TestResult {
        let flask = std::num::NonZeroU32::new(285).ok_or("flask id")?;
        let potion = std::num::NonZeroU32::new(266).ok_or("potion id")?;
        let expected = [(flask, 1), (potion, 3)];
        assert!(!quantities_match(&expected, |id| if id == flask { 1 } else { 2 }));
        assert!(quantities_match(&expected, |id| if id == flask { 1 } else { 3 }));
        Ok(())
    }

    #[test]
    fn ordinary_consumables_do_not_complete_cask_and_special_food_coverage() {
        let passed = BTreeSet::from([
            "npc-purchase",
            "food-consume",
            "potion-consume-flask",
            "potion-exhaust-refusal",
            "fresh-admission",
            "missing-variant-refusal",
        ]);
        assert_eq!(next_step(&passed), "cask-keg-refill");
    }

    #[test]
    fn i14_seller_approach_uses_the_wire_boundary_on_the_same_floor() {
        let seller = oteryn_world_bundle::npc::Placement {
            x: 32398,
            y: 32222,
            floor: -7,
            direction: oteryn_world_bundle::npc::Direction::North,
        };
        let at = TilePos { x: 32394, y: 32226, floor: -7 };
        let range = oteryn_protocol_oteryn::npc_service::TALK_RANGE_TILES;
        assert!(seller_in_range(at, &seller, range));
        assert!(!seller_in_range(at, &seller, 3));
        assert!(!seller_in_range(TilePos { x: at.x - 1, ..at }, &seller, range));
        assert!(!seller_in_range(TilePos { floor: -6, ..at }, &seller, range));
    }

    #[test]
    fn next_step_is_first_unobserved_action() {
        let mut passed = BTreeSet::new();
        assert_eq!(next_step(&passed), "npc-purchase");
        passed.insert("npc-purchase");
        assert_eq!(next_step(&passed), "food-consume");
    }
}

#[test]
fn trade_window_respects_negotiated_quote_details() -> TestResult {
    use oteryn_protocol_oteryn::{
        npc_service::{self as wire, NpcOffer, NpcTradeSide, NpcTradeWindow},
        trade_details::{CAPABILITY_TRADE_DETAILS_V1, NpcQuoteDetails},
    };

    let mut window = NpcTradeWindow {
        open: Some((oteryn_dev_client::EntityRef { identity: [1; 16], generation: 1 }, 9)),
        offers: vec![NpcOffer {
            item_definition_ref: 42,
            side: NpcTradeSide::Buy,
            count: 1,
            unit_price: 17,
        }],
        details: Vec::new(),
    };
    let legacy = wire::encode_npc_trade_window(&window)
        .map_err(|error| format!("legacy fixture encoding: {error:?}"))?;
    assert_eq!(decode_trade_window(&[], &legacy)?, window);
    window.details.push(NpcQuoteDetails { name: "fish".into(), maximum_quantity: 37 });
    let extended = wire::encode_npc_trade_window_for(&window, true)
        .map_err(|error| format!("extended fixture encoding: {error:?}"))?;
    assert!(decode_trade_window(&[], &extended).is_err());
    assert_eq!(decode_trade_window(&[CAPABILITY_TRADE_DETAILS_V1], &extended)?, window);
    Ok(())
}
