//! E2EF20/E02: native NPC dialogue coin exchange and durable backpack custody.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind, EntityRef};
use oteryn_dev_client::client_runtime::npc_shop as runtime;
use oteryn_protocol_oteryn::npc_service as wire;
use std::collections::BTreeMap;

const GOLD_ID: u32 = 3031;
const PLATINUM_ID: u32 = 3035;
const CRYSTAL_ID: u32 = 3043;
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
            if result.is_ok() && self.steps.values().all(|passed| *passed) {
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
    let result = tokio::time::timeout(Duration::from_secs(180), journey(&mut row, clients))
        .await.unwrap_or_else(|elapsed| Err(elapsed.into()));
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

fn backpack_coins(client: &DevClientSession, refs: [std::num::NonZeroU32; 3]) -> TestResult<[u64; 3]> {
    let inventory = client.inventory().ok_or("step=inventory observed=snapshot-absent")?;
    let mut counts = [0; 3];
    for entry in &inventory.entries {
        if let Some(index) = refs.iter().position(|reference| *reference == entry.item_definition_ref) {
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
            return Err(format!("step={step} observed=coin-delta-timeout:{observed:?} expected={expected:?}").into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
}

async fn talk(client: &mut DevClientSession, actor: EntityRef, text: &str) -> TestResult<wire::NpcConversation> {
    tokio::time::sleep(Duration::from_millis(260)).await;
    let disposition = runtime::talk(
        client.session_mut(),
        &wire::NpcTalkIntent { npc_actor: actor, text: text.into() },
    )
    .await?;
    if disposition != wire::NpcIntentDisposition::Ok {
        return Err(format!("step=talk-{text} observed=disposition:{disposition:?}").into());
    }
    let delta = client
        .session_mut()
        .read_gated_delta(wire::STATE_DOMAIN_NPC_CONVERSATION, 1)
        .await?;
    wire::decode_npc_conversation(&delta.payload)
        .map_err(|error| format!("step=talk-{text} observed=malformed-conversation:{error:?}").into())
}

fn visible_npc(client: &DevClientSession, npc: &oteryn_world_bundle::npc::Npc) -> Option<EntityRef> {
    client.world_entities()?.iter().find(|entity| {
        entity.kind == EntityKind::Npc
            && npc.placements.iter().any(|placement| {
                entity.position.x == i32::from(placement.x)
                    && entity.position.y == i32::from(placement.y)
                    && entity.position.floor == -i16::from(placement.floor)
            })
    }).map(|entity| entity.entity)
}

async fn approach(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npc: &oteryn_world_bundle::npc::Npc,
) -> TestResult<EntityRef> {
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        npc.placements.iter().any(|placement| {
            placement.floor == at.floor
                && at.x.abs_diff(placement.x).max(at.y.abs_diff(placement.y)) <= 3
        })
    }).ok_or_else(|| format!("step=walk-npc observed=no-route:{}", npc.key))?;
    walk(client, world, &path).await?;
    visible_npc(client, npc).ok_or_else(|| format!("step=walk-npc observed=actor-not-visible:{}", npc.key).into())
}

async fn find_exchange_npc(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
) -> TestResult<(EntityRef, String)> {
    let mut attempted = Vec::new();
    for npc in &npcs.npcs {
        if route(world, tile(client.world_spatial().actor_position)?, |at| {
            npc.placements.iter().any(|placement| placement.floor == at.floor
                && at.x.abs_diff(placement.x).max(at.y.abs_diff(placement.y)) <= 3)
        }).is_none() {
            continue;
        }
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

async fn earn_gold(
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
    evidence(&format!("E2EF20_ACQUISITION legitimate-player-sales={attempts:?}"));
    Ok(observed)
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
    before.into_iter().zip(after).zip(delta).all(|((before, after), delta)| {
        i128::from(after) - i128::from(before) == i128::from(delta)
    })
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(bytes, &pin, WorldId::decode(&clients.world)?, ChannelId::decode(&clients.channel)?)?;
    let refs = coin_refs()?;
    let mut client = connect_world(clients, 0xb0).await?;
    let initial = earn_gold(&mut client, world, &metadata.npcs, refs).await?;
    row.pass("legitimate-gold", initial);

    let (actor, npc_key) = find_exchange_npc(&mut client, world, &metadata.npcs).await?;
    row.pass("walk-and-greet", npc_key);

    let reply = exchange(&mut client, actor, &["1", "yes"]).await?;
    let platinum = [initial[0] - 100, initial[1] + 1, initial[2]];
    wait_coins(&mut client, refs, platinum, "gold-to-platinum").await?;
    if !exact_delta(initial, platinum, [-100, 1, 0]) || !reply.lines.iter().any(|line| line == "Here you are.") {
        return Err(format!("step=gold-to-platinum observed=reply:{:?},coins:{platinum:?}", reply.lines).into());
    }
    row.pass("gold-to-platinum", (initial, platinum, reply.lines));

    let reply = exchange(&mut client, actor, &["change platinum", "gold", "1", "yes"]).await?;
    wait_coins(&mut client, refs, initial, "platinum-to-gold").await?;
    if !exact_delta(platinum, initial, [100, -1, 0]) || !reply.lines.iter().any(|line| line == "Here you are.") {
        return Err(format!("step=platinum-to-gold observed=reply:{:?},coins:{:?}", reply.lines, backpack_coins(&client, refs)?).into());
    }
    row.pass("platinum-to-gold", (platinum, initial, reply.lines));

    let amount = insufficient_gold_amount(initial[0])?;
    let reply = exchange(&mut client, actor, &["change gold", &amount, "yes"]).await?;
    let after_refusal = backpack_coins(&client, refs)?;
    if after_refusal != initial || !reply.lines.iter().any(|line| line.contains("enough")) {
        return Err(format!("step=insufficient-refusal observed=reply:{:?},coins:{after_refusal:?}", reply.lines).into());
    }
    row.pass("insufficient-refusal", (reply.lines, after_refusal));

    let reply = exchange(&mut client, actor, &["deposit all", "yes"]).await?;
    wait_coins(&mut client, refs, [0, 0, 0], "deposit-all").await?;
    let refusal = exchange(&mut client, actor, &["change gold", "1", "yes"]).await?;
    if backpack_coins(&client, refs)? != [0, 0, 0] || !refusal.lines.iter().any(|line| line.contains("enough")) {
        return Err(format!("step=bank-only-refusal observed=deposit:{:?},exchange:{:?},coins:{:?}", reply.lines, refusal.lines, backpack_coins(&client, refs)?).into());
    }
    row.pass("bank-only-refusal", (reply.lines, refusal.lines));

    let withdrawal = exchange(&mut client, actor, &["withdraw 100", "yes"]).await?;
    wait_coins(&mut client, refs, [100, 0, 0], "withdraw-after-refusal").await?;
    let old_session = client.join_snapshot().game_session_id;
    if client.session_mut().logout().await?.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err("step=logout observed=refused".into());
    }
    drop(client);
    let mut fresh = connect_world(clients, 0xb1).await?;
    let persisted = backpack_coins(&fresh, refs)?;
    if fresh.join_snapshot().game_session_id == old_session || persisted != [100, 0, 0] {
        return Err(format!("step=fresh-admission-persistence observed=session-reused:{},coins:{persisted:?}", fresh.join_snapshot().game_session_id == old_session).into());
    }
    row.pass("fresh-admission-persistence", (withdrawal.lines, persisted));
    if fresh.session_mut().logout().await?.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err("step=final-logout observed=refused".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_coin_delta_rejects_value_preserving_wrong_denominations() {
        assert!(exact_delta([100, 0, 0], [0, 1, 0], [-100, 1, 0]));
        assert!(!exact_delta([100, 0, 0], [0, 0, 1], [-100, 1, 0]));
        assert!(exact_delta([0, 1, 0], [100, 0, 0], [100, -1, 0]));
    }
}
