//! E2EF4: real-world economy/social probes, with fail-closed full-row evidence.
//! No fixture provisioning, SQL mutations, clock overrides or invented wire handles.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityRef};
use std::collections::BTreeSet;

struct Row {
    id: &'static str,
    required: &'static [&'static str],
    passed: BTreeSet<&'static str>,
    started: std::time::Instant,
    failures: Vec<String>,
    missing_server: Option<&'static str>,
}

impl Row {
    fn new(id: &'static str, required: &'static [&'static str]) -> Self {
        Self {
            id,
            required,
            passed: BTreeSet::new(),
            started: std::time::Instant::now(),
            failures: Vec::new(),
            missing_server: None,
        }
    }

    fn pass(&mut self, action: &'static str, observation: &str) {
        if !self.required.contains(&action) || !self.passed.insert(action) {
            self.failures
                .push(format!("duplicate-or-unknown-action:{action}"));
            return;
        }
        evidence(&format!(
            "E2EF4_CHECK {} {action} PASS {observation}",
            self.id
        ));
    }

    fn finish(mut self, result: TestResult) {
        if let Err(error) = result {
            self.failures.push(error.to_string());
        }
        for action in self.required {
            if !self.passed.contains(action) {
                evidence(&format!(
                    "E2EF4_CHECK {} {action} FAIL assertion-not-completed",
                    self.id
                ));
            }
        }
        let complete = self.passed.len() == self.required.len() && self.failures.is_empty();
        // Detail is separate from the report's single-token evidence field. A failure is
        // never turned into MISSING-SERVER merely because this journey is unfinished.
        for failure in &self.failures {
            evidence(&format!("E2EF4_GAP {} {failure}", self.id));
        }
        evidence(&format!(
            "e2e_all scenario={}.full status={} seconds={:.3} evidence={}",
            self.id,
            if complete {
                "PASS"
            } else if self.missing_server.is_some() {
                "MISSING-SERVER"
            } else {
                "FAIL"
            },
            self.started.elapsed().as_secs_f64(),
            self.missing_server.unwrap_or(
                "source:apps/game-server/src/qualification/client_phases/full_e2ef4.rs:1"
            ),
        ));
    }
}

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
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
) {
    // Each row reports even when another row has missing content or a refused command.
    // Socket waits are bounded; a timed-out session is not evidence of server absence.
    macro_rules! row {
        ($id:literal, $actions:ident, $journey:ident $(, $arg:expr)*) => {{
            let mut row = Row::new($id, $actions);
            let result = tokio::time::timeout(
                Duration::from_secs(90),
                $journey(&mut row, client $(, $arg)*),
            ).await;
            row.finish(match result {
                Ok(result) => result,
                Err(_) => Err("native-journey-deadline;remaining-assertions-unproven".into()),
            });
        }};
    }
    row!("E01", E01, shop, clients, world, npcs);
    row!("E03", E03, coins);
    row!("E04", E04, trade, clients, world);
    row!("E05", E05, market, world);
    row!("E07", E07, storage, clients, world);
    row!("S01", S01, party, clients, world);
    row!("S03", S03, guild);
    row!("S09", S09, logout, clients, world);
    row!("S18", S18, speech);
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
    let logout = client.session_mut().logout().await?;
    if logout.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err(format!("durability-check logout refused: {logout:?}").into());
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

async fn shop(
    row: &mut Row,
    client: &mut DevClientSession,
    _clients: &SeamClients<'_>,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
) -> TestResult {
    use oteryn_dev_client::client_runtime::npc_shop as runtime;
    use oteryn_protocol_oteryn::{npc_service as wire, npc_shop as shop};
    let npc = npcs
        .npcs
        .iter()
        .find(|npc| npc.key == "oteryn:npc.sam")
        .ok_or("content:oteryn:npc.sam absent")?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        npc.placements
            .iter()
            .any(|p| p.floor == at.floor && at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)) <= 3)
    })
    .ok_or("content:oteryn:npc.sam unreachable")?;
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
        .map(|entity| entity.entity)
        .ok_or("Sam actor not visible at authored placement")?;
    for text in ["hi", "trade"] {
        let result = runtime::talk(
            client.session_mut(),
            &wire::NpcTalkIntent {
                npc_actor: actor,
                text: text.into(),
            },
        )
        .await?;
        if result != wire::NpcIntentDisposition::Ok {
            return Err(format!("Sam {text}: {result:?}").into());
        }
        if text == "hi" {
            let delta = client
                .session_mut()
                .read_gated_delta(wire::STATE_DOMAIN_NPC_CONVERSATION, 1)
                .await?;
            let conversation = wire::decode_npc_conversation(&delta.payload)
                .map_err(|e| format!("NPC conversation: {e:?}"))?;
            if conversation.open.is_none() {
                return Err("greeting did not open conversation".into());
            }
            row.pass("greet", "native-conversation-open");
        }
    }
    let delta = client
        .session_mut()
        .read_gated_delta(wire::STATE_DOMAIN_NPC_TRADE_WINDOW, 1)
        .await?;
    let window =
        wire::decode_npc_trade_window(&delta.payload).map_err(|e| format!("shop window: {e:?}"))?;
    let (index, offer) = window
        .offers
        .iter()
        .enumerate()
        .filter(|(_, o)| o.side == wire::NpcTradeSide::Buy)
        .min_by_key(|(_, o)| o.unit_price)
        .ok_or("Sam has no buy offer")?;
    let result = runtime::send(
        client.session_mut(),
        &runtime::NpcShopCommand(shop::NpcShopIntent {
            npc_actor: actor,
            catalogue_revision: window.open.ok_or("trade window closed")?.1,
            offer_index: u32::try_from(index)?,
            side: offer.side,
            action: shop::NpcShopAction::Look,
            expected_unit_price: offer.unit_price,
        }),
    )
    .await?;
    if !result
        .result
        .is_some_and(|r| r.disposition == wire::NpcIntentDisposition::Ok && !r.look_text.is_empty())
    {
        return Err("shop look did not return a description".into());
    }
    row.pass("look", "native-offer-description-nonempty");
    let result = runtime::buy(
        client.session_mut(),
        &wire::NpcTradeIntent {
            npc_actor: actor,
            catalogue_revision: window.open.ok_or("trade window closed")?.1,
            offer_index: u32::try_from(index)?,
            side: offer.side,
            quantity: 1,
            expected_unit_price: offer.unit_price,
        },
    )
    .await?;
    // A receipt alone cannot prove inventory/bank custody or qualify buy/sell.
    Err(format!("buy-disposition={result:?};sell/sell-all/bank/relog-assertions-unimplemented;source:crates/gameplay/dispatch/src/gameplay_transport/npc_trade_command.rs:1").into())
}

async fn coins(_row: &mut Row, client: &mut DevClientSession) -> TestResult {
    let definitions = provisioning::item_index()?;
    let mut observed = Vec::new();
    for id in [3031, 3035, 3043, 23722, 25719, 28525, 28526] {
        let reference = provisioning::item_reference(&definitions, id)?;
        let item = client
            .inventory()
            .and_then(|inventory| {
                inventory
                    .entries
                    .iter()
                    .find(|i| i.item_definition_ref == reference)
            })
            .copied();
        if let Some(item) = item {
            let result = client.use_item(item.handle).await?;
            observed.push(format!(
                "{id}:{}:{:?}",
                item.count.get(),
                result.disposition
            ));
        } else {
            observed.push(format!("{id}:absent"));
        }
    }
    Err(format!("coin-use-observations={};conversion-quantities/charges/tick/retirement/relog-unproven;source:crates/gameplay/dispatch/src/gameplay_transport/coin_use.rs:42", observed.join(",")).into())
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
        Err("accept/logout-race/relog-custody assertions unfinished".into())
    }
    .await;
    let cleanup = peer.session_mut().logout().await?;
    if cleanup.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("trade peer cleanup logout refused: {cleanup:?}").into());
    }
    result
}

async fn market(row: &mut Row, client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
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
    Err("cross-player-accept/history/relog-settlement assertions unfinished".into())
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
    let cleanup = peer.session_mut().logout().await?;
    if cleanup.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("party peer cleanup logout refused: {cleanup:?}").into());
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
        let cleanup = peer.session_mut().logout().await?;
        return Err(format!("logout peer is not on authored PZ tile: cleanup={cleanup:?}").into());
    }
    let session = peer.join_snapshot().game_session_id;
    let logout = peer.session_mut().logout().await?;
    if logout.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err(format!("PZ logout refused: {logout:?}").into());
    }
    row.pass("pz-logout", "authored-PZ-native-logout-accepted");
    drop(peer);
    let mut peer = connect_character(clients, 0x84, 0).await?;
    let retained = peer.world_spatial().actor_position == position
        && peer.join_snapshot().game_session_id != session;
    let cleanup = peer.session_mut().logout().await?;
    if !retained || cleanup.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("fresh-login-position-or-cleanup-mismatch: retained={retained}; cleanup={cleanup:?}").into());
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
