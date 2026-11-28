//! G09: pocket watch USE shows the world time (server package SRVPKG1, `novelty_item_use.rs`,
//! Crystal `watch.lua`).
//! 1. Admit the dedicated character, walk to Lubo and buy one watch (item 2906).
//! 2. USE it by handle: expect an accepted `Committed` reply and a private "Game" chat line
//!    `The time is H:MM.` published by the server matching the World clock during the command.
//! 3. The watch is not consumed: the carried count is unchanged.
//! 4. Log out, freshly admit and use again: require a new line with the advanced World time.
//!
//! The fixture adds 19 gold toward Crystal Lubo's 20-gold price. The purchase
//! leaves one fewer coin than the character's observed starter balance.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::{
    chat::ChatLine,
    item_view::CharacterInventory,
    logout::LogoutResult,
    npc_service as npc,
    world_object::{self, UseDisposition, UseIntent, UseTarget},
    world_spatial_entities::{EntityKind, EntityRef, WorldSpatialEntity},
};
const GOLD: u32 = 3031;
const WATCH: u32 = 2906;
const PRICE: u32 = 20;
const PURCHASE_FUNDING: u32 = PRICE - 1;
const LATE_CLOCK_PHASE_MS: std::ops::RangeInclusive<u64> = 2000..=2100;
const EARLY_CLOCK_PHASE_MS: std::ops::RangeInclusive<u64> = 50..=250;
// Pinned Lubo behavior in content/behaviors/behaviors-02500-02999.json.
const LUBO_WANDER_RADIUS: u16 = 2;
// Walkable southeast approach in the pinned Thais/Lubo bundle.
const LUBO_APPROACH: TilePos = TilePos {
    x: 32492,
    y: 32122,
    floor: -7,
};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    // The cardinal walk from Thais to Lubo may take several minutes at the
    // server's ordinary step pace. Each step still has to commit on the first send.
    let result =
        verdict::indexed_timeout(Duration::from_secs(600), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = match result {
        Ok(Ok(())) => (
            "PASS",
            "lubo-purchase-watch-time-line-and-not-consumed".to_owned(),
        ),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G09.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

/// Minute of day from a `The time is H:MM.` line, or `None` for any other text.
fn time_line_minute(text: &str) -> Option<u32> {
    let clock = text.strip_prefix("The time is ")?.strip_suffix('.')?;
    let (hours, minutes) = clock.split_once(':')?;
    if minutes.len() != 2
        || !minutes.bytes().all(|byte| byte.is_ascii_digit())
        || !hours.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let (hours, minutes): (u32, u32) = (hours.parse().ok()?, minutes.parse().ok()?);
    (hours < 24 && minutes < 60 && clock == format!("{hours}:{minutes:02}"))
        .then_some(hours * 60 + minutes)
}

// Global time advances one minute per 2.5 real seconds. The wire clock
// uses the Unix epoch as its origin.
const GLOBAL_MINUTE_MS: u64 = 2_500;
const GLOBAL_DAY_MINUTES: u64 = 1_440;

fn clock_elapsed_ms() -> TestResult<u64> {
    Ok(u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis(),
    )?)
}

fn validate_published_minute(minute: u16, from_ms: u64, through_ms: u64) -> TestResult {
    let first = from_ms / GLOBAL_MINUTE_MS;
    let last = through_ms / GLOBAL_MINUTE_MS;
    if !(first..=last).any(|tick| tick % GLOBAL_DAY_MINUTES == u64::from(minute)) {
        return Err(format!(
            "step=clock observed=published-minute:{minute} expected=ticks:{first}..={last}"
        )
        .into());
    }
    Ok(())
}

fn minute_in_command(minute: u32, first: u64, last: u64) -> bool {
    first <= last
        && [first, last]
            .iter()
            .any(|tick| tick % GLOBAL_DAY_MINUTES == u64::from(minute))
}

fn carried(
    client: &DevClientSession,
    definition: std::num::NonZeroU32,
) -> Vec<(oteryn_dev_client::ItemHandle, u32)> {
    client
        .inventory()
        .map_or_else(Vec::new, |inventory| carried_in(inventory, definition))
}

fn carried_in(
    inventory: &CharacterInventory,
    definition: std::num::NonZeroU32,
) -> Vec<(oteryn_dev_client::ItemHandle, u32)> {
    inventory
        .main_backpack
        .iter()
        .chain(&inventory.entries)
        .filter(|entry| entry.item_definition_ref == definition)
        .map(|entry| (entry.handle, entry.count.get()))
        .collect()
}

async fn use_target(
    client: &mut DevClientSession,
    target: UseTarget,
) -> TestResult<(CommandStatus, UseDisposition, u64)> {
    let payload = world_object::encode_item_use_intent(&UseIntent {
        target,
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(|error| format!("step=encode observed={error:?}"))?;
    let reply = client.session_mut().world_context_use(&payload).await?;
    let disposition = world_object::decode_use_result(&reply.payload)
        .map_err(|error| format!("step=use observed=reply-decode:{error:?}"))?;
    Ok((reply.status, disposition, reply.server_sequence))
}

#[derive(Default)]
struct NpcEvidence {
    greeting: Option<npc::NpcConversation>,
    window: Option<npc::NpcTradeWindow>,
}

impl NpcEvidence {
    fn check(&self, event: &SessionEvent) -> TestResult {
        let consistent = match event {
            SessionEvent::NpcConversation(delta) => self.greeting.as_ref() == Some(&delta.value),
            SessionEvent::NpcTradeWindow(delta) => self.window.as_ref() == Some(&delta.value),
            SessionEvent::GatedSnapshotReplaced(delta) => !matches!(
                delta.value.domain_id,
                npc::STATE_DOMAIN_NPC_CONVERSATION | npc::STATE_DOMAIN_NPC_TRADE_WINDOW
            ),
            _ => true,
        };
        if !consistent {
            return Err(format!(
                "step=npc observed=conflicting-update domain={}",
                event.domain_id()
            )
            .into());
        }
        Ok(())
    }
}

fn fixed_position(position: ActorPosition) -> TestResult {
    if tile(position)? != LUBO_APPROACH {
        return Err(
            format!("step=position observed={position:?} expected={LUBO_APPROACH:?}").into(),
        );
    }
    Ok(())
}

fn check_position_event(event: &SessionEvent) -> TestResult {
    match event {
        SessionEvent::WorldSpatial(delta) => fixed_position(delta.value.actor_position),
        SessionEvent::WorldSpatialEntities(delta) => fixed_position(delta.value.actor_position),
        _ => Ok(()),
    }
}

async fn read_time(
    client: &mut DevClientSession,
    handle: oteryn_dev_client::ItemHandle,
    before: &CharacterInventory,
    phase: std::ops::RangeInclusive<u64>,
    npc: &NpcEvidence,
    mut clock_from: u64,
) -> TestResult<u32> {
    let minute_ms = GLOBAL_MINUTE_MS;
    let mut minute = None;
    let phase_deadline = tokio::time::Instant::now() + Duration::from_millis(2 * minute_ms);
    let (first, phase_ms) = loop {
        observe_use(
            client,
            before,
            npc,
            &mut clock_from,
            &mut minute,
            u64::MAX,
            0,
            0,
        )?;
        let elapsed = clock_elapsed_ms()?;
        if phase.contains(&(elapsed % minute_ms)) {
            break (elapsed / minute_ms, elapsed % minute_ms);
        }
        if tokio::time::Instant::now() >= phase_deadline {
            return Err("step=clock observed=missed-sample-phase".into());
        }
        client.service_liveness(Duration::from_millis(10)).await?;
    };
    let (status, disposition, reply) = use_target(client, UseTarget::Item(handle)).await?;
    let last = clock_elapsed_ms()? / minute_ms;
    if status != CommandStatus::Accepted || disposition != UseDisposition::Committed {
        return Err(
            format!("step=use observed=status:{status:?},disposition:{disposition:?}").into(),
        );
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while tokio::time::Instant::now() < deadline {
        observe_use(
            client,
            before,
            npc,
            &mut clock_from,
            &mut minute,
            reply,
            first,
            last,
        )?;
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    observe_use(
        client,
        before,
        npc,
        &mut clock_from,
        &mut minute,
        reply,
        first,
        last,
    )?;
    let observed = minute.ok_or("step=chat observed=no-server-time-line")?;
    logout(client).await?;
    observe_use(
        client,
        before,
        npc,
        &mut clock_from,
        &mut minute,
        reply,
        first,
        last,
    )?;
    evidence(&format!(
        "G09_CHECK step=clock status=PASS minute={observed} phase_ms={phase_ms} reply_sequence={reply}"
    ));
    Ok(observed)
}

fn observe_use(
    client: &mut DevClientSession,
    before: &CharacterInventory,
    npc: &NpcEvidence,
    clock_from: &mut u64,
    minute: &mut Option<u32>,
    reply: u64,
    first: u64,
    last: u64,
) -> TestResult {
    let through = clock_elapsed_ms()?;
    fixed_position(client.world_spatial().actor_position)?;
    for event in client.take_events() {
        npc.check(&event)?;
        check_position_event(&event)?;
        match event {
            SessionEvent::WorldClock(delta) => {
                validate_published_minute(delta.value.tibian_minute, *clock_from, through)?
            }

            SessionEvent::ChatLine(delta) => {
                if let Some(text) = private_clock_text(&delta.value)? {
                    check_clock_line(minute, text, delta.server_sequence, reply, first, last)?;
                }
            }
            SessionEvent::ChatSnapshot(delta) => {
                for line in delta.value.lines() {
                    if private_clock_text(line)?.is_some() {
                        return Err("step=chat observed=clock-in-replacement-snapshot".into());
                    }
                }
            }
            SessionEvent::Inventory(delta) => require_unchanged_inventory(before, &delta.value)?,
            _ => {}
        }
    }
    *clock_from = through;
    if client.inventory() != Some(before) {
        return Err("step=use observed=inventory-changed-after-exchange".into());
    }
    Ok(())
}

fn private_clock_text(line: &ChatLine) -> TestResult<Option<&str>> {
    let (text, private_game) = match line {
        ChatLine::Private { speaker_name, text } => (text, speaker_name == "Game"),
        ChatLine::Local { text, .. }
        | ChatLine::Room { text, .. }
        | ChatLine::Party { text, .. }
        | ChatLine::System { text } => (text, false),
        ChatLine::Guild(line) => (&line.text, false),
        ChatLine::PrivateChannel(line) => (&line.text, false),
        ChatLine::Dropped => return Err("step=chat observed=dropped-lines".into()),
    };
    if !text.starts_with("The time is") {
        return Ok(None);
    }
    if !private_game {
        return Err("step=chat observed=clock-outside-private-game-line".into());
    }
    Ok(Some(text))
}

fn require_unchanged_inventory(
    before: &CharacterInventory,
    observed: &CharacterInventory,
) -> TestResult {
    if before != observed {
        return Err("step=inventory observed=unexpected-intermediate-state".into());
    }
    Ok(())
}

fn durable_inventory_matches(before: &CharacterInventory, after: &CharacterInventory) -> bool {
    fn without_session_handles(inventory: &CharacterInventory) -> CharacterInventory {
        let mut inventory = inventory.clone();
        for entry in inventory
            .main_backpack
            .iter_mut()
            .chain(&mut inventory.entries)
            .chain(inventory.equipment.iter_mut().map(|entry| &mut entry.item))
        {
            entry.handle = std::num::NonZeroU64::MIN;
        }
        inventory
    }
    without_session_handles(before) == without_session_handles(after)
}

fn check_clock_line(
    observed: &mut Option<u32>,
    text: &str,
    sequence: u64,
    reply: u64,
    first: u64,
    last: u64,
) -> TestResult {
    let minute = time_line_minute(text).ok_or("step=chat observed=invalid-clock-format")?;
    if observed.is_some() || sequence <= reply || !minute_in_command(minute, first, last) {
        return Err(format!("step=chat observed=minute:{minute},sequence:{sequence},duplicate:{} expected=command-ticks:{first}..={last},sequence-after:{reply}", observed.is_some()).into());
    }
    *observed = Some(minute);
    Ok(())
}

pub(super) async fn logout(client: &mut DevClientSession) -> TestResult {
    require_first_logout(client.session_mut().logout().await?.result)
}

fn require_first_logout(result: Option<LogoutResult>) -> TestResult {
    if result != Some(LogoutResult::Accepted) {
        return Err(format!("step=logout observed={result:?}").into());
    }
    Ok(())
}

fn within(a: TilePos, b: TilePos, range: u32) -> bool {
    a.floor == b.floor && u32::from(a.x.abs_diff(b.x).max(a.y.abs_diff(b.y))) <= range
}

fn gold_balance(client: &DevClientSession, gold: std::num::NonZeroU32) -> u64 {
    carried(client, gold)
        .into_iter()
        .map(|(_, count)| u64::from(count))
        .sum()
}

fn require_departure(position: TilePos) -> TestResult {
    if position != TOWN {
        return Err(format!("step=walk observed=start:{position:?} expected={TOWN:?}").into());
    }
    Ok(())
}

fn is_approach(position: TilePos) -> bool {
    position == LUBO_APPROACH
}

fn lubo_identity(
    table: &oteryn_world_bundle::npc::Table,
    clients: &SeamClients<'_>,
) -> TestResult<EntityRef> {
    use crate::foundation::{
        ActorLocalGeneration, ActorLocalId, ActorRef, ExactActorRef, ScopeOwnershipGeneration,
    };

    // NPC-LOAD-1 allocates the canonical admitted placements from the empty
    // carrier before creature actors. Rashid's rotation sorts after Lubo.
    let ordinal = table
        .npcs
        .iter()
        .take_while(|npc| npc.key.as_str() < "oteryn:npc.lubo")
        .map(|npc| npc.placements.len())
        .sum::<usize>()
        + 1;
    let scope = ScopeOwnershipGeneration::new(clients.scope_generation)
        .map_err(|_| "step=seller observed=invalid-scope-generation")?;
    let actor = ExactActorRef(ActorRef::from_owner_parts(
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
        scope,
        ActorLocalId(u32::try_from(ordinal)?),
        ActorLocalGeneration(1),
    ));
    Ok(EntityRef {
        identity: actor.placement_identity(),
        generation: actor.actor_local_generation(),
    })
}

fn validate_seller_placement(
    entry: &WorldSpatialEntity,
    expected: EntityRef,
    home: TilePos,
) -> TestResult {
    if entry.entity == expected
        && (entry.kind != EntityKind::Npc
            || !entry
                .display_name
                .is_some_and(|name| name.as_str() == Ok("Lubo"))
            || !tile(entry.position)
                .is_ok_and(|at| within(at, home, u32::from(LUBO_WANDER_RADIUS))))
    {
        return Err(format!(
            "step=seller observed=invalid-lubo-placement:{:?} home={home:?}",
            entry.position
        )
        .into());
    }
    Ok(())
}

#[cfg(test)]
fn is_lubo(
    entry: &WorldSpatialEntity,
    expected: EntityRef,
    home: TilePos,
    player: TilePos,
) -> bool {
    entry.entity == expected
        && validate_seller_placement(entry, expected, home).is_ok()
        && tile(entry.position).is_ok_and(|at| within(at, player, npc::TALK_RANGE_TILES))
}

fn inspect_approach_events(
    client: &mut DevClientSession,
    inventory: &CharacterInventory,
    seller: EntityRef,
    home: TilePos,
    npc: &NpcEvidence,
    at_shop: bool,
) -> TestResult {
    if at_shop {
        fixed_position(client.world_spatial().actor_position)?;
    }
    for event in client.take_events() {
        npc.check(&event)?;
        if at_shop {
            check_position_event(&event)?;
        }
        match event {
            SessionEvent::Inventory(delta) => require_unchanged_inventory(inventory, &delta.value)?,
            SessionEvent::WorldSpatialEntities(delta) => {
                for entry in delta.value.enter.iter().chain(&delta.value.update) {
                    validate_seller_placement(entry, seller, home)?;
                }
            }
            _ => {}
        }
    }
    for entry in client
        .world_entities()
        .into_iter()
        .flat_map(|entities| entities.others())
    {
        validate_seller_placement(entry, seller, home)?;
    }
    require_unchanged_inventory(
        inventory,
        client
            .inventory()
            .ok_or("step=approach missing inventory")?,
    )
}

fn watch_offer(offers: &[npc::NpcOffer], watch: u32) -> TestResult<(usize, &npc::NpcOffer)> {
    let mut matches = offers.iter().enumerate().filter(|(_, offer)| {
        offer.side == npc::NpcTradeSide::Buy && offer.item_definition_ref == watch
    });
    let (index, offer) = matches
        .next()
        .ok_or("step=buy observed=lubo-omitted-watch-offer")?;
    if matches.next().is_some() || offer.unit_price != PRICE || offer.count != 1 {
        return Err(format!(
            "step=buy observed=ambiguous-or-incorrect-watch-offer price={} quantity={}",
            offer.unit_price, offer.count
        )
        .into());
    }
    Ok((index, offer))
}

fn purchase_inventory_matches(
    before: &CharacterInventory,
    inventory: &CharacterInventory,
    watch: std::num::NonZeroU32,
    gold: std::num::NonZeroU32,
    remaining_gold: u64,
) -> bool {
    let unrelated = |state: &CharacterInventory| {
        let mut state = state.clone();
        state.entries.retain(|entry| {
            entry.item_definition_ref != watch && entry.item_definition_ref != gold
        });
        state
    };
    unrelated(before) == unrelated(inventory)
        && matches!(carried_in(inventory, watch).as_slice(), [(_, 1)])
        && carried_in(inventory, gold)
            .iter()
            .map(|(_, count)| u64::from(*count))
            .sum::<u64>()
            == remaining_gold
}

async fn npc_command(
    client: &mut DevClientSession,
    command: u32,
    payload: &[u8],
) -> TestResult<u64> {
    let reply = client
        .session_mut()
        .gated_command(npc::CAPABILITY_NPC_SERVICE_V1, command, payload)
        .await?;
    let disposition = npc::decode_npc_intent_result(&reply.payload)
        .map_err(|error| format!("step=npc command={command} reply-decode={error:?}"))?;
    if reply.status != CommandStatus::Accepted || disposition != npc::NpcIntentDisposition::Ok {
        return Err(format!(
            "step=npc command={command} status={:?} disposition={disposition:?}",
            reply.status
        )
        .into());
    }
    Ok(reply.server_sequence)
}

async fn talk(
    client: &mut DevClientSession,
    actor: oteryn_dev_client::EntityRef,
    text: &str,
) -> TestResult<u64> {
    let payload = npc::encode_npc_talk_intent(&npc::NpcTalkIntent {
        npc_actor: actor,
        text: text.into(),
    })
    .map_err(|error| format!("step=talk-{text} encode={error:?}"))?;
    npc_command(client, npc::COMMAND_TYPE_NPC_TALK_INTENT, &payload).await
}

fn lubo_greeting_matches(lines: &[String]) -> bool {
    lines == ["Welcome to my adventurer shop, Pocket Watch Player! What do you need?"]
}

async fn buy_watch(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    watch: std::num::NonZeroU32,
    gold: std::num::NonZeroU32,
    funded_gold: u64,
    remaining_gold: u64,
) -> TestResult<(CharacterInventory, NpcEvidence, u64)> {
    let mut npc_state = NpcEvidence::default();
    let before = client
        .inventory()
        .cloned()
        .ok_or("step=walk missing inventory")?;
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let bundle = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let seller = bundle
        .npcs
        .npcs
        .iter()
        .find(|seller| seller.key == "oteryn:npc.lubo")
        .ok_or("step=seller observed=lubo-missing-from-bundle")?;
    let [placement] = seller.placements.as_slice() else {
        return Err("step=seller observed=expected-one-lubo-placement".into());
    };
    let expected_seller = lubo_identity(&bundle.npcs, clients)?;
    let home = TilePos {
        x: placement.x,
        y: placement.y,
        floor: placement.floor,
    };
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let here = tile(client.world_spatial().actor_position)?;
    require_departure(here)?;
    let path = route_with_landings(
        here,
        is_approach,
        |_| true,
        |at, direction| destination_with_teleports(world, at, direction, false),
    )
    .ok_or("step=seller observed=no-route-to-lubo-approach")?;
    if path.is_empty() {
        return Err("step=walk observed=empty-route".into());
    }
    evidence(&format!(
        "G09_ROUTE from={here:?} approach={LUBO_APPROACH:?} seller_home={home:?} steps={}",
        path.len()
    ));
    inspect_approach_events(client, &before, expected_seller, home, &npc_state, false)?;
    for direction in path {
        let from = tile(client.world_spatial().actor_position)?;
        let outcome = client.step(direction).await?;
        let after = tile(client.world_spatial().actor_position)?;
        if outcome.status != CommandStatus::Accepted
            || outcome.disposition != StepDisposition::Moved
            || from == after
            || outcome.world_spatial_delta.is_none()
            || destination_with_teleports(world, from, direction, false) != Some(after)
        {
            return Err(format!(
                "step=walk from={from:?} direction={direction:?} to={after:?} observed={:?}",
                outcome.disposition
            )
            .into());
        }
        verdict::Harness::global().progress("walk");
        inspect_approach_events(client, &before, expected_seller, home, &npc_state, false)?;
        if outcome.step_wait_ms != 0 {
            client
                .service_liveness(Duration::from_millis(u64::from(outcome.step_wait_ms)))
                .await?;
        }
    }
    let here = tile(client.world_spatial().actor_position)?;
    if !is_approach(here) {
        return Err(
            format!("step=walk observed=arrival:{here:?} expected={LUBO_APPROACH:?}").into(),
        );
    }
    inspect_approach_events(client, &before, expected_seller, home, &npc_state, false)?;
    let entry = {
        let mut visible = client
            .world_entities()
            .into_iter()
            .flat_map(|entities| entities.others())
            .filter(|entry| entry.entity == expected_seller);
        let entry = visible
            .next()
            .copied()
            .ok_or("step=seller observed=lubo-not-visible-on-arrival")?;
        validate_seller_placement(&entry, expected_seller, home)?;
        if visible.next().is_some() || !within(tile(entry.position)?, here, npc::TALK_RANGE_TILES) {
            return Err(format!(
                "step=seller observed=ambiguous-or-out-of-talk-range at={:?}",
                entry.position
            )
            .into());
        }
        entry
    };
    let actor = entry.entity;
    evidence(&format!(
        "G09_CHECK step=seller status=PASS position={:?}",
        entry.position
    ));

    let conversation = client
        .session_mut()
        .gated_snapshot(npc::STATE_DOMAIN_NPC_CONVERSATION)
        .ok_or("step=seller observed=missing-conversation-snapshot")?;
    let initial = npc::decode_npc_conversation(&conversation.payload)
        .map_err(|error| format!("step=seller observed=conversation-snapshot-decode:{error:?}"))?;
    let details = client
        .selected_capabilities()
        .contains(&oteryn_protocol_oteryn::trade_details::CAPABILITY_TRADE_DETAILS_V1);
    let window = client
        .session_mut()
        .gated_snapshot(npc::STATE_DOMAIN_NPC_TRADE_WINDOW)
        .ok_or("step=seller observed=missing-trade-snapshot")?;
    let initial_window = npc::decode_npc_trade_window_for(&window.payload, details)
        .map_err(|error| format!("step=seller observed=trade-snapshot-decode:{error:?}"))?;
    if initial.open.is_some() || initial_window.open.is_some() {
        return Err("step=seller observed=conversation-or-trade-already-open".into());
    }
    let greeting = talk(client, actor, "hi").await?;
    let delta = client
        .session_mut()
        .read_gated_delta(npc::STATE_DOMAIN_NPC_CONVERSATION, 1)
        .await?;
    let conversation = npc::decode_npc_conversation(&delta.payload)
        .map_err(|error| format!("step=seller observed=conversation-delta-decode:{error:?}"))?;
    if delta.server_sequence <= greeting
        || conversation.open.map(|open| open.0) != Some(actor)
        || !lubo_greeting_matches(&conversation.lines)
    {
        return Err(format!(
            "step=seller expected=authored-lubo-greeting reply_sequence={greeting} delta_sequence={} open={:?} lines={:?}",
            delta.server_sequence, conversation.open, conversation.lines
        ).into());
    }
    inspect_approach_events(client, &before, expected_seller, home, &npc_state, true)?;
    npc_state.greeting = Some(conversation);
    let trade = talk(client, actor, "trade").await?;
    let delta = client
        .session_mut()
        .read_gated_delta(npc::STATE_DOMAIN_NPC_TRADE_WINDOW, 1)
        .await?;
    let window = npc::decode_npc_trade_window_for(&delta.payload, details)
        .map_err(|error| format!("step=buy observed=window-decode:{error:?}"))?;
    let (seller, revision) = window.open.ok_or("step=buy observed=trade-window-closed")?;
    if delta.server_sequence <= trade || seller != actor {
        return Err("step=buy observed=trade-window-for-other-seller".into());
    }
    let (offer_index, offer) = watch_offer(&window.offers, watch.get())?;
    npc_state.window = Some(window.clone());
    inspect_approach_events(client, &before, expected_seller, home, &npc_state, true)?;
    if !carried(client, watch).is_empty() || gold_balance(client, gold) != funded_gold {
        return Err("step=buy observed=inventory-changed-before-purchase".into());
    }
    let payload = npc::encode_npc_trade_intent(&npc::NpcTradeIntent {
        npc_actor: actor,
        catalogue_revision: revision,
        offer_index: u32::try_from(offer_index)?,
        side: offer.side,
        quantity: 1,
        expected_unit_price: offer.unit_price,
    })
    .map_err(|error| format!("step=buy encode={error:?}"))?;
    let clock_from = clock_elapsed_ms()?;
    if !client.selected_capabilities().contains(&npc::CAPABILITY_NPC_PURCHASE_ATTRIBUTION_V1) {
        return Err("step=buy observed=purchase-attribution-not-negotiated".into());
    }
    let purchase_command_id = client.next_command_id();
    let purchase = npc_command(client, npc::COMMAND_TYPE_NPC_TRADE_INTENT, &payload).await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let mut purchased = None;
        fixed_position(client.world_spatial().actor_position)?;
        for event in client.take_events() {
            npc_state.check(&event)?;
            check_position_event(&event)?;
            if let SessionEvent::WorldClock(delta) = &event {
                validate_published_minute(
                    delta.value.tibian_minute,
                    clock_from,
                    clock_elapsed_ms()?,
                )?;
            }
            if let SessionEvent::Inventory(delta) = event {
                if purchased.is_some() {
                    // The session exposes attribution for its latest inventory revision.
                    return Err("step=buy observed=ambiguous-purchase-inventory-batch".into());
                } else {
                    if delta.server_sequence <= purchase
                        || !purchase_inventory_matches(
                            &before,
                            &delta.value,
                            watch,
                            gold,
                            remaining_gold,
                        )
                    {
                        return Err("step=buy observed=incorrect-first-inventory-delta".into());
                    }
                    purchased = Some(delta.value);
                }
            }
        }
        if let Some(purchased) = purchased {
            if client.session_mut().inventory_npc_purchase_command_ids() != [purchase_command_id] {
                return Err("step=buy observed=missing-purchase-command-attribution".into());
            }
            require_unchanged_inventory(
                &purchased,
                client.inventory().ok_or("step=buy missing inventory")?,
            )?;
            evidence(&format!(
                "G09_CHECK step=buy status=PASS watch={WATCH} price={PRICE} gold_before={funded_gold} gold_after={remaining_gold}"
            ));
            return Ok((purchased, npc_state, clock_from));
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step=buy observed=missing-exact-watch-credit-and-gold-debit".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    character_prep::declare_seeded(
        "G09",
        &format!("{PURCHASE_FUNDING} gold"),
        "Lubo's authored watch price",
    );
    let identity = character_prep::dedicated_character(clients, "NODE_BOOT_G09_ACCOUNT").await?;
    let clients = &identity.clients(clients);
    let index = provisioning::item_index()?;
    let definition = provisioning::item_reference(&index, WATCH)?;
    let gold = provisioning::item_reference(&index, GOLD)?;
    let mut client = connect_character(clients, 0xcd, 0).await?;
    if !carried(&client, definition).is_empty() {
        return Err("step=fixture observed=watch-already-present".into());
    }
    let starter_gold = gold_balance(&client, gold);
    let funded_gold = starter_gold + u64::from(PURCHASE_FUNDING);
    let remaining_gold = funded_gold
        .checked_sub(u64::from(PRICE))
        .ok_or("step=fixture observed=no-starter-coin-for-purchase")?;
    character_prep::grant_item(&client, GOLD, PURCHASE_FUNDING).await?;
    logout(&mut client).await?;
    drop(client);

    let mut client = connect_character(clients, 0xce, 0).await?;
    if !carried(&client, definition).is_empty() || gold_balance(&client, gold) != funded_gold {
        return Err("step=fixture observed=expected-only-purchase-gold".into());
    }
    let (purchased_inventory, npc_state, clock_from) = buy_watch(
        &mut client,
        clients,
        definition,
        gold,
        funded_gold,
        remaining_gold,
    )
    .await?;
    let inventory = carried(&client, definition);
    let [(handle, 1)] = inventory.as_slice() else {
        return Err("step=fixture observed=expected-exactly-one-served-watch".into());
    };
    let handle = *handle;
    // Sample near the end of the tick so premature rollover becomes observable.
    let first_minute = read_time(
        &mut client,
        handle,
        &purchased_inventory,
        LATE_CLOCK_PHASE_MS,
        &npc_state,
        clock_from,
    )
    .await?;
    if carried(&client, definition) != inventory {
        return Err("step=consume observed=watch-consumed;needed=clock-read-keeps-item".into());
    }
    let previous_session = client.join_snapshot().game_session_id;
    drop(client);
    let admission_started = clock_elapsed_ms()?;
    let mut fresh = connect_character(clients, 0xcf, 0).await?;
    if fresh.join_snapshot().game_session_id == previous_session
        || gold_balance(&fresh, gold) != remaining_gold
    {
        return Err("step=fresh observed=stale-session-or-position-or-gold-debit-lost".into());
    }
    fixed_position(fresh.world_spatial().actor_position)?;
    use oteryn_protocol_oteryn::world_clock::{STATE_DOMAIN_WORLD_CLOCK, decode_world_clock};
    let snapshot = fresh
        .session_mut()
        .gated_snapshot(STATE_DOMAIN_WORLD_CLOCK)
        .ok_or("step=clock observed=missing-world-clock-snapshot")?;
    let published = decode_world_clock(&snapshot.payload)
        .map_err(|error| format!("step=clock observed=snapshot-decode:{error:?}"))?;
    validate_published_minute(
        published.tibian_minute,
        admission_started,
        clock_elapsed_ms()?,
    )?;
    let fresh_inventory = fresh
        .inventory()
        .cloned()
        .ok_or("step=fresh missing inventory")?;
    if !durable_inventory_matches(&purchased_inventory, &fresh_inventory) {
        return Err("step=fresh observed=full-durable-inventory-mismatch".into());
    }
    let durable_inventory = carried(&fresh, definition);
    let [(again, 1)] = durable_inventory.as_slice() else {
        return Err("step=fresh observed=expected-exactly-one-durable-watch".into());
    };
    let second_minute = read_time(
        &mut fresh,
        *again,
        &fresh_inventory,
        EARLY_CLOCK_PHASE_MS,
        &NpcEvidence::default(),
        admission_started,
    )
    .await
    .map_err(|error| verdict::context("step=fresh-clock".into(), error))?;
    let after = carried(&fresh, definition);
    if second_minute == first_minute {
        return Err("step=fresh observed=clock-did-not-advance".into());
    }
    if after != durable_inventory {
        return Err("step=fresh observed=watch-consumed-on-second-read".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_seller_after_valid_window() -> TestResult {
        let seller = EntityRef {
            identity: [1; 16],
            generation: 1,
        };
        let window = npc::NpcTradeWindow {
            open: Some((seller, 1)),
            ..Default::default()
        };
        let state = NpcEvidence {
            window: Some(window.clone()),
            ..Default::default()
        };
        let event = |value| {
            SessionEvent::NpcTradeWindow(oteryn_dev_client::AppliedDelta {
                server_sequence: 5,
                base_revision: 1,
                new_revision: 2,
                value,
            })
        };
        state.check(&event(window.clone()))?;
        assert!(
            state
                .check(&event(npc::NpcTradeWindow {
                    open: Some((
                        EntityRef {
                            identity: [2; 16],
                            ..seller
                        },
                        1
                    )),
                    ..window
                }))
                .is_err()
        );
        Ok(())
    }

    #[test]
    fn rejects_transient_move_and_wrong_published_minute() -> TestResult {
        let approach = ActorPosition {
            x: i32::from(LUBO_APPROACH.x),
            y: i32::from(LUBO_APPROACH.y),
            floor: -i16::from(LUBO_APPROACH.floor),
        };
        fixed_position(approach)?;
        assert!(
            fixed_position(ActorPosition {
                x: approach.x + 1,
                ..approach
            })
            .is_err()
        );
        let start = 5 * GLOBAL_MINUTE_MS;
        assert!(minute_in_command(5, 5, 5));
        validate_published_minute(5, start, start + 2000)?;
        assert!(validate_published_minute(6, start, start + 2000).is_err());
        Ok(())
    }

    #[test]
    fn conflicting_watch_quote_cannot_hide_behind_correct_quote() {
        let offer = npc::NpcOffer {
            item_definition_ref: WATCH,
            side: npc::NpcTradeSide::Buy,
            count: 1,
            unit_price: PRICE,
        };
        assert!(watch_offer(&[offer], WATCH).is_ok());
        let wrong = npc::NpcOffer {
            unit_price: PRICE + 1,
            ..offer
        };
        assert!(watch_offer(&[offer, wrong], WATCH).is_err());
        assert!(watch_offer(&[wrong, offer], WATCH).is_err());
    }

    #[test]
    fn transient_inventory_mutation_and_relog_item_loss_are_rejected() {
        use oteryn_protocol_oteryn::item_view::ItemEntry;
        use std::num::{NonZeroU32, NonZeroU64};
        let item = ItemEntry {
            appearance_id: 1,
            handle: NonZeroU64::MIN,
            item_definition_ref: NonZeroU32::MIN,
            count: NonZeroU32::MIN,
            sub_type: 0,
        };
        let expected = CharacterInventory {
            entries: vec![item],
            ..Default::default()
        };
        let missing = CharacterInventory::default();
        assert!(
            [&expected, &missing, &expected]
                .into_iter()
                .try_for_each(|inventory| require_unchanged_inventory(&expected, inventory))
                .is_err()
        );
        let mut fresh = expected.clone();
        fresh.entries[0].handle = NonZeroU64::MAX;
        assert!(durable_inventory_matches(&expected, &fresh));
        assert!(!durable_inventory_matches(&expected, &missing));
    }

    #[test]
    fn extra_clock_line_in_another_channel_is_rejected() {
        let text = "The time is 0:06.".to_owned();
        let correct = ChatLine::Private {
            speaker_name: "Game".to_owned(),
            text: text.clone(),
        };
        let wrong = ChatLine::System { text };
        assert!(private_clock_text(&correct).is_ok_and(|line| line.is_some()));
        assert!(
            [correct, wrong]
                .iter()
                .try_for_each(|line| private_clock_text(line).map(|_| ()))
                .is_err()
        );
    }

    #[test]
    fn route_requires_town_departure_and_exact_lubo_approach() {
        assert!(require_departure(TOWN).is_ok());
        assert!(require_departure(LUBO_APPROACH).is_err());
        assert!(is_approach(LUBO_APPROACH));
        assert!(!is_approach(TilePos {
            x: LUBO_APPROACH.x - 1,
            ..LUBO_APPROACH
        }));
    }

    #[test]
    fn first_logout_refuses_busy() {
        assert!(require_first_logout(Some(LogoutResult::Accepted)).is_ok());
        assert!(require_first_logout(Some(LogoutResult::Busy)).is_err());
    }

    #[test]
    fn same_name_and_position_do_not_substitute_for_lubos_identity() -> TestResult {
        use oteryn_protocol_oteryn::{
            entity_names::EntityDisplayName, world_spatial_entities::EntityDetail,
        };
        let expected = EntityRef {
            identity: [1; 16],
            generation: 1,
        };
        let home = TilePos {
            x: 10,
            y: 10,
            floor: -7,
        };
        let player = TilePos { x: 8, ..home };
        let mut entry = WorldSpatialEntity {
            entity: expected,
            kind: EntityKind::Npc,
            position: ActorPosition {
                x: 11,
                y: 10,
                floor: 7,
            },
            display_name: Some(
                EntityDisplayName::new("Lubo").map_err(|error| format!("{error:?}"))?,
            ),
            detail: EntityDetail::Actor {
                direction: StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
            motion: None,
        };
        let mut outside = entry;
        outside.position.x = i32::from(home.x + LUBO_WANDER_RADIUS + 1);
        assert!(
            [&outside, &entry]
                .into_iter()
                .try_for_each(|observed| validate_seller_placement(observed, expected, home))
                .is_err()
        );

        assert!(is_lubo(&entry, expected, home, player));
        entry.entity.identity = [2; 16];
        assert!(!is_lubo(&entry, expected, home, player));
        entry.entity = EntityRef {
            generation: 2,
            ..expected
        };
        assert!(!is_lubo(&entry, expected, home, player));
        Ok(())
    }

    #[test]
    fn purchase_delta_must_itself_show_watch_credit_and_gold_debit() -> TestResult {
        use oteryn_protocol_oteryn::item_view::ItemEntry;
        use std::num::{NonZeroU32, NonZeroU64};
        let watch = NonZeroU32::new(WATCH).ok_or("watch")?;
        let gold = NonZeroU32::new(GOLD).ok_or("gold")?;
        let mut event_value = CharacterInventory {
            entries: vec![ItemEntry {
                appearance_id: 0,
                handle: NonZeroU64::MIN,
                item_definition_ref: gold,
                count: NonZeroU32::new(69).ok_or("funded gold")?,
                sub_type: 0,
            }],
            ..Default::default()
        };
        let before = event_value.clone();
        assert!(!purchase_inventory_matches(
            &before,
            &event_value,
            watch,
            gold,
            49
        ));
        event_value.entries.push(ItemEntry {
            appearance_id: 0,
            handle: NonZeroU64::new(2).ok_or("watch handle")?,
            item_definition_ref: watch,
            count: NonZeroU32::MIN,
            sub_type: 0,
        });
        assert!(!purchase_inventory_matches(
            &before,
            &event_value,
            watch,
            gold,
            49
        ));
        event_value.entries[0].count = NonZeroU32::new(49).ok_or("remaining gold")?;
        assert!(purchase_inventory_matches(
            &before,
            &event_value,
            watch,
            gold,
            49
        ));
        let mut unexpected = event_value.clone();
        unexpected.entries.push(ItemEntry {
            item_definition_ref: NonZeroU32::MIN,
            ..event_value.entries[1]
        });
        assert!(!purchase_inventory_matches(
            &before,
            &unexpected,
            watch,
            gold,
            49
        ));
        event_value.entries.pop();
        assert!(!purchase_inventory_matches(
            &before,
            &event_value,
            watch,
            gold,
            49
        ));
        Ok(())
    }

    #[test]
    fn lubo_wander_and_talk_range_are_independent() {
        let home = TilePos {
            x: 10,
            y: 10,
            floor: -7,
        };
        let player = TilePos {
            x: home.x - 3,
            ..home
        };
        let moved = TilePos {
            x: home.x + 1,
            ..home
        };
        let furthest = TilePos {
            x: home.x + LUBO_WANDER_RADIUS,
            ..home
        };
        let wander = u32::from(LUBO_WANDER_RADIUS);
        assert!(within(moved, home, wander));
        assert!(within(moved, player, npc::TALK_RANGE_TILES));
        assert!(within(furthest, home, wander));
        assert!(!within(furthest, player, npc::TALK_RANGE_TILES));
        assert!(!within(
            TilePos {
                floor: home.floor - 1,
                ..moved
            },
            home,
            wander
        ));
    }

    #[test]
    fn late_sample_exposes_premature_rollover() {
        let minute_ms = GLOBAL_MINUTE_MS;
        for phase in LATE_CLOCK_PHASE_MS {
            assert!(!EARLY_CLOCK_PHASE_MS.contains(&phase));
            // A clock ticking 500 ms early already reads minute 6 at this sample.
            let premature = (5 * minute_ms + phase + 500) / minute_ms;
            assert!(!minute_in_command(
                u32::try_from(premature).expect("sample minute fits"),
                5,
                5
            ));
        }
    }

    #[test]
    fn clock_match_is_bounded_and_wraps_at_midnight() {
        assert!(minute_in_command(1439, 1439, 1439));
        assert!(minute_in_command(0, 1440, 1440));
        assert!(minute_in_command(1439, 1439, 1440));
        assert!(minute_in_command(0, 1439, 1440));
        assert!(!minute_in_command(1, 1439, 1440));
        assert!(!minute_in_command(0, 1439, 1443));
        assert!(!minute_in_command(0, 1440, 1439));
    }

    #[test]
    fn clock_line_accepts_only_sampled_endpoint_minutes() -> TestResult {
        check_clock_line(&mut None, "The time is 0:05.", 11, 10, 5, 6)?;
        check_clock_line(&mut None, "The time is 0:06.", 11, 10, 5, 6)?;
        assert!(check_clock_line(&mut None, "The time is 0:07.", 11, 10, 5, 6).is_err());
        assert!(check_clock_line(&mut None, "The time is 0:06.", 11, 10, 5, 7).is_err());
        Ok(())
    }

    #[test]
    fn clock_rejects_pre_reply_wrong_and_duplicate_lines() -> TestResult {
        let mut seen = None;
        assert!(check_clock_line(&mut seen, "The time is 0:05.", 10, 10, 5, 5).is_err());
        assert!(check_clock_line(&mut seen, "The time is 0:06.", 11, 10, 5, 5).is_err());
        check_clock_line(&mut seen, "The time is 0:05.", 11, 10, 5, 5)?;
        assert!(check_clock_line(&mut seen, "The time is 0:06.", 12, 10, 5, 5).is_err());
        assert!(check_clock_line(&mut seen, "The time is 0:05.", 12, 10, 5, 5).is_err());
        Ok(())
    }

    #[test]
    fn greeting_requires_exact_global_text() {
        assert!(lubo_greeting_matches(&[
            "Welcome to my adventurer shop, Pocket Watch Player! What do you need?".into()
        ]));
        assert!(!lubo_greeting_matches(&[
            "Welcome to my adventurer shop, Pocket Watch Player! What do you need? Ask me for a {trade} to look at my wares.".into()
        ]));
        assert!(!lubo_greeting_matches(&[
            "Welcome, Pocket Watch Player!".into()
        ]));
    }

    #[test]
    fn time_line_is_parsed_only_for_the_server_format() {
        assert_eq!(time_line_minute("The time is 7:05."), Some(425));
        assert_eq!(time_line_minute("The time is 23:59."), Some(1439));
        assert_eq!(time_line_minute("The time is 24:00."), None);
        assert_eq!(time_line_minute("The time is 7:5."), None);
        assert_eq!(time_line_minute("The time is 07:05."), None);
        assert_eq!(time_line_minute("It is noon."), None);
        assert_eq!(token("a  b"), "a-b");
    }
}
