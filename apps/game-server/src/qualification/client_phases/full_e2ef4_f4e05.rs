//! E05: hunted gold, depot-backed sell offers and two-player durable settlement.
use super::*;
use oteryn_dev_client::{client_runtime::market as runtime, client_ui::market as ui};
use oteryn_protocol_oteryn::{depot, depot_search, market::MarketRow};

// The original MP mount is private inside world_journey; reuse its test-free bot helpers
// here without changing that mount. E05 uses no MP combat/party helpers or profiles.
#[allow(dead_code, clippy::duplicate_mod)]
#[path = "mp/bots.rs"]
mod bots;
// The bot helpers require these same test-free diagnostics in their parent scope.
#[allow(clippy::duplicate_mod)]
#[path = "mp/diagnostics.rs"]
mod diagnostics;

const WARE: u32 = 3267; // A plain dagger, sold by Sam; never a fixture-minted item.
const PRICE: u64 = 1;

/// Like mp/bots.rs::with_liveness (private to that module), poll the player's step to
/// completion while the other player services its socket. Never cancel a half-read frame.
async fn with_liveness<T>(
    idle: &mut DevClientSession,
    step: impl std::future::Future<Output = TestResult<T>>,
) -> TestResult<T> {
    let mut step = std::pin::pin!(step);
    let mut tick = tokio::time::interval(Duration::from_millis(250));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            biased;
            result = &mut step => return result,
            _ = tick.tick() => {
                idle.service_liveness(Duration::from_millis(20)).await?;
                idle.take_events();
            }
        }
    }
}

/// Avoid the other player when choosing a locker approach and its walking path.
async fn locker(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let occupied = client
        .world_entities()
        .ok_or("E05 entity view missing")?
        .iter()
        .filter(|entity| entity.kind == oteryn_dev_client::EntityKind::Player)
        .map(|entity| tile(entity.position))
        .collect::<TestResult<BTreeSet<_>>>()?;
    let mut queue = VecDeque::from([from]);
    let mut parents = BTreeMap::from([(from, None)]);
    while let Some(at) = queue.pop_front() {
        if near_locker(world, at) {
            let mut path = Vec::new();
            let mut cursor = at;
            while let Some(Some((previous, direction))) = parents.get(&cursor) {
                path.push(*direction);
                cursor = *previous;
            }
            path.reverse();
            return walk(client, world, &path).await;
        }
        if parents.len() % 4096 == 0 {
            progress();
        }
        for direction in [
            StepDirection::North,
            StepDirection::East,
            StepDirection::South,
            StepDirection::West,
        ] {
            let Some(next) = destination(world, at, direction) else {
                continue;
            };
            if next.x.abs_diff(from.x) > 160
                || next.y.abs_diff(from.y) > 160
                || next.floor.abs_diff(from.floor) > 2
                || occupied.contains(&next)
                || parents.contains_key(&next)
            {
                continue;
            }
            parents.insert(next, Some((at, direction)));
            queue.push_back(next);
        }
    }
    Err("E05 no unoccupied authored locker approach".into())
}

async fn command(
    client: &mut DevClientSession,
    panel: &mut ui::MarketPanel,
    intent: ui::Intent,
) -> TestResult<oteryn_protocol_oteryn::market::MarketReply> {
    progress();
    let request =
        ui::request(panel, intent).ok_or_else(|| format!("E05 {intent:?}: UI input refused"))?;
    let response = runtime::send(client.session_mut(), &request).await?;
    if !ui::apply_result(panel, &response) {
        return Err(format!("E05 {intent:?}: native market refused: {response:?}").into());
    }
    response
        .reply
        .ok_or_else(|| "E05 market receipt missing".into())
}

fn panel(definition: u32) -> ui::MarketPanel {
    ui::MarketPanel {
        selected: true,
        definition,
        amount: 1,
        price: PRICE,
        sell: true,
        ..Default::default()
    }
}

/// Depot Search is the player's view of depot, stash and Inbox custody, including
/// market returns/deliveries. A missing result is zero only for an explicit NotFound.
async fn holdings(client: &mut DevClientSession) -> TestResult<[u32; 3]> {
    use oteryn_dev_client::client_ui::depot_search as search;
    let mut panel = search::DepotSearchPanel {
        selected: true,
        item_id: WARE,
        ..Default::default()
    };
    let command = search::search(&mut panel).ok_or("E05 depot search UI input")?;
    let response = client
        .session_mut()
        .gated_command(command.capability, command.command_type, &command.payload)
        .await?;
    search::apply_result(&mut panel, &response);
    let reply = panel
        .reply
        .ok_or_else(|| format!("E05 depot search refused: {:?}", panel.feedback))?;
    match (reply.outcome, reply.results.as_slice()) {
        (depot_search::Outcome::NotFound, []) => Ok([0; 3]),
        (depot_search::Outcome::Found, [row]) if row.client_id == WARE => {
            Ok([row.depot, row.stash, row.inbox])
        }
        _ => Err(format!("E05 depot search unexpected: {reply:?}").into()),
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Settlement {
    inventory: BTreeMap<u32, u64>,
    holdings: [u32; 3],
    bank: u64,
}

async fn snapshot(
    row: &mut Row,
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
) -> TestResult<Settlement> {
    let (_, _, bank) = find_banker(row, client, world, npcs).await?;
    locker(client, world).await?;
    Ok(Settlement {
        inventory: inventory_units(client)?,
        holdings: holdings(client).await?,
        bank,
    })
}

async fn verify(
    row: &mut Row,
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    expected: &Settlement,
    step: &str,
) -> TestResult {
    let actual = snapshot(row, client, world, npcs).await?;
    if &actual != expected {
        return Err(format!("E05 {step}: expected={expected:?}; observed={actual:?}").into());
    }
    Ok(())
}

async fn deposit_gold(
    row: &mut Row,
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
) -> TestResult {
    let (_, actor, balance) = find_banker(row, client, world, npcs).await?;
    let refs = coin_refs()?;
    let before = inventory_units(client)?;
    let gold = wealth(&before, &refs);
    if gold == 0 {
        return Err("E05 hunting/purchase left no gold to deposit".into());
    }
    bank_exchange(
        client,
        actor,
        "deposit all",
        &format!("Do you want to deposit {gold} gold?"),
        &format!("You have deposited {gold} gold."),
    )
    .await?;
    let expected = non_coin(&before, &refs);
    wait_inventory(client, &expected).await?;
    let actual = bank_balance(client, actor).await?;
    if actual != balance.checked_add(gold).ok_or("E05 bank overflow")? {
        return Err(format!("E05 deposit balance: {balance}+{gold}, observed={actual}").into());
    }
    Ok(())
}

async fn own_offer(
    client: &mut DevClientSession,
    panel: &mut ui::MarketPanel,
    expected: &MarketRow,
    present: bool,
) -> TestResult {
    command(client, panel, ui::Intent::OwnOffers).await?;
    let matches: Vec<_> = panel
        .rows
        .iter()
        .filter(|row| row.offer == expected.offer)
        .collect();
    if (present && matches.as_slice() != [expected]) || (!present && !matches.is_empty()) {
        return Err(format!(
            "E05 own offers expected present={present} row={expected:?}; observed={matches:?}"
        )
        .into());
    }
    Ok(())
}

async fn history(
    client: &mut DevClientSession,
    panel: &mut ui::MarketPanel,
    required: &[MarketRow],
    forbidden: Option<[u8; 16]>,
) -> TestResult {
    command(client, panel, ui::Intent::History).await?;
    let mut observed = Vec::new();
    loop {
        observed.extend_from_slice(&panel.rows);
        if panel.rows.len() < 32 || panel.page == 18 {
            break;
        }
        command(client, panel, ui::Intent::NextPage).await?;
    }
    for expected in required {
        let matches: Vec<_> = observed
            .iter()
            .filter(|row| row.offer == expected.offer)
            .collect();
        if matches.as_slice() != [expected] {
            return Err(
                format!("E05 history expected exactly {expected:?}; observed={matches:?}").into(),
            );
        }
    }
    if forbidden.is_some_and(|id| observed.iter().any(|row| row.offer == id)) {
        return Err("E05 buyer history exposed seller's cancelled offer".into());
    }
    Ok(())
}

async fn journey(
    row: &mut Row,
    seller: &mut DevClientSession,
    buyer: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    definition: u32,
    fee: u64,
) -> TestResult<(Settlement, Settlement, Vec<MarketRow>, MarketRow)> {
    let mut seller_before = with_liveness(buyer, snapshot(row, seller, world, npcs)).await?;
    let mut buyer_before = with_liveness(seller, snapshot(row, buyer, world, npcs)).await?;
    let mut maker = panel(definition);
    let mut taker = panel(definition);
    with_liveness(buyer, command(seller, &mut maker, ui::Intent::Browse)).await?;
    row.pass("browse", "native-book-for-purchased-marketable-ware");
    let cancelled = with_liveness(buyer, command(seller, &mut maker, ui::Intent::CreateOffer))
        .await?
        .offer;
    let open = MarketRow {
        offer: cancelled,
        definition,
        side: 2,
        amount: 1,
        price: PRICE,
        state: 1,
        anonymous: false,
    };
    with_liveness(buyer, own_offer(seller, &mut maker, &open, true)).await?;
    seller_before.bank = seller_before
        .bank
        .checked_sub(fee)
        .ok_or("E05 first fee exceeds bank")?;
    seller_before.holdings[0] = seller_before.holdings[0]
        .checked_sub(1)
        .ok_or("E05 first escrow exceeds depot")?;
    with_liveness(
        buyer,
        verify(
            row,
            seller,
            world,
            npcs,
            &seller_before,
            "create-fee-and-escrow",
        ),
    )
    .await?;
    row.pass(
        "create",
        "sell-offer-exact-id-ware-quantity-price-fee-and-depot-escrow",
    );

    maker.selected_offer = Some(cancelled);
    let receipt =
        with_liveness(buyer, command(seller, &mut maker, ui::Intent::CancelOffer)).await?;
    if receipt.offer != cancelled {
        return Err("E05 cancel receipt names another offer".into());
    }
    with_liveness(buyer, own_offer(seller, &mut maker, &open, false)).await?;
    seller_before.holdings[2] = seller_before.holdings[2]
        .checked_add(1)
        .ok_or("E05 cancellation inbox overflow")?;
    with_liveness(
        buyer,
        verify(
            row,
            seller,
            world,
            npcs,
            &seller_before,
            "cancel-inbox-return-fee-retained",
        ),
    )
    .await?;
    row.pass("cancel", "offer-absent-item-returned-to-inbox-fee-retained");

    let accepted = with_liveness(buyer, command(seller, &mut maker, ui::Intent::CreateOffer))
        .await?
        .offer;
    if accepted == cancelled {
        return Err("E05 second create reused cancelled offer id".into());
    }
    let accepted_open = MarketRow {
        offer: accepted,
        ..open.clone()
    };
    with_liveness(buyer, own_offer(seller, &mut maker, &accepted_open, true)).await?;
    seller_before.bank = seller_before
        .bank
        .checked_sub(fee)
        .ok_or("E05 second fee exceeds bank")?;
    seller_before.holdings[0] = seller_before.holdings[0]
        .checked_sub(1)
        .ok_or("E05 second escrow exceeds depot")?;
    with_liveness(
        buyer,
        verify(row, seller, world, npcs, &seller_before, "second-create"),
    )
    .await?;
    with_liveness(seller, command(buyer, &mut taker, ui::Intent::Browse)).await?;
    if taker
        .rows
        .iter()
        .filter(|row| row.offer == accepted)
        .collect::<Vec<_>>()
        .as_slice()
        != [&accepted_open]
    {
        return Err("E05 buyer book lacks exact seller offer".into());
    }
    taker.selected_offer = Some(accepted);
    let receipt =
        with_liveness(seller, command(buyer, &mut taker, ui::Intent::AcceptOffer)).await?;
    if receipt.offer != accepted {
        return Err("E05 accept receipt names another offer".into());
    }
    seller_before.bank = seller_before
        .bank
        .checked_add(PRICE)
        .ok_or("E05 sale bank overflow")?;
    buyer_before.bank = buyer_before
        .bank
        .checked_sub(PRICE)
        .ok_or("E05 purchase exceeds bank")?;
    buyer_before.holdings[2] = buyer_before.holdings[2]
        .checked_add(1)
        .ok_or("E05 purchase inbox overflow")?;
    with_liveness(buyer, own_offer(seller, &mut maker, &accepted_open, false)).await?;
    with_liveness(
        buyer,
        verify(
            row,
            seller,
            world,
            npcs,
            &seller_before,
            "seller-settlement",
        ),
    )
    .await?;
    with_liveness(
        seller,
        verify(row, buyer, world, npcs, &buyer_before, "buyer-settlement"),
    )
    .await?;
    row.pass(
        "accept",
        "real-buyer-debited-seller-credited-exact-item-delivered-to-buyer-inbox",
    );

    let cancelled_history = MarketRow { state: 2, ..open };
    let accepted_history = MarketRow {
        state: 2,
        ..accepted_open
    };
    let maker_history = vec![cancelled_history, accepted_history.clone()];
    with_liveness(buyer, history(seller, &mut maker, &maker_history, None)).await?;
    with_liveness(
        seller,
        history(
            buyer,
            &mut taker,
            std::slice::from_ref(&accepted_history),
            Some(cancelled),
        ),
    )
    .await?;
    row.pass(
        "history",
        "seller-cancel-and-fill-buyer-fill-exact-offer-ware-side-quantity-price-state",
    );
    Ok((seller_before, buyer_before, maker_history, accepted_history))
}

async fn run_inner(
    row: &mut Row,
    seller: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let fee = crate::durability::market::placement_fee(1, PRICE)
        .map_err(|e| format!("E05 placement fee: {e:?}"))?;
    let definition = provisioning::item_reference(&provisioning::item_index()?, WARE)?.get();
    let needed = full_gold::price(WARE, 2)?
        .checked_add(fee.checked_mul(2).ok_or("E05 fees overflow")?)
        .ok_or("E05 funding overflow")?;
    // No other player is connected yet. Heartbeat even while an underlying command waits.
    let preparation = async {
        full_gold::earn(seller, world, &metadata.spawns, "E05-seller", needed).await?;
        full_gold::buy(seller, world, &metadata.npcs, WARE, 2).await?;
        deposit_gold(row, seller, world, &metadata.npcs).await?;
        locker(seller, world).await?;
        let town = depot_call(seller, None, depot::DepotAction::Browse)
            .await?
            .town
            .ok_or("E05 depot town missing")?;
        let before = inventory_units(seller)?;
        let mut expected = before.clone();
        let count = expected
            .get_mut(&definition)
            .ok_or("E05 purchased daggers absent")?;
        *count = count
            .checked_sub(2)
            .ok_or("E05 purchased fewer than two daggers")?;
        if *count == 0 {
            expected.remove(&definition);
        }
        let mut current = before;
        for _ in 0..2 {
            let item = seller
                .inventory()
                .and_then(|view| {
                    view.entries.iter().find(|item| {
                        item.item_definition_ref.get() == definition && item.count.get() == 1
                    })
                })
                .copied()
                .ok_or("E05 plain single dagger absent")?;
            let moved =
                depot_call(seller, Some(town), depot::DepotAction::Deposit(item.handle)).await?;
            if moved.outcome != depot::DepotOutcome::Moved {
                return Err(format!("E05 deposit dagger refused: {moved:?}").into());
            }
            let count = current
                .get_mut(&definition)
                .ok_or("E05 deposited dagger not carried")?;
            *count = count
                .checked_sub(1)
                .ok_or("E05 deposit quantity underflow")?;
            if *count == 0 {
                current.remove(&definition);
            }
            wait_inventory(seller, &current).await?;
        }
        if current != expected {
            return Err("E05 depot preparation quantity mismatch".into());
        }
        TestResult::Ok(())
    };
    // Every awaited phase, including setup and cleanup, ticks the harness at least every 250ms.
    heartbeat(preparation).await?;
    let (mut peers, connected) = with_liveness(seller, async {
        let (peers, result) = bots::connect_bots(&[(clients, 0, None)]).await;
        Ok((peers, result))
    })
    .await?;
    let result = async {
        connected?;
        let bot = peers.first_mut().ok_or("E05 second bot absent")?;
        let buyer = bot.client()?;
        let starting_gold = full_gold::coin_value(buyer)?;
        with_liveness(seller, async {
            full_gold::earn(
                buyer,
                world,
                &metadata.spawns,
                "E05-buyer",
                starting_gold
                    .checked_add(PRICE)
                    .ok_or("E05 hunting target overflow")?,
            )
            .await?;
            deposit_gold(row, buyer, world, &metadata.npcs).await?;
            locker(buyer, world).await
        })
        .await?;
        let (seller_after, buyer_after, maker_history, buyer_history) =
            journey(row, seller, buyer, world, &metadata.npcs, definition, fee).await?;
        let buyer_session = buyer.join_snapshot().game_session_id;
        with_liveness(seller, bots::relog_all(&mut peers)).await?;
        let buyer = peers.first_mut().ok_or("E05 relog bot absent")?.client()?;
        if buyer.join_snapshot().game_session_id == buyer_session {
            return Err("E05 buyer relog reused session".into());
        }
        with_liveness(buyer, relog(seller, clients)).await?;
        with_liveness(
            buyer,
            verify(
                row,
                seller,
                world,
                &metadata.npcs,
                &seller_after,
                "seller-relog",
            ),
        )
        .await?;
        with_liveness(
            seller,
            verify(
                row,
                buyer,
                world,
                &metadata.npcs,
                &buyer_after,
                "buyer-relog",
            ),
        )
        .await?;
        let mut maker = panel(definition);
        let mut taker = panel(definition);
        with_liveness(buyer, history(seller, &mut maker, &maker_history, None)).await?;
        with_liveness(
            seller,
            history(
                buyer,
                &mut taker,
                std::slice::from_ref(&buyer_history),
                Some(maker_history[0].offer),
            ),
        )
        .await?;
        for ended in &maker_history {
            with_liveness(buyer, own_offer(seller, &mut maker, ended, false)).await?;
        }
        row.pass(
            "relog-settlement",
            "both-fresh-sessions-exact-inventories-bank-depot-inbox-history-no-reopened-offers",
        );
        TestResult::Ok(())
    }
    .await;
    let cleanup = with_liveness(seller, async {
        for bot in &mut peers {
            if let Some(peer) = bot.session.as_mut() {
                let (outcome, detail, attempts, busy) =
                    super::super::logout_retrying_busy(peer).await?;
                if outcome != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
                    return Err(format!(
                        "E05 buyer cleanup: {detail}; attempts={attempts}; busy_timed_out={busy}"
                    )
                    .into());
                }
                bot.session = None;
            }
        }
        Ok(())
    })
    .await;
    match (result, cleanup) {
        (Err(error), Err(cleanup)) => Err(format!("{error}; cleanup={cleanup}").into()),
        (Err(error), _) | (_, Err(error)) => Err(error),
        (Ok(()), Ok(())) => Ok(()),
    }
}

async fn heartbeat<T>(step: impl std::future::Future<Output = TestResult<T>>) -> TestResult<T> {
    let mut step = std::pin::pin!(step);
    let mut tick = tokio::time::interval(Duration::from_millis(250));
    loop {
        tokio::select! {
            biased;
            result = &mut step => return result,
            _ = tick.tick() => progress(),
        }
    }
}

pub(super) async fn run(
    row: &mut Row,
    seller: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    heartbeat(run_inner(row, seller, clients, world)).await
}
