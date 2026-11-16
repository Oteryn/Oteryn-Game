//! E2EF4: real-world economy/social probes, with fail-closed full-row evidence.
//! No fixture provisioning, SQL mutations, clock overrides or invented wire handles.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityRef};

use verdict::Row;

// Detail stays on the GAP lines, separate from the report's single-token evidence field.
// A failure is never turned into MISSING-SERVER merely because this journey is unfinished.
const EVIDENCE: &str = "source:apps/game-server/src/qualification/client_phases/full_e2ef4.rs:1";

const E01: &[&str] = &[
    "greet",
    "look",
    "buy",
    "sell",
    "sell-all",
    "deposit",
    "withdraw",
    "transfer",
    "relog-inventory-bank",
];
const E03: &[&str] = &[
    "gold-up",
    "platinum-down",
    "platinum-up",
    "crystal-down",
    "converter-charge",
    "converter-retirement",
    "magic-toggle",
    "magic-tick",
    "relog-coins-charges",
];
const E04: &[&str] = &[
    "request",
    "look",
    "accept",
    "cancel",
    "logout-race",
    "relog-custody",
];
const E05: &[&str] = &[
    "browse",
    "create",
    "cancel",
    "accept",
    "history",
    "relog-settlement",
];
const E07: &[&str] = &["depot", "inbox", "stow", "chosen-withdraw", "relog-custody"];
const S01: &[&str] = &[
    "invite",
    "join",
    "leadership",
    "shared-xp-toggle",
    "shared-xp-split",
    "leave",
];
const S03: &[&str] = &[
    "guild",
    "ranks",
    "motd",
    "chat",
    "bank",
    "wars",
    "relog-guild-bank",
];
const S09: &[&str] = &[
    "pz-logout",
    "fight-refusal",
    "nologout",
    "idle-warning",
    "idle-removal",
    "gameplay-resets-idle",
    "relog-position",
];
const S18: &[&str] = &[
    "say",
    "whisper",
    "yell",
    "range",
    "yell-level-premium",
    "mute",
];

pub(super) async fn run(
    client: &mut Option<DevClientSession>,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
) {
    // Each row reports even when another row has missing content or a refused command.
    // Socket waits are bounded; a timed-out session is not evidence of server absence.
    macro_rules! row {
        ($id:literal, $actions:ident, $journey:ident $(, $arg:expr)*) => {{
            let mut row = Row::new("E2EF4", $id, $actions).evidence(EVIDENCE, EVIDENCE);
            let watch = row.watch();
            let result = if let Some(active) = client.as_mut() {
                let mut completed = false;
                let result = watch.bound(
                    Duration::from_secs(90),
                    "native-journey-deadline;remaining-assertions-unproven",
                    async {
                        let result = $journey(&mut row, active $(, $arg)*).await;
                        completed = true;
                        result
                    },
                ).await;
                if !completed {
                    // An interrupted command or frame cannot be resumed by another row.
                    *client = None;
                }
                result
            } else {
                Err("prior-native-journey-deadline;connection-closed;actions-unproven".into())
            };
            row.finish(&result);
        }};
    }
    row!("E01", E01, shop, clients, world, npcs);
    row!("E03", E03, coins, world, npcs);
    row!("E04", E04, trade, clients, world);
    row!("E05", E05, market, clients, world);
    row!("E07", E07, storage, clients, world);
    row!("S01", S01, party, clients, world);
    row!("S03", S03, guild);
    row!("S09", S09, logout, clients, world);
    row!("S18", S18, speech);
    let Some(client) = client.as_mut() else {
        return;
    };
    // Restore the entry position expected by the pre-existing world phases.
    let restore: TestResult = async {
        let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
            at == TOWN
        })
        .ok_or("E2EF4 return route unavailable")?;
        walk(client, world, &path).await
    }
    .await;
    if let Err(error) = restore {
        evidence(&format!("E2EF4_GAP return {error}"));
    }
}

fn own(client: &DevClientSession) -> TestResult<EntityRef> {
    let entities = client.world_entities().ok_or("native entities absent")?;
    entities
        .iter()
        .find(|entity| &entity.entity.identity == entities.own_identity())
        .map(|entity| entity.entity)
        .ok_or_else(|| "native own actor absent".into())
}

async fn approach_peer(
    client: &mut DevClientSession,
    peer: &DevClientSession,
    world: &BundleWorld,
) -> TestResult {
    let other = tile(peer.world_spatial().actor_position)?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at != other
            && at.floor == other.floor
            && at.x.abs_diff(other.x).max(at.y.abs_diff(other.y)) <= 1
    })
    .ok_or("real-peer approach route unavailable")?;
    walk(client, world, &path).await
}

fn inventory_units(client: &DevClientSession) -> TestResult<BTreeMap<u32, u64>> {
    let inventory = client.inventory().ok_or("native inventory absent")?;
    let mut units = BTreeMap::new();
    for item in inventory
        .entries
        .iter()
        .chain(inventory.main_backpack.iter())
        .chain(inventory.equipment.iter().map(|e| &e.item))
    {
        *units.entry(item.item_definition_ref.get()).or_default() += u64::from(item.count.get());
    }
    Ok(units)
}

async fn wait_inventory(
    client: &mut DevClientSession,
    expected: &BTreeMap<u32, u64>,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if &inventory_units(client)? == expected {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("native-inventory-quantity-deadline".into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
        client.take_events();
    }
}

async fn relog(client: &mut DevClientSession, clients: &SeamClients<'_>) -> TestResult {
    let old = client.join_snapshot().game_session_id;
    let (result, logout, attempts, busy_timed_out) = super::logout_retrying_busy(client).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("durability-check logout refused: {logout}; attempts={attempts}; busy_timed_out={busy_timed_out}").into());
    }
    *client = connect_character(clients, 0x85, 1).await?;
    if client.join_snapshot().game_session_id == old {
        return Err("fresh-login reused old session".into());
    }
    Ok(())
}

async fn locker(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        near_locker(world, at)
    })
    .ok_or("no reachable production depot locker")?;
    walk(client, world, &path).await
}

/// Gold value of the carried coins and the carried non-coin units, from the client inventory.
fn split_coins(
    units: &BTreeMap<u32, u64>,
    refs: [std::num::NonZeroU32; 3],
) -> (BTreeMap<u32, u64>, u64) {
    let mut items = units.clone();
    let mut value = 0;
    for (reference, worth) in refs.into_iter().zip([1, 100, 10_000]) {
        value += items.remove(&reference.get()).unwrap_or(0) * worth;
    }
    (items, value)
}

/// Waits until the client inventory shows exactly `items` and coins worth `value`.
async fn wait_trade(
    client: &mut DevClientSession,
    refs: [std::num::NonZeroU32; 3],
    items: &BTreeMap<u32, u64>,
    value: u64,
    step: &str,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let (observed, coins) = split_coins(&inventory_units(client)?, refs);
        if &observed == items && coins == value {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(
                format!("{step}-inventory-deadline;coins={coins};expected-coins={value}").into(),
            );
        }
        client.service_liveness(Duration::from_millis(20)).await?;
        client.take_events();
    }
}

fn adjust(items: &mut BTreeMap<u32, u64>, definition: u32, delta: i64) -> TestResult {
    let entry = items.entry(definition).or_default();
    *entry = entry
        .checked_add_signed(delta)
        .ok_or_else(|| format!("inventory-unit-underflow;definition={definition}"))?;
    if *entry == 0 {
        items.remove(&definition);
    }
    Ok(())
}

/// One NPC utterance through the native talk adapter, reduced by the native dialogue panel.
async fn say(
    client: &mut DevClientSession,
    service: &mut oteryn_dev_client::client_ui::npc_shop::NpcServicePanel,
    actor: EntityRef,
    text: &str,
) -> TestResult<Vec<String>> {
    use oteryn_dev_client::{client_runtime::npc_shop as runtime, client_ui::npc_shop as ui};
    use ui::npc_service as wire;
    // The NPC talk rate limit refuses utterances closer than its interval.
    tokio::time::sleep(Duration::from_millis(260)).await;
    let disposition = runtime::talk(
        client.session_mut(),
        &wire::NpcTalkIntent {
            npc_actor: actor,
            text: text.into(),
        },
    )
    .await?;
    if disposition != wire::NpcIntentDisposition::Ok {
        return Err(format!("npc-say-{text}:{disposition:?}").into());
    }
    let delta = client
        .session_mut()
        .read_gated_delta(wire::STATE_DOMAIN_NPC_CONVERSATION, 1)
        .await?;
    if !ui::apply_conversation(service, &delta.payload) {
        return Err(format!("npc-say-{text}:conversation-reducer-rejected").into());
    }
    Ok(service
        .conversation
        .as_ref()
        .map(|conversation| conversation.lines.clone())
        .unwrap_or_default())
}

const BALANCE_PREFIX: &str = "Your account balance is ";

fn balance_of(lines: &[String]) -> TestResult<u64> {
    let line = lines
        .iter()
        .rev()
        .find_map(|line| line.strip_prefix(BALANCE_PREFIX))
        .ok_or_else(|| format!("no-balance-line;lines={lines:?}"))?;
    Ok(line.trim_end_matches(" gold.").parse()?)
}

async fn balance(
    client: &mut DevClientSession,
    service: &mut oteryn_dev_client::client_ui::npc_shop::NpcServicePanel,
    actor: EntityRef,
) -> TestResult<u64> {
    balance_of(&say(client, service, actor, "balance").await?)
}

/// A prompted bank operation: the native dialogue must prompt, then confirm with `done`.
async fn bank(
    client: &mut DevClientSession,
    service: &mut oteryn_dev_client::client_ui::npc_shop::NpcServicePanel,
    actor: EntityRef,
    command: &str,
    prompt: &str,
    done: &str,
) -> TestResult {
    let lines = say(client, service, actor, command).await?;
    if !lines.iter().any(|line| line == prompt) {
        return Err(format!("{command}:no-prompt;lines={lines:?}").into());
    }
    let lines = say(client, service, actor, "yes").await?;
    if !lines.iter().any(|line| line == done) {
        return Err(format!("{command}:not-confirmed;lines={lines:?}").into());
    }
    Ok(())
}

/// The first reachable NPC whose native dialogue answers `balance` with an account balance.
async fn find_banker<'n>(
    client: &mut DevClientSession,
    service: &mut oteryn_dev_client::client_ui::npc_shop::NpcServicePanel,
    world: &BundleWorld,
    npcs: &'n oteryn_world_bundle::npc::Table,
) -> TestResult<(&'n oteryn_world_bundle::npc::Npc, EntityRef, u64)> {
    let mut attempted = Vec::new();
    for npc in &npcs.npcs {
        let from = tile(client.world_spatial().actor_position)?;
        if route(world, from, |at| {
            npc.placements
                .iter()
                .any(|p| p.floor == at.floor && at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)) <= 3)
        })
        .is_none()
        {
            continue;
        }
        let actor = super::full_e2ef20::approach(client, world, npc).await?;
        say(client, service, actor, "hi").await?;
        let lines = say(client, service, actor, "balance").await?;
        if let Ok(amount) = balance_of(&lines) {
            return Ok((npc, actor, amount));
        }
        attempted.push(npc.key.clone());
    }
    Err(format!("content:no-reachable-banker;attempted={attempted:?}").into())
}

async fn shop(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
) -> TestResult {
    use oteryn_dev_client::{client_runtime::npc_shop as runtime, client_ui::npc_shop as ui};
    use ui::npc_service as wire;
    let refs = super::full_e2ef20::coin_refs()?;
    let coin_ids = refs.map(std::num::NonZeroU32::get);
    let npc = npcs
        .npcs
        .iter()
        .find(|npc| npc.key == "oteryn:npc.sam")
        .ok_or("content:oteryn:npc.sam absent")?;
    let actor = super::full_e2ef20::approach(client, world, npc).await?;
    let mut service = ui::NpcServicePanel::default();
    say(client, &mut service, actor, "hi").await?;
    if service
        .conversation
        .as_ref()
        .is_none_or(|conversation| conversation.open.is_none())
    {
        return Err("greeting did not open conversation".into());
    }
    row.pass("greet", "native-conversation-open");
    say(client, &mut service, actor, "trade").await?;
    let delta = client
        .session_mut()
        .read_gated_delta(wire::STATE_DOMAIN_NPC_TRADE_WINDOW, 1)
        .await?;
    if !ui::apply_window(&mut service, &delta.payload) {
        return Err("shop window reducer rejected the payload".into());
    }
    let window = service.window.clone().ok_or("shop window absent")?;
    let (npc_actor, revision) = window.open.ok_or("trade window closed")?;
    let offer_intent = |index: usize, action| -> TestResult<runtime::npc_shop::NpcShopIntent> {
        let offer = window.offers.get(index).ok_or("offer index")?;
        Ok(runtime::npc_shop::NpcShopIntent {
            npc_actor,
            catalogue_revision: revision,
            offer_index: u32::try_from(index)?,
            side: offer.side,
            action,
            expected_unit_price: offer.unit_price,
        })
    };
    let trade_intent = |index: usize| -> TestResult<wire::NpcTradeIntent> {
        let offer = window.offers.get(index).ok_or("offer index")?;
        Ok(wire::NpcTradeIntent {
            npc_actor,
            catalogue_revision: revision,
            offer_index: u32::try_from(index)?,
            side: offer.side,
            quantity: 1,
            expected_unit_price: offer.unit_price,
        })
    };
    let cheapest = window
        .offers
        .iter()
        .enumerate()
        .filter(|(_, o)| o.side == wire::NpcTradeSide::Buy)
        .min_by_key(|(_, o)| o.unit_price)
        .map(|(index, _)| index)
        .ok_or("content:oteryn:npc.sam has no buy offer")?;
    let mut panel = ui::NpcShopPanel {
        selected: true,
        ..Default::default()
    };
    let command = ui::request(
        &mut panel,
        ui::Intent::Look(offer_intent(
            cheapest,
            runtime::npc_shop::NpcShopAction::Look,
        )?),
    )
    .ok_or("shop look input")?;
    ui::apply_result(
        &mut panel,
        &runtime::send(client.session_mut(), &command).await?,
    );
    if panel.feedback.is_some()
        || panel
            .result
            .as_ref()
            .is_none_or(|result| result.look_text.is_empty())
    {
        return Err(format!("shop look did not describe the offer: {:?}", panel.feedback).into());
    }
    row.pass("look", "native-offer-description-nonempty");

    // SELL one lot of a carried starter item Sam buys: the item leaves, its price arrives as coins.
    let (mut items, mut value) = split_coins(&inventory_units(client)?, refs);
    let (sell, offer) = window
        .offers
        .iter()
        .enumerate()
        .find(|(_, o)| {
            o.side == wire::NpcTradeSide::Sell
                && !coin_ids.contains(&o.item_definition_ref)
                && items.get(&o.item_definition_ref).copied().unwrap_or(0) >= u64::from(o.count)
        })
        .ok_or("content:oteryn:npc.sam-buys-no-carried-starter-item")?;
    let disposition = runtime::trade(client.session_mut(), &trade_intent(sell)?).await?;
    if disposition != wire::NpcIntentDisposition::Ok {
        return Err(format!("sell refused: {disposition:?}").into());
    }
    adjust(
        &mut items,
        offer.item_definition_ref,
        -i64::from(offer.count),
    )?;
    value += u64::from(offer.unit_price);
    wait_trade(client, refs, &items, value, "sell").await?;
    row.pass(
        "sell",
        format!(
            "definition={};units={};gold={}",
            offer.item_definition_ref, offer.count, offer.unit_price
        ),
    );

    // BUY the cheapest affordable lot, preferring one Sam buys back one unit at a time.
    let buy_back = |definition: u32| {
        window.offers.iter().position(|o| {
            o.side == wire::NpcTradeSide::Sell
                && o.count == 1
                && o.item_definition_ref == definition
        })
    };
    let mut affordable: Vec<_> = window
        .offers
        .iter()
        .enumerate()
        .filter(|(_, o)| o.side == wire::NpcTradeSide::Buy && u64::from(o.unit_price) <= value)
        .collect();
    affordable.sort_by_key(|(_, o)| (buy_back(o.item_definition_ref).is_none(), o.unit_price));
    let (buy, offer) = affordable.first().copied().ok_or_else(|| {
        format!("content:no-buy-offer-affordable-with-legitimate-gold;coins={value}")
    })?;
    let disposition = runtime::trade(client.session_mut(), &trade_intent(buy)?).await?;
    if disposition != wire::NpcIntentDisposition::Ok {
        return Err(format!("buy refused: {disposition:?}").into());
    }
    adjust(
        &mut items,
        offer.item_definition_ref,
        i64::from(offer.count),
    )?;
    value -= u64::from(offer.unit_price);
    wait_trade(client, refs, &items, value, "buy").await?;
    row.pass(
        "buy",
        format!(
            "definition={};units={};gold={}",
            offer.item_definition_ref, offer.count, offer.unit_price
        ),
    );

    // SELL-ALL a count-1 SELL offer of a carried definition through the native shop panel.
    let sell_all = buy_back(offer.item_definition_ref)
        .or_else(|| {
            window.offers.iter().position(|o| {
                o.side == wire::NpcTradeSide::Sell
                    && o.count == 1
                    && !coin_ids.contains(&o.item_definition_ref)
                    && items.contains_key(&o.item_definition_ref)
            })
        })
        .ok_or("content:oteryn:npc.sam-has-no-count-1-sell-offer-for-a-carried-item")?;
    let command = ui::request(
        &mut panel,
        ui::Intent::SellAll(offer_intent(
            sell_all,
            runtime::npc_shop::NpcShopAction::SellAll,
        )?),
    )
    .ok_or("sell-all input")?;
    ui::apply_result(
        &mut panel,
        &runtime::send(client.session_mut(), &command).await?,
    );
    let sold = panel.result.clone().ok_or("sell-all result absent")?;
    let offer = *window.offers.get(sell_all).ok_or("sell-all offer index")?;
    if panel.feedback.is_some()
        || sold.sold_units == 0
        || sold.total_gold_units != u64::from(sold.sold_units) * u64::from(offer.unit_price)
    {
        return Err(format!("sell-all: {:?} {sold:?}", panel.feedback).into());
    }
    adjust(
        &mut items,
        offer.item_definition_ref,
        -i64::from(sold.sold_units),
    )?;
    value += sold.total_gold_units;
    wait_trade(client, refs, &items, value, "sell-all").await?;
    row.pass(
        "sell-all",
        format!(
            "definition={};units={};gold={}",
            offer.item_definition_ref, sold.sold_units, sold.total_gold_units
        ),
    );

    // Bank through the native dialogue: deposit, withdraw and transfer, each confirmed.
    let (banker, actor, opening) = find_banker(client, &mut service, world, npcs).await?;
    let deposit = value.min(10);
    if deposit < 2 {
        return Err(
            format!("content:legitimate-gold-below-bank-journey-minimum;coins={value}").into(),
        );
    }
    bank(
        client,
        &mut service,
        actor,
        &format!("deposit {deposit} gold"),
        &format!("Do you want to deposit {deposit} gold?"),
        &format!("You have deposited {deposit} gold."),
    )
    .await?;
    value -= deposit;
    wait_trade(client, refs, &items, value, "deposit").await?;
    let after = balance(client, &mut service, actor).await?;
    if after != opening + deposit {
        return Err(format!("deposit balance {opening}->{after}").into());
    }
    row.pass(
        "deposit",
        format!("banker={};gold={deposit};balance={after}", banker.key),
    );
    bank(
        client,
        &mut service,
        actor,
        "withdraw 1 gold",
        "Do you want to withdraw 1 gold?",
        "You have withdrawn 1 gold.",
    )
    .await?;
    value += 1;
    wait_trade(client, refs, &items, value, "withdraw").await?;
    let withdrawn = balance(client, &mut service, actor).await?;
    if withdrawn != after - 1 {
        return Err(format!("withdraw balance {after}->{withdrawn}").into());
    }
    row.pass("withdraw", format!("gold=1;balance={withdrawn}"));
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let recipient: String = sqlx::query_scalar(
        "SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(clients.characters[0].as_slice())
    .fetch_one(&mut connection)
    .await?;
    let recipient_balance = async |connection: &mut sqlx::PgConnection| -> TestResult<i64> {
        Ok(sqlx::query_scalar(
            "SELECT COALESCE((SELECT balance FROM game_account_bank_balances WHERE account_id=$1::uuid AND world_id=encode($2,'hex')::uuid),0)::bigint",
        )
        .bind(&clients.accounts[0])
        .bind(clients.world.as_slice())
        .fetch_one(connection)
        .await?)
    };
    let received_before = recipient_balance(&mut connection).await?;
    bank(
        client,
        &mut service,
        actor,
        &format!("transfer 1 gold to {recipient}"),
        &format!("Do you want to transfer 1 gold to {recipient}?"),
        &format!("Very well. You have transferred 1 gold to {recipient}."),
    )
    .await?;
    let transferred = balance(client, &mut service, actor).await?;
    let received_after = recipient_balance(&mut connection).await?;
    if transferred != withdrawn - 1 || received_after != received_before + 1 {
        return Err(format!(
            "transfer balances sender {withdrawn}->{transferred} recipient {received_before}->{received_after}"
        )
        .into());
    }
    row.pass(
        "transfer",
        format!("gold=1;balance={transferred};recipient=+1"),
    );

    // A fresh admission shows the same inventory and the same bank balance.
    let carried = inventory_units(client)?;
    relog(client, clients).await?;
    if inventory_units(client)? != carried {
        return Err("fresh-admission inventory differs from the committed trades".into());
    }
    let actor = super::full_e2ef20::approach(client, world, banker).await?;
    let mut service = ui::NpcServicePanel::default();
    say(client, &mut service, actor, "hi").await?;
    let persisted = balance(client, &mut service, actor).await?;
    if persisted != transferred {
        return Err(format!("fresh-admission balance {persisted} != {transferred}").into());
    }
    row.pass(
        "relog-inventory-bank",
        format!("inventory=same;balance={persisted}"),
    );
    Ok(())
}

/// Native USE of a carried coin stack; waits for the backpack coin counts the use must produce.
async fn use_coin(
    client: &mut DevClientSession,
    refs: [std::num::NonZeroU32; 3],
    coin: usize,
    stack: fn(u32) -> bool,
    delta: [i64; 3],
    step: &'static str,
) -> TestResult<String> {
    let entry = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|e| e.item_definition_ref == refs[coin] && stack(e.count.get()))
        })
        .copied()
        .ok_or_else(|| format!("{step}:no-carried-coin-stack-to-use"))?;
    let before = super::full_e2ef20::backpack_coins(client, refs)?;
    let mut expected = before;
    for (slot, change) in expected.iter_mut().zip(delta) {
        *slot = slot
            .checked_add_signed(change)
            .ok_or_else(|| format!("{step}:coin-underflow"))?;
    }
    let outcome = client.use_item(entry.handle).await?;
    if outcome.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed {
        return Err(format!("{step}:use-{:?}", outcome.disposition).into());
    }
    super::full_e2ef20::wait_coins(client, refs, expected, step).await?;
    Ok(format!("coins={before:?}->{expected:?}"))
}

async fn coins(
    row: &mut Row,
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
) -> TestResult {
    let refs = super::full_e2ef20::coin_refs()?;
    // Gold comes only from ordinary NPC sales of carried starter items.
    super::full_e2ef20::earn_gold(client, world, npcs, refs).await?;
    // USE of a gold stack of at least 100 is GoldToPlatinum (coin_use.rs).
    let observed = use_coin(
        client,
        refs,
        0,
        |count| count >= 100,
        [-100, 1, 0],
        "gold-up",
    )
    .await?;
    row.pass("gold-up", observed);
    let observed = use_coin(
        client,
        refs,
        1,
        |count| count < 100,
        [100, -1, 0],
        "platinum-down",
    )
    .await?;
    row.pass("platinum-down", observed);
    Err("content-gap:platinum-up-needs-100-platinum-(10000-gold)-and-crystal-down-a-crystal-coin-from-a-legitimate-source;converter-charge/converter-retirement/magic-toggle/magic-tick/relog-coins-charges-need-a-gold-or-magic-converter-(i23722/i25719/i28525/i28526)-that-no-reachable-npc-catalogue-offers;source:crates/gameplay/dispatch/src/gameplay_transport/coin_use.rs:1".into())
}

async fn trade(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    use oteryn_dev_client::{
        client_runtime::player_trade as runtime, client_ui::player_trade as ui,
    };
    // The cancel is still exercised when the bootstrap has no transferable leaf.
    let mut panel = ui::PlayerTradePanel {
        selected: true,
        ..Default::default()
    };
    let command = ui::request(&mut panel, ui::Intent::Cancel).ok_or("trade cancel input")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    ui::apply_result(&mut panel, &reply);
    if reply.state.trade_id.is_some() {
        return Err("cancel retained open trade".into());
    }
    // Cancelling no active trade does not qualify the row's cancellation action.
    let item = client
        .inventory()
        .and_then(|i| i.entries.first())
        .copied()
        .ok_or("no direct-backpack leaf for real trade;starter backpack itself is not a leaf")?;
    let mut peer = connect_character(clients, 0x81, 0).await?;
    let mut peer_open = true;
    let result: TestResult = async {
        approach_peer(client, &peer, world).await?;
        let command = runtime::PlayerTradeCommand::new(runtime::player_trade::Intent::Offer {
            partner: own(&peer)?,
            item: item.handle.get(),
        })
        .ok_or("trade offer encoding")?;
        let reply = runtime::send(client.session_mut(), &command).await?;
        if reply.result != runtime::player_trade::ResultCode::Offered {
            return Err(format!("trade request: {:?}", reply.result).into());
        }
        row.pass("request", "real-peer-offer-opened");
        peer.session_mut().read_gated_delta(26, 1).await?;
        let id = reply.state.trade_id.ok_or("offer has no trade id")?;
        let command = runtime::PlayerTradeCommand::new(runtime::player_trade::Intent::Look {
            trade_id: id,
            counter_offer: false,
        })
        .ok_or("trade look encoding")?;
        let reply = runtime::send(client.session_mut(), &command).await?;
        if reply.state.look.is_empty() {
            return Err("offered leaf look was empty".into());
        }
        row.pass("look", "offered-leaf-description");
        let command = runtime::PlayerTradeCommand::new(runtime::player_trade::Intent::Cancel)
            .ok_or("trade cancel encoding")?;
        let reply = runtime::send(client.session_mut(), &command).await?;
        if reply.result != runtime::player_trade::ResultCode::Cancelled || reply.state.phase != 4 {
            return Err("active trade cancellation did not close escrow".into());
        }
        row.pass("cancel", "active-offer-closed");
        // Offer and counter-offer through the native panels, then both accept.
        let counter = peer
            .inventory()
            .and_then(|i| i.entries.first())
            .copied()
            .ok_or("content:peer-starter-has-no-direct-backpack-leaf-to-counter-offer")?;
        let mut panel = ui::PlayerTradePanel {
            selected: true,
            ..Default::default()
        };
        let mut peer_panel = ui::PlayerTradePanel {
            selected: true,
            ..Default::default()
        };
        let command = ui::request(
            &mut panel,
            ui::Intent::Offer {
                partner: own(&peer)?,
                item: item.handle.get(),
            },
        )
        .ok_or("trade offer input")?;
        let reply = runtime::send(client.session_mut(), &command).await?;
        ui::apply_result(&mut panel, &reply);
        if reply.result != runtime::player_trade::ResultCode::Offered {
            return Err(format!("second trade request: {:?}", reply.result).into());
        }
        peer.session_mut().read_gated_delta(26, 1).await?;
        let command = ui::request(
            &mut peer_panel,
            ui::Intent::Offer {
                partner: own(client)?,
                item: counter.handle.get(),
            },
        )
        .ok_or("trade counter offer input")?;
        let reply = runtime::send(peer.session_mut(), &command).await?;
        ui::apply_result(&mut peer_panel, &reply);
        if reply.result != runtime::player_trade::ResultCode::Offered {
            return Err(format!("trade counter offer: {:?}", reply.result).into());
        }
        let delta = client.session_mut().read_gated_delta(26, 1).await?;
        panel.state = Some(
            runtime::player_trade::decode_state(&delta.payload)
                .map_err(|error| format!("trade update: {error:?}"))?,
        );
        let command = ui::request(&mut panel, ui::Intent::Accept).ok_or("trade accept input")?;
        let reply = runtime::send(client.session_mut(), &command).await?;
        ui::apply_result(&mut panel, &reply);
        if reply.result != runtime::player_trade::ResultCode::Accepted {
            return Err(format!("trade accept: {:?}", reply.result).into());
        }
        let delta = peer.session_mut().read_gated_delta(26, 1).await?;
        peer_panel.state = Some(
            runtime::player_trade::decode_state(&delta.payload)
                .map_err(|error| format!("trade counter update: {error:?}"))?,
        );
        let command =
            ui::request(&mut peer_panel, ui::Intent::Accept).ok_or("trade counter accept input")?;
        let reply = runtime::send(peer.session_mut(), &command).await?;
        ui::apply_result(&mut peer_panel, &reply);
        if reply.result != runtime::player_trade::ResultCode::Completed {
            return Err(format!("trade completion: {:?}", reply.result).into());
        }
        client.session_mut().read_gated_delta(26, 1).await?;
        let mut mine = inventory_units(client)?;
        let mut theirs = inventory_units(&peer)?;
        for (units, gone, came) in [(&mut mine, item, counter), (&mut theirs, counter, item)] {
            for (entry, sign) in [(gone, -1), (came, 1)] {
                let slot = units.entry(entry.item_definition_ref.get()).or_default();
                *slot = slot
                    .checked_add_signed(sign * i64::from(entry.count.get()))
                    .ok_or("trade unit underflow")?;
                if *slot == 0 {
                    units.remove(&entry.item_definition_ref.get());
                }
            }
        }
        wait_inventory(client, &mine).await?;
        wait_inventory(&mut peer, &theirs).await?;
        row.pass("accept", "both-inventories-exchanged-exactly");

        // A partner that logs out while an offer is open ends the trade; the offer stays carried.
        let received = client
            .inventory()
            .and_then(|i| {
                i.entries
                    .iter()
                    .find(|e| e.item_definition_ref == counter.item_definition_ref)
            })
            .copied()
            .ok_or("received leaf absent from the direct backpack")?;
        let command = runtime::PlayerTradeCommand::new(runtime::player_trade::Intent::Offer {
            partner: own(&peer)?,
            item: received.handle.get(),
        })
        .ok_or("race offer encoding")?;
        let reply = runtime::send(client.session_mut(), &command).await?;
        if reply.result != runtime::player_trade::ResultCode::Offered {
            return Err(format!("race offer: {:?}", reply.result).into());
        }
        peer.session_mut().read_gated_delta(26, 1).await?;
        let (logout, detail, attempts, busy_timed_out) =
            super::logout_retrying_busy(&mut peer).await?;
        peer_open = false;
        if logout != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            return Err(format!("partner logout during open trade refused: {detail}; attempts={attempts}; busy_timed_out={busy_timed_out}").into());
        }
        let delta = client.session_mut().read_gated_delta(26, 1).await?;
        let state = runtime::player_trade::decode_state(&delta.payload)
            .map_err(|error| format!("race update: {error:?}"))?;
        if state.trade_id.is_some() {
            return Err(format!("partner logout left the trade open: {state:?}").into());
        }
        wait_inventory(client, &mine).await?;
        row.pass("logout-race", "partner-logout-closed-trade;offer-retained");

        // Fresh admissions show the exchanged custody on both sides.
        relog(client, clients).await?;
        if inventory_units(client)? != mine {
            return Err("fresh-admission inventory differs from the completed trade".into());
        }
        let mut fresh = connect_character(clients, 0x86, 0).await?;
        let persisted = inventory_units(&fresh)?;
        let (logout, detail, attempts, busy_timed_out) =
            super::logout_retrying_busy(&mut fresh).await?;
        if logout != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            return Err(format!("partner fresh logout refused: {detail}; attempts={attempts}; busy_timed_out={busy_timed_out}").into());
        }
        if persisted != theirs {
            return Err("partner fresh-admission inventory differs from the completed trade".into());
        }
        row.pass("relog-custody", "both-fresh-admissions-match-exchange");
        Ok(())
    }
    .await;
    if !peer_open {
        return result;
    }
    let (cleanup_result, cleanup, attempts, busy_timed_out) =
        super::logout_retrying_busy(&mut peer).await?;
    if cleanup_result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("trade peer cleanup logout refused: {cleanup}; attempts={attempts}; busy_timed_out={busy_timed_out}; journey={result:?}").into());
    }
    result
}

async fn market(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    use oteryn_dev_client::{client_runtime::market as runtime, client_ui::market as ui};
    locker(client, world).await?;
    let mut panel = ui::MarketPanel {
        selected: true,
        ..Default::default()
    };
    for (intent, action) in [
        (ui::Intent::Browse, "browse"),
        (ui::Intent::History, "history"),
    ] {
        let command = ui::request(&mut panel, intent).ok_or("market request input")?;
        let reply = runtime::send(client.session_mut(), &command).await?;
        if !ui::apply_result(&mut panel, &reply) {
            return Err(format!("market {action}: {:?}", reply.reply).into());
        }
        // Empty history is a successful query, but not settlement history evidence.
        if action == "browse" {
            row.pass("browse", "native-market-browse-decoded");
        }
    }
    let item = client
        .inventory()
        .and_then(|i| i.entries.first().or(i.main_backpack.as_ref()))
        .copied()
        .ok_or("no native item definition for market offer")?;
    panel.definition = item.item_definition_ref.get();
    panel.sell = false;
    let command = ui::request(&mut panel, ui::Intent::CreateOffer).ok_or("market create input")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    if !ui::apply_result(&mut panel, &reply) {
        return Err(format!(
            "market-create-refused:{:?};no-minted-funds-or-premium",
            reply.reply
        )
        .into());
    }
    let id = reply.reply.ok_or("market create receipt missing")?.offer;
    let command = ui::request(&mut panel, ui::Intent::OwnOffers).ok_or("market own input")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    if !ui::apply_result(&mut panel, &reply)
        || !panel.rows.iter().any(|r| {
            r.offer == id && r.amount == 1 && r.price == 1 && r.definition == panel.definition
        })
    {
        return Err("created market offer absent or quantity/price differ".into());
    }
    row.pass("create", "own-offers-exact-id-definition-amount-price");
    panel.selected_offer = Some(id);
    let command = ui::request(&mut panel, ui::Intent::CancelOffer).ok_or("market cancel input")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    if !ui::apply_result(&mut panel, &reply) {
        return Err("market cancellation refused".into());
    }
    let command = ui::request(&mut panel, ui::Intent::OwnOffers).ok_or("market verify cancel")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    if !ui::apply_result(&mut panel, &reply) || panel.rows.iter().any(|r| r.offer == id) {
        return Err("cancelled offer remains active".into());
    }
    row.pass("cancel", "cancelled-id-absent-from-own-offers");
    let command = ui::request(&mut panel, ui::Intent::History).ok_or("market history input")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    if !ui::apply_result(&mut panel, &reply)
        || !panel.rows.iter().any(|r| r.offer == id && r.state == 2)
    {
        return Err("cancelled offer absent from history".into());
    }
    row.pass("history", "cancelled-id-in-history-state-2");

    // A second player sells into a fresh one-unit buy offer through its own native panel.
    let command = ui::request(&mut panel, ui::Intent::CreateOffer).ok_or("market create input")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    if !ui::apply_result(&mut panel, &reply) {
        return Err(format!("second market create refused: {:?}", reply.reply).into());
    }
    let accepted = reply
        .reply
        .ok_or("second market create receipt missing")?
        .offer;
    let mut peer = connect_character(clients, 0x87, 0).await?;
    let result: TestResult = async {
        let mut units = inventory_units(&peer)?;
        let carried = units.entry(panel.definition).or_default();
        *carried = carried
            .checked_sub(1)
            .ok_or("content:peer-starter-does-not-carry-the-offered-definition")?;
        if *carried == 0 {
            units.remove(&panel.definition);
        }
        locker(&mut peer, world).await?;
        let mut peer_panel = ui::MarketPanel {
            selected: true,
            selected_offer: Some(accepted),
            ..Default::default()
        };
        let command =
            ui::request(&mut peer_panel, ui::Intent::AcceptOffer).ok_or("market accept input")?;
        let reply = runtime::send(peer.session_mut(), &command).await?;
        if !ui::apply_result(&mut peer_panel, &reply) {
            return Err(format!("market accept refused: {:?}", reply.reply).into());
        }
        wait_inventory(&mut peer, &units).await
    }
    .await;
    let (logout, detail, attempts, busy_timed_out) = super::logout_retrying_busy(&mut peer).await?;
    result?;
    if logout != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("market peer logout refused: {detail}; attempts={attempts}; busy_timed_out={busy_timed_out}").into());
    }
    let settled = |panel: &ui::MarketPanel| {
        [id, accepted]
            .iter()
            .all(|offer| panel.rows.iter().any(|r| r.offer == *offer && r.state != 1))
    };
    let command = ui::request(&mut panel, ui::Intent::History).ok_or("market history input")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    if !ui::apply_result(&mut panel, &reply) || !settled(&panel) {
        return Err("accepted offer absent from the creator's history".into());
    }
    row.pass(
        "accept",
        "peer-sold-into-buy-offer;item-left-peer;offer-ended-in-history",
    );

    // A fresh admission at the same locker shows the same ended offers and no active one.
    relog(client, clients).await?;
    locker(client, world).await?;
    let mut panel = ui::MarketPanel {
        selected: true,
        ..Default::default()
    };
    let command = ui::request(&mut panel, ui::Intent::OwnOffers).ok_or("market own input")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    if !ui::apply_result(&mut panel, &reply)
        || panel
            .rows
            .iter()
            .any(|r| r.offer == id || r.offer == accepted)
    {
        return Err("fresh-admission own offers still list an ended offer".into());
    }
    let command = ui::request(&mut panel, ui::Intent::History).ok_or("market history input")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    if !ui::apply_result(&mut panel, &reply) || !settled(&panel) {
        return Err("fresh-admission history lost an ended offer".into());
    }
    row.pass(
        "relog-settlement",
        "fresh-admission-history-and-own-offers-match",
    );
    Ok(())
}

async fn storage(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    use oteryn_dev_client::{client_runtime::stash as runtime, client_ui::stash as ui};
    use oteryn_protocol_oteryn::depot;
    locker(client, world).await?;
    for box_number in [1] {
        let payload = depot::encode_depot_intent(depot::DepotIntent {
            town: None,
            box_number,
            page: 0,
            action: depot::DepotAction::Browse,
        })
        .map_err(|e| format!("depot intent: {e:?}"))?;
        let reply = client
            .session_mut()
            .gated_command(
                depot::CAPABILITY_DEPOT_V1,
                depot::COMMAND_TYPE_DEPOT_INTENT,
                &payload,
            )
            .await?;
        let reply =
            depot::decode_depot_reply(&reply.payload).map_err(|e| format!("depot reply: {e:?}"))?;
        if reply.outcome != depot::DepotOutcome::Opened
            || reply.box_number != box_number
            || reply.town.is_none()
        {
            return Err(format!("depot/inbox browse: {reply:?}").into());
        }
        // Open views alone do not prove depot/inbox custody.
    }
    let mut panel = ui::StashPanel {
        selected: true,
        ..Default::default()
    };
    let command = ui::request(&mut panel, ui::Intent::Refresh).ok_or("stash browse input")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    ui::apply_result(&mut panel, &reply);
    if reply.reply.outcome != ui::stash::StashOutcome::Opened {
        return Err(format!("stash open: {:?}", reply.reply.outcome).into());
    }
    let item = client
        .inventory()
        .and_then(|i| i.entries.iter().find(|i| i.count.get() >= 2))
        .copied()
        .ok_or("no carried stack of at least two units for chosen-count withdrawal")?;
    let carried = inventory_units(client)?;
    let before: u64 = panel.rows.iter().map(|r| u64::from(r.quantity)).sum();
    let command =
        ui::request(&mut panel, ui::Intent::Stow(item.handle)).ok_or("stash stow input")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    ui::apply_result(&mut panel, &reply);
    if reply.reply.outcome != ui::stash::StashOutcome::Committed
        || reply.reply.stowed_quantity != item.count.get()
    {
        return Err(format!("stash stow: {:?}", reply.reply).into());
    }
    let command = ui::request(&mut panel, ui::Intent::Refresh).ok_or("stash refresh after stow")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    ui::apply_result(&mut panel, &reply);
    let after: u64 = panel.rows.iter().map(|r| u64::from(r.quantity)).sum();
    if after != before + u64::from(item.count.get()) {
        return Err("stash count delta differs from stowed quantity".into());
    }
    row.pass("stow", "stash-total-increased-by-exact-stack");
    let definitions = provisioning::item_index()?;
    let selected = panel
        .rows
        .iter()
        .position(|r| {
            definitions.definition_ref("Item", &r.definition_key, &r.definition_revision)
                == Some(item.item_definition_ref)
        })
        .ok_or("stowed definition absent from stash page")?;
    let command = ui::request(
        &mut panel,
        ui::Intent::Withdraw {
            row: selected,
            amount: 1,
        },
    )
    .ok_or("chosen-count withdrawal input")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    ui::apply_result(&mut panel, &reply);
    if reply.reply.outcome != ui::stash::StashOutcome::Committed {
        return Err(format!("withdraw refused: {:?}", reply.reply.outcome).into());
    }
    let command =
        ui::request(&mut panel, ui::Intent::Refresh).ok_or("stash refresh after withdraw")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    ui::apply_result(&mut panel, &reply);
    if panel
        .rows
        .iter()
        .map(|r| u64::from(r.quantity))
        .sum::<u64>()
        != after - 1
    {
        return Err("chosen one-unit withdrawal changed the wrong stash quantity".into());
    }
    let mut expected = carried;
    let units = expected
        .get_mut(&item.item_definition_ref.get())
        .ok_or("stowed inventory definition absent")?;
    *units = units
        .checked_sub(u64::from(item.count.get()) - 1)
        .ok_or("stow inventory underflow")?;
    wait_inventory(client, &expected).await?;
    row.pass(
        "chosen-withdraw",
        "one-unit-inventory-increase-and-one-unit-stash-decrease",
    );
    let stash_before: BTreeMap<_, _> = panel
        .rows
        .iter()
        .map(|r| (r.definition_key.clone(), r.quantity))
        .collect();
    relog(client, clients).await?;
    if inventory_units(client)? != expected {
        return Err("stash relog inventory changed".into());
    }
    let command =
        ui::request(&mut panel, ui::Intent::Refresh).ok_or("fresh-session stash browse")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    ui::apply_result(&mut panel, &reply);
    let stash_after: BTreeMap<_, _> = panel
        .rows
        .iter()
        .map(|r| (r.definition_key.clone(), r.quantity))
        .collect();
    if reply.reply.outcome != ui::stash::StashOutcome::Opened || stash_after != stash_before {
        return Err("fresh-session stash differs from committed withdrawal".into());
    }
    row.pass(
        "relog-custody",
        "fresh-admission-inventory-and-stash-quantities-retained",
    );
    Err(
        "depot-move-and-inbox-retrieval assertions unfinished;box-17-is-not-treated-as-an-inbox"
            .into(),
    )
}

async fn party(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    use oteryn_dev_client::client_runtime::party as runtime;
    use runtime::party::{PartyDisposition, PartyIntent};
    async fn send(
        client: &mut DevClientSession,
        intent: Option<PartyIntent>,
    ) -> TestResult<runtime::party::PartyResult> {
        let command = match intent {
            Some(intent) => {
                runtime::PartyCommand::intent(intent).map_err(|e| format!("party input: {e:?}"))?
            }
            None => runtime::PartyCommand::query(),
        };
        let reply = runtime::send(client.session_mut(), &command).await?;
        let operation = runtime::party::decode_party_result(&reply.operation)
            .map_err(|e| format!("party operation: {e:?}"))?;
        if operation.disposition != PartyDisposition::Ok {
            return Err(format!("party refused: {:?}", operation.disposition).into());
        }
        runtime::party::decode_party_result(
            reply.view.as_deref().ok_or("party current view absent")?,
        )
        .map_err(|e| format!("party view: {e:?}").into())
    }
    let mut peer = connect_character(clients, 0x82, 0).await?;
    let result: TestResult = async {
        approach_peer(client, &peer, world).await?;
        send(client, Some(PartyIntent::Invite(own(&peer)?))).await?;
        let invitations = send(&mut peer, None).await?;
        let handle = invitations
            .invitations
            .first()
            .ok_or("party invitation not visible to peer")?
            .handle;
        row.pass("invite", "peer-current-query-has-invitation");
        let state = send(&mut peer, Some(PartyIntent::Accept(handle))).await?;
        if state.members.len() != 2 {
            return Err("party join did not produce two members".into());
        }
        row.pass("join", "two-current-members");
        let result = client
            .chat(&oteryn_dev_client::ChatIntent::Say {
                mode: oteryn_dev_client::ChatSpeechMode::Say,
                text: "/sharedxp on".into(),
            })
            .await?;
        if result.disposition != oteryn_dev_client::ChatDisposition::Ok
            || !send(client, None).await?.shared_xp_enabled
        {
            return Err("shared XP toggle not reflected by party owner".into());
        }
        row.pass(
            "shared-xp-toggle",
            "leader-chat-toggle-current-state-enabled",
        );
        let state = send(client, None).await?;
        let member = state
            .members
            .iter()
            .find(|m| !m.leader)
            .ok_or("no nonleader member")?;
        let target_name = member.name.clone();
        let state = send(client, Some(PartyIntent::PassLeadership(member.handle))).await?;
        if !state
            .members
            .iter()
            .any(|m| m.leader && m.name == target_name)
        {
            return Err("leadership did not pass to selected member".into());
        }
        row.pass("leadership", "current-query-selected-member-is-leader");
        send(client, Some(PartyIntent::Leave)).await?;
        if !send(client, None).await?.members.is_empty() {
            return Err("party leave retained membership".into());
        }
        row.pass("leave", "leaver-current-query-empty");
        Err("shared-XP-eligible-kill-and-split assertion unfinished".into())
    }
    .await;
    let _ = send(client, Some(PartyIntent::Leave)).await;
    let (cleanup_result, cleanup, attempts, busy_timed_out) =
        super::logout_retrying_busy(&mut peer).await?;
    if cleanup_result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("party peer cleanup logout refused: {cleanup}; attempts={attempts}; busy_timed_out={busy_timed_out}; journey={result:?}").into());
    }
    result
}

async fn guild(row: &mut Row, client: &mut DevClientSession) -> TestResult {
    use oteryn_dev_client::client_runtime::guild as runtime;
    row.missing_server = Some("source:crates/protocol-oteryn/src/guild.rs:17");
    let reply = runtime::send(
        client.session_mut(),
        &runtime::GuildCommand::Query(runtime::guild::GuildQuery::Own),
    )
    .await?;
    // Native GuildIntent has membership/rank writers but no MOTD writer. Reading
    // GuildSummary.message cannot prove an update. Keep every other gap visible too.
    evidence(
        "E2EF4_MISSING S03 motd source:crates/protocol-oteryn/src/guild.rs:17 native-GuildIntent-has-no-MOTD-write",
    );
    Err(format!("guild-query={reply:?};guild/ranks/chat/bank/wars/relog assertions unfinished;MOTD-write-unavailable").into())
}

async fn logout(
    row: &mut Row,
    _client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    let mut peer = connect_character(clients, 0x83, 0).await?;
    let position = peer.world_spatial().actor_position;
    if !world
        .base()
        .tile(tile(position)?.x, tile(position)?.y, tile(position)?.floor)
        .is_some_and(|t| t.protection_zone())
    {
        let (_, cleanup, attempts, busy_timed_out) = super::logout_retrying_busy(&mut peer).await?;
        return Err(format!("logout peer is not on authored PZ tile: cleanup={cleanup}; attempts={attempts}; busy_timed_out={busy_timed_out}").into());
    }
    let session = peer.join_snapshot().game_session_id;
    let (result, logout, attempts, busy_timed_out) = super::logout_retrying_busy(&mut peer).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!(
            "PZ logout refused: {logout}; attempts={attempts}; busy_timed_out={busy_timed_out}"
        )
        .into());
    }
    row.pass("pz-logout", "authored-PZ-native-logout-accepted");
    drop(peer);
    let mut peer = connect_character(clients, 0x84, 0).await?;
    let retained = peer.world_spatial().actor_position == position
        && peer.join_snapshot().game_session_id != session;
    let (cleanup_result, cleanup, attempts, busy_timed_out) =
        super::logout_retrying_busy(&mut peer).await?;
    if !retained || cleanup_result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("fresh-login-position-or-cleanup-mismatch: retained={retained}; cleanup={cleanup}; attempts={attempts}; busy_timed_out={busy_timed_out}").into());
    }
    row.pass("relog-position", "fresh-session-retained-saved-position");
    Err("fight/NOLOGOUT/15-minute-warning/16-minute-removal/gameplay-reset assertions unfinished;no-clock-override-used;source:crates/gameplay/dispatch/src/gameplay_transport/connection/serve/idle.rs:1".into())
}

async fn speech(row: &mut Row, client: &mut DevClientSession) -> TestResult {
    use oteryn_dev_client::{ChatDisposition, ChatIntent, ChatSpeechMode};
    for (action, mode) in [
        ("say", ChatSpeechMode::Say),
        ("whisper", ChatSpeechMode::Whisper),
        ("yell", ChatSpeechMode::Yell),
    ] {
        let result = client
            .chat(&ChatIntent::Say {
                mode,
                text: format!("E2EF4 {action}"),
            })
            .await?;
        evidence(&format!(
            "E2EF4_OBSERVATION S18 {action} {:?}",
            result.disposition
        ));
        if result.disposition == ChatDisposition::Ok {
            let expected = if mode == ChatSpeechMode::Yell {
                format!("E2EF4 {action}").to_uppercase()
            } else {
                format!("E2EF4 {action}")
            };
            let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
            loop {
                if client.chat_log().is_some_and(|log| log.lines().any(|line| matches!(
                    line, oteryn_dev_client::ChatLine::Local { mode: heard, text, .. } if *heard == mode && text == &expected
                ))) { row.pass(action, "native-local-line-exact-mode-and-text"); break; }
                if tokio::time::Instant::now() >= deadline {
                    return Err(format!("{action} accepted but local line absent").into());
                }
                client.service_liveness(Duration::from_millis(20)).await?;
                client.take_events();
            }
        }
    }
    for index in 0..32 {
        let result = client
            .chat(&ChatIntent::Say {
                mode: ChatSpeechMode::Say,
                text: format!("E2EF4 mute {index}"),
            })
            .await?;
        if result.disposition == ChatDisposition::Muted && result.wait_seconds > 0 {
            row.pass("mute", "native-spam-gate-muted-with-positive-wait");
            break;
        }
    }
    Err("say/whisper/yell-recipient-range-and-level-premium-matrix assertions unfinished;source:crates/gameplay/dispatch/src/gameplay_transport/chat_intent.rs:625".into())
}
