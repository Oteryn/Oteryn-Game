//! E2EF20/E02: native NPC dialogue coin exchange and durable backpack custody.
use super::*;
use oteryn_dev_client::client_runtime::npc_shop as runtime;
use oteryn_dev_client::{DevClientSession, EntityKind, EntityRef, SessionEvent};
use oteryn_protocol_oteryn::npc_service as wire;
use std::collections::BTreeMap;

const GOLD_ID: u32 = 3031;
const PLATINUM_ID: u32 = 3035;
const CRYSTAL_ID: u32 = 3043;
const CRYSTAL_VALUE: u64 = 10_000;
const DEPOSIT_CHUNK: u64 = 1_000;
const PROMPT: &str = "How many platinum coins would you like to get?";

struct Row {
    started: std::time::Instant,
    steps: BTreeMap<&'static str, bool>,
}

impl Row {
    fn new() -> Self {
        Self {
            started: std::time::Instant::now(),
            steps: [
                "legitimate-gold",
                "walk-and-greet",
                "gold-to-platinum",
                "platinum-to-gold",
                "platinum-to-crystal",
                "crystal-to-platinum",
                "insufficient-refusal",
                "bank-only-refusal",
                "fresh-admission-persistence",
            ]
            .into_iter()
            .map(|step| (step, false))
            .collect(),
        }
    }

    fn pass(&mut self, step: &'static str, observed: impl std::fmt::Debug) {
        if let Some(done) = self.steps.get_mut(step) {
            *done = true;
            evidence(&format!(
                "E2EF20_CHECK E02 {step} PASS observed={observed:?}"
            ));
        }
    }

    fn finish(self, result: &TestResult) {
        if let Err(error) = result {
            evidence(&format!("E2EF20_GAP E02 step=journey observed={error}"));
        }
        for (step, passed) in &self.steps {
            if !passed {
                evidence(&format!("E2EF20_GAP E02 step={step} observed=not-reached"));
            }
        }
        let suffix = result.as_ref().err().map_or_else(String::new, |error|
            verdict::classify(error.as_ref()).suffix("E02.full/journey"));
        evidence(&format!(
            "e2e_all scenario=E02.full status={} seconds={:.3}{suffix}",
            if result.is_ok()
                && conversion_coverage(
                    [
                        "gold-to-platinum",
                        "platinum-to-gold",
                        "platinum-to-crystal",
                        "crystal-to-platinum"
                    ]
                    .map(|step| self.steps[step])
                )
                && self.steps.values().all(|passed| *passed)
            {
                "PASS"
            } else {
                "FAIL"
            },
            self.started.elapsed().as_secs_f64()
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) -> TestResult {
    let mut row = Row::new();
    let earning = full_gold::allowance(DEPOSIT_CHUNK)
        * u32::try_from(CRYSTAL_VALUE.div_ceil(DEPOSIT_CHUNK)).unwrap_or(u32::MAX);
    let budget = earning + Duration::from_secs(180);
    let result = tokio::time::timeout(budget, journey(&mut row, clients)).await;
    let result = result.unwrap_or_else(|elapsed| Err(elapsed.into()));
    let released = release_row_characters(clients).await;
    row.finish(verdict::release_affects_verdict(&result, &released));
    released
}

fn coin_refs() -> TestResult<[std::num::NonZeroU32; 3]> {
    let index = provisioning::item_index()?;
    Ok([
        provisioning::item_reference(&index, GOLD_ID)?,
        provisioning::item_reference(&index, PLATINUM_ID)?,
        provisioning::item_reference(&index, CRYSTAL_ID)?,
    ])
}

fn backpack_coins(
    client: &DevClientSession,
    refs: [std::num::NonZeroU32; 3],
) -> TestResult<[u64; 3]> {
    let inventory = client
        .inventory()
        .ok_or("step=inventory observed=snapshot-absent")?;
    let mut counts = [0; 3];
    for entry in &inventory.entries {
        if let Some(index) = refs
            .iter()
            .position(|reference| *reference == entry.item_definition_ref)
        {
            counts[index] += u64::from(entry.count.get());
        }
    }
    Ok(counts)
}

async fn wait_coins(
    client: &mut DevClientSession,
    refs: [std::num::NonZeroU32; 3],
    expected: [u64; 3],
    step: &'static str,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    loop {
        let observed = backpack_coins(client, refs)?;
        if observed == expected {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "step={step} observed=coin-delta-timeout:{observed:?} expected={expected:?}"
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
}

pub(super) async fn wait_for_event<T>(
    client: &mut DevClientSession,
    window: Duration,
    mut matching: impl FnMut(SessionEvent) -> Option<T>,
) -> TestResult<Option<T>> {
    let deadline = tokio::time::Instant::now() + window;
    loop {
        let result = client
            .take_events()
            .into_iter()
            .filter_map(&mut matching)
            .next_back();
        if let Some(result) = result {
            return Ok(Some(result));
        }
        let now = tokio::time::Instant::now();
        if now >= deadline {
            return Ok(None);
        }
        // service_liveness stops at the first-byte boundary and completes any started frame.
        let read = client
            .service_liveness((deadline - now).min(Duration::from_millis(50)))
            .await;
        if let Some(result) = trade_probe_result(read, || {
            client
                .take_events()
                .into_iter()
                .filter_map(&mut matching)
                .next_back()
        })? {
            return Ok(Some(result));
        }
    }
}

pub(super) async fn send_npc_talk(
    client: &mut DevClientSession,
    actor: EntityRef,
    text: &str,
) -> TestResult<u64> {
    let payload = wire::encode_npc_talk_intent(&wire::NpcTalkIntent {
        npc_actor: actor,
        text: text.into(),
    })
    .map_err(|error| format!("step=talk-{text} observed=invalid-intent:{error:?}"))?;
    let limit = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        client.take_events();
        let reply = client.session_mut().gated_command(
            wire::CAPABILITY_NPC_SERVICE_V1,
            wire::COMMAND_TYPE_NPC_TALK_INTENT,
            &payload,
        ).await?;
        if reply.status != CommandStatus::Accepted {
            return Err(format!("step=talk-{text} observed={reply:?}").into());
        }
        let disposition = wire::decode_npc_intent_result(&reply.payload)
            .map_err(|error| format!("step=talk-{text} observed=malformed-result:{error:?}"))?;
        match disposition {
            wire::NpcIntentDisposition::Ok if tokio::time::Instant::now() < limit => {
                return Ok(reply.server_sequence);
            }
            wire::NpcIntentDisposition::RateLimited if tokio::time::Instant::now() < limit => {
                client.service_liveness(Duration::from_millis(100)).await?;
            }
            _ => {
                return Err(format!("step=talk-{text} observed=disposition:{disposition:?}").into());
            }
        }
    }
}

async fn talk(
    client: &mut DevClientSession,
    actor: EntityRef,
    text: &str,
) -> TestResult<wire::NpcConversation> {
    let sequence = send_npc_talk(client, actor, text).await?;
    wait_for_event(client, Duration::from_secs(2), |event| match event {
        SessionEvent::NpcConversation(delta)
            if delta.server_sequence > sequence
                && delta.value.open.is_some_and(|(npc, _)| npc == actor) =>
        {
            Some(delta.value)
        }
        _ => None,
    })
    .await?
    .ok_or_else(|| format!("step=talk-{text} observed=no-conversation-reply:{actor:?}").into())
}

fn visible_npc(
    client: &DevClientSession,
    basis: spatial_basis::Basis,
    npc: &oteryn_world_bundle::npc::Npc,
) -> Option<EntityRef> {
    client
        .world_entities()?
        .iter()
        .find(|entity| {
            entity.kind == EntityKind::Npc
                && npc.placements.iter().any(|placement| {
                    entity.position.x == i32::from(placement.x)
                        && entity.position.y == i32::from(placement.y)
                        && entity.position.floor == basis.floor(placement.floor)
                })
        })
        .map(|entity| entity.entity)
}

async fn approach(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npc: &oteryn_world_bundle::npc::Npc,
) -> TestResult<EntityRef> {
    let path = route(world, actor_tile(client)?, |at| {
        npc.placements.iter().any(|placement| {
            placement.floor == at.floor
                && at.x.abs_diff(placement.x).max(at.y.abs_diff(placement.y)) <= 3
        })
    })
    .ok_or_else(|| format!("step=walk-npc observed=no-route:{}", npc.key))?;
    walk(client, world, &path).await?;
    let basis = spatial_basis::current(client)?;
    let until = tokio::time::Instant::now() + Duration::from_secs(2);
    while tokio::time::Instant::now() < until {
        if let Some(actor) = visible_npc(client, basis, npc) {
            return Ok(actor);
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    Err(format!("step=walk-npc observed=actor-not-visible:{}", npc.key).into())
}

async fn find_exchange_npc(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
) -> TestResult<(EntityRef, String)> {
    let bankers: std::collections::BTreeSet<_> =
        full_gold::records("content/npcs/definitions", "npcs-")?
            .into_iter()
            .filter(|npc| {
                npc["services"].as_array().is_some_and(|services| {
                    services
                        .iter()
                        .any(|service| service["key"] == "oteryn:service.bank.standard")
                })
            })
            .filter_map(|npc| npc["identity"]["key"].as_str().map(str::to_owned))
            .collect();
    let here = actor_tile(client)?;
    let mut reachable: Vec<_> = npcs
        .npcs
        .iter()
        .filter(|npc| bankers.contains(&npc.key))
        .filter_map(|npc| {
            route(world, here, |at| {
                npc.placements.iter().any(|placement| {
                    placement.floor == at.floor
                        && at.x.abs_diff(placement.x).max(at.y.abs_diff(placement.y)) <= 3
                })
            })
            .map(|path| (path.len(), npc))
        })
        .collect();
    reachable.sort_by_key(|(steps, npc)| (*steps, &npc.key));
    let mut attempted = Vec::new();
    for (_, npc) in reachable {
        let actor = approach(client, world, npc).await?;
        let greeting = talk(client, actor, "hi").await?;
        attempted.push(format!("{}:hi={:?}", npc.key, greeting.lines));
        let reply = talk(client, actor, "change gold").await?;
        attempted.push(format!("{}:change-gold={:?}", npc.key, reply.lines));
        if reply.lines.iter().any(|line| line == PROMPT) {
            return Ok((actor, npc.key.clone()));
        }
    }
    Err(format!("step=find-exchange-npc observed=no-live-prompt attempts={attempted:?}; need=compiled-world-reachable-banker").into())
}

async fn exchange(
    client: &mut DevClientSession,
    actor: EntityRef,
    words: &[&str],
) -> TestResult<wire::NpcConversation> {
    let mut last = None;
    for word in words {
        last = Some(talk(client, actor, word).await?);
    }
    last.ok_or_else(|| "step=exchange observed=empty-command-sequence".into())
}

async fn bank_balance(client: &mut DevClientSession, actor: EntityRef) -> TestResult<u64> {
    let reply = talk(client, actor, "balance").await?;
    reply
        .lines
        .iter()
        .find_map(|line| {
            line.strip_prefix("Your account balance is ")?
                .strip_suffix(" gold.")?
                .parse()
                .ok()
        })
        .ok_or_else(|| format!("step=bank-balance observed={:?}", reply.lines).into())
}

async fn deposit_all(
    client: &mut DevClientSession,
    actor: EntityRef,
    refs: [std::num::NonZeroU32; 3],
) -> TestResult<u64> {
    let before = bank_balance(client, actor).await?;
    let carried = full_gold::coin_value(client)?;
    if carried == 0 {
        return Ok(before);
    }
    exchange(client, actor, &["deposit all", "yes"]).await?;
    wait_coins(client, refs, [0; 3], "deposit-all").await?;
    let after = bank_balance(client, actor).await?;
    if before.checked_add(carried) != Some(after) {
        return Err(
            format!("step=deposit-all before={before} deposited={carried} after={after}").into(),
        );
    }
    Ok(after)
}

async fn optional_shop_window<T>(
    mut observe: impl AsyncFnMut(Duration) -> TestResult<Option<T>>,
) -> TestResult<Option<T>> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    while let Some(remaining) = deadline.checked_duration_since(tokio::time::Instant::now()) {
        if let Some(window) = observe(remaining.min(Duration::from_millis(50))).await? {
            return Ok(Some(window));
        }
    }
    Ok(None)
}

#[test]
fn optional_shop_window_propagates_a_session_failure() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let result = optional_shop_window::<()>(async |_| {
                Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof).into())
            }).await;
            assert!(result.is_err(), "the shop wait reused a failed session");
            Ok(())
        })
}

pub(super) async fn fund_market_account(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
) -> TestResult {
    let refs = coin_refs()?;
    earn_market_gold(client, world, npcs, refs).await?;
    let (actor, _) = find_exchange_npc(client, world, npcs).await?;
    exchange(client, actor, &["deposit all", "yes"]).await?;
    wait_coins(client, refs, [0, 0, 0], "market funding deposit").await
}

async fn earn_market_gold(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    refs: [std::num::NonZeroU32; 3],
) -> TestResult<[u64; 3]> {
    let mut observed = backpack_coins(client, refs)?;
    let mut attempts = Vec::new();
    for npc in &npcs.npcs {
        if observed[0] >= 100
            || route(world, tile(client.world_spatial().actor_position)?, |at| {
                npc.placements.iter().any(|placement| placement.floor == at.floor
                    && at.x.abs_diff(placement.x).max(at.y.abs_diff(placement.y)) <= 3)
            }).is_none()
        {
            continue;
        }
        let actor = approach(client, world, npc).await?;
        let greeting = talk(client, actor, "hi").await?;
        let before_trade = client.last_server_sequence();
        let trade = talk(client, actor, "trade").await?;
        attempts.push(format!("{}:hi={:?}:trade={:?}", npc.key, greeting.lines, trade.lines));
        let window = optional_shop_window(async |duration| {
            let window = client.take_events().into_iter().find_map(|event| match event {
                oteryn_dev_client::SessionEvent::NpcTradeWindow(delta)
                    if delta.server_sequence > before_trade => Some(delta.value),
                _ => None,
            });
            if window.is_none() {
                // A quiet interval may finish before a frame starts. Once a
                // frame starts, the session finishes its read or reports failure.
                client.service_liveness(duration).await?;
            }
            Ok(window)
        }).await?;
        let Some(window) = window else { continue };
        let Some((_, revision)) = window.open else { continue };
        let carried: BTreeMap<_, _> = client.inventory()
            .ok_or("step=earn-gold observed=inventory-absent")?
            .entries.iter()
            .map(|entry| (entry.item_definition_ref.get(), entry.count.get()))
            .collect();
        for (index, offer) in window.offers.iter().enumerate() {
            if observed[0] >= 100 { break; }
            let Some(available) = carried.get(&offer.item_definition_ref).copied() else { continue };
            if offer.side != wire::NpcTradeSide::Sell || refs.iter().any(|r| r.get() == offer.item_definition_ref) {
                continue;
            }
            let needed = u32::try_from(100 - observed[0]).unwrap_or(u32::MAX);
            let quantity = available.min(needed.div_ceil(offer.unit_price.max(1))).max(1);
            let disposition = runtime::buy(client.session_mut(), &wire::NpcTradeIntent {
                npc_actor: actor,
                catalogue_revision: revision,
                offer_index: u32::try_from(index)?,
                side: offer.side,
                quantity,
                expected_unit_price: offer.unit_price,
            }).await?;
            let before = observed;
            if disposition != wire::NpcIntentDisposition::Ok {
                attempts.push(format!("{}:sell-ref={}:qty={quantity}:reply={disposition:?}", npc.key, offer.item_definition_ref));
                continue;
            }
            let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
            loop {
                observed = backpack_coins(client, refs)?;
                if observed[0] > before[0] { break; }
                if tokio::time::Instant::now() >= deadline {
                    return Err(format!("step=earn-gold observed=sale-accepted-without-gold-delta npc={} ref={} before={before:?} after={observed:?}", npc.key, offer.item_definition_ref).into());
                }
                client.service_liveness(Duration::from_millis(25)).await?;
                client.take_events();
            }
            attempts.push(format!("{}:sold-ref={}:qty={quantity}:price={}:coins={observed:?}", npc.key, offer.item_definition_ref, offer.unit_price));
        }
    }
    if observed[0] < 100 {
        return Err(format!("step=legitimate-gold observed=backpack-coins:{observed:?},sale-attempts={attempts:?}; need=reachable-compiled-world-buyer-for-starter-items").into());
    }
    Ok(observed)
}

async fn earn_gold(
    client: &mut DevClientSession,
    world: &BundleWorld,
    metadata: &oteryn_world_bundle::bundle::Visited,
    refs: [std::num::NonZeroU32; 3],
) -> TestResult<(EntityRef, String, u64)> {
    let (mut actor, npc_key) = find_exchange_npc(client, world, &metadata.npcs).await?;
    let starting_bank = deposit_all(client, actor, refs).await?;
    let npc = metadata
        .npcs
        .npcs
        .iter()
        .find(|npc| npc.key == npc_key)
        .ok_or("banker absent")?;
    let needed_new = CRYSTAL_VALUE.saturating_sub(starting_bank).max(100);
    let mut balance = starting_bank;
    let mut earned = 0;
    while !acquisition_observed(starting_bank, balance, needed_new) {
        let target = DEPOSIT_CHUNK.min(needed_new - earned);
        full_gold::earn(client, world, &metadata.spawns, "E02", target).await?;
        actor = approach(client, world, npc).await?;
        talk(client, actor, "hi").await?;
        balance = deposit_all(client, actor, refs).await?;
        earned = balance
            .checked_sub(starting_bank)
            .ok_or("step=legitimate-gold observed=bank-balance-regressed")?;
    }
    evidence(&format!(
        "E2EF20_ACQUISITION bank_before={starting_bank} bank_after={balance} newly_earned={earned}"
    ));
    Ok((actor, npc_key, earned))
}

fn trade_probe_result<T, E>(
    read: Result<(), E>,
    observe: impl FnOnce() -> Option<T>,
) -> Result<Option<T>, E> {
    read.map(|()| observe())
}

fn acquisition_observed(before: u64, after: u64, needed: u64) -> bool {
    after
        .checked_sub(before)
        .is_some_and(|earned| earned >= needed)
}

fn conversion_coverage(legs: [bool; 4]) -> bool {
    legs.into_iter().all(|observed| observed)
}

fn insufficient_gold_amount(gold: u64) -> TestResult<String> {
    let amount = u32::try_from(gold / 100 + 1)?;
    Ok(amount.to_string())
}

#[test]
fn insufficient_gold_probe_costs_more_than_the_backpack_balance() -> TestResult {
    for gold in [100, 199, 200, 1_000, 42_000] {
        let amount: u64 = insufficient_gold_amount(gold)?.parse()?;
        assert!((1..=u64::from(u32::MAX)).contains(&amount));
        assert!(amount * 100 > gold, "gold={gold} amount={amount}");
    }
    Ok(())
}

fn exact_delta(before: [u64; 3], after: [u64; 3], delta: [i64; 3]) -> bool {
    before
        .into_iter()
        .zip(after)
        .zip(delta)
        .all(|((before, after), delta)| i128::from(after) - i128::from(before) == i128::from(delta))
}

async fn checked_conversion(
    client: &mut DevClientSession,
    actor: EntityRef,
    refs: [std::num::NonZeroU32; 3],
    step: &'static str,
    words: &[&str],
    delta: [i64; 3],
) -> TestResult<([u64; 3], [u64; 3])> {
    let before = backpack_coins(client, refs)?;
    let [Some(gold), Some(platinum), Some(crystal)] =
        std::array::from_fn(|index| before[index].checked_add_signed(delta[index]))
    else {
        return Err(format!("step={step} observed=insufficient-input:{before:?}").into());
    };
    let reply = exchange(client, actor, words).await?;
    wait_coins(client, refs, [gold, platinum, crystal], step).await?;
    let after = backpack_coins(client, refs)?;
    if !exact_delta(before, after, delta) || !reply.lines.iter().any(|line| line == "Here you are.")
    {
        return Err(format!(
            "step={step} observed=reply:{:?},coins:{after:?}",
            reply.lines
        )
        .into());
    }
    Ok((before, after))
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
    let refs = coin_refs()?;
    let mut client = connect_world(clients, 0xb0).await?;
    let (actor, npc_key, newly_earned) = earn_gold(&mut client, world, &metadata, refs).await?;
    row.pass("legitimate-gold", newly_earned);
    row.pass("walk-and-greet", npc_key);

    exchange(&mut client, actor, &["withdraw 10000", "yes"]).await?;
    wait_coins(&mut client, refs, [0, 0, 1], "withdraw-crystal-funding").await?;
    for (step, words, delta) in [
        (
            "crystal-to-platinum",
            &["change crystal", "1", "yes"][..],
            [0, 100, -1],
        ),
        (
            "platinum-to-gold",
            &["change platinum", "gold", "1", "yes"][..],
            [100, -1, 0],
        ),
        (
            "gold-to-platinum",
            &["change gold", "1", "yes"][..],
            [-100, 1, 0],
        ),
        (
            "platinum-to-crystal",
            &["change platinum", "crystal", "1 crystal", "yes"][..],
            [0, -100, 1],
        ),
    ] {
        let observed = checked_conversion(&mut client, actor, refs, step, words, delta).await?;
        row.pass(step, observed);
    }
    let initial = backpack_coins(&client, refs)?;
    const INSUFFICIENT: &str = "Sorry, you do not have enough gold coins.";
    let amount = insufficient_gold_amount(initial[0])?;
    let reply = exchange(&mut client, actor, &["change gold", &amount, "yes"]).await?;
    let after_refusal = backpack_coins(&client, refs)?;
    if after_refusal != initial || !reply.lines.iter().any(|line| line == INSUFFICIENT) {
        return Err(format!(
            "step=insufficient-refusal observed=reply:{:?},coins:{after_refusal:?}",
            reply.lines
        )
        .into());
    }
    row.pass("insufficient-refusal", (reply.lines, after_refusal));

    let deposited = deposit_all(&mut client, actor, refs).await?;
    let refusal = exchange(&mut client, actor, &["change gold", "1", "yes"]).await?;
    if backpack_coins(&client, refs)? != [0; 3]
        || !refusal.lines.iter().any(|line| line == INSUFFICIENT)
        || bank_balance(&mut client, actor).await? != deposited
    {
        return Err(format!(
            "step=bank-only-refusal observed=bank:{deposited},exchange:{:?},coins:{:?}",
            refusal.lines,
            backpack_coins(&client, refs)?
        )
        .into());
    }
    row.pass("bank-only-refusal", (deposited, refusal.lines));

    let withdrawal = exchange(&mut client, actor, &["withdraw 100", "yes"]).await?;
    wait_coins(&mut client, refs, [0, 1, 0], "withdraw-after-refusal").await?;
    checked_conversion(
        &mut client,
        actor,
        refs,
        "withdrawn-platinum-to-gold",
        &["change platinum", "gold", "1", "yes"],
        [100, -1, 0],
    )
    .await?;
    let old_session = client.join_snapshot().game_session_id;
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    drop(client);
    let mut fresh = connect_world(clients, 0xb1).await?;
    let persisted = backpack_coins(&fresh, refs)?;
    if fresh.join_snapshot().game_session_id == old_session || persisted != [100, 0, 0] {
        return Err(format!(
            "step=fresh-admission-persistence observed=session-reused:{},coins:{persisted:?}",
            fresh.join_snapshot().game_session_id == old_session
        )
        .into());
    }
    row.pass("fresh-admission-persistence", (withdrawal.lines, persisted));
    if fresh.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=final-logout observed=refused".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trade_probe_preserves_transport_failure() {
        assert_eq!(trade_probe_result(Err("EOF"), || Some(1)), Err("EOF"));
        assert_eq!(trade_probe_result::<u32, &str>(Ok(()), || None), Ok(None));
        assert_eq!(
            trade_probe_result::<_, &str>(Ok(()), || Some(1)),
            Ok(Some(1))
        );
    }

    #[test]
    fn gold_acquisition_requires_new_money() {
        assert!(!acquisition_observed(100, 100, 100));
        assert!(acquisition_observed(0, 100, 100));
        assert!(acquisition_observed(100, 200, 100));
        assert!(!acquisition_observed(100, 199, 100));
    }

    #[test]
    fn exchange_coverage_requires_both_crystal_directions() {
        assert!(!conversion_coverage([true, true, false, false]));
        assert!(!conversion_coverage([true, true, true, false]));
        assert!(!conversion_coverage([true, true, false, true]));
        assert!(conversion_coverage([true; 4]));
    }

    #[test]
    fn exact_coin_delta_rejects_value_preserving_wrong_denominations() {
        assert!(exact_delta([100, 0, 0], [0, 1, 0], [-100, 1, 0]));
        assert!(!exact_delta([100, 0, 0], [0, 0, 1], [-100, 1, 0]));
        assert!(exact_delta([0, 1, 0], [100, 0, 0], [100, -1, 0]));
    }
}
