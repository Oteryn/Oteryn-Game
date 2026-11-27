//! W05c: timed NPC spawns (weekday / day-night).
//!
//! Concrete player scenario derived from the W05c matrix/re-audit row and the
//! server-owned Rashid UTC schedule:
//! 1. Admit the provisioned node-boot character, compute today's UTC Rashid
//!    destination, and walk there with native STEP commands; every reply must
//!    say `Moved` and carry the matching world-spatial delta.
//! 2. Observe exactly one NPC actor at today's destination, send native NPC
//!    `hi` and `trade` commands, and require accepted command replies plus an
//!    open conversation delta and a non-empty trade-window delta for that actor.
//! 3. Walk out of interaction range and send `trade` to the formerly visible
//!    actor; require the server's refusal and no newly opened trade window.
//! 4. Log out, perform a fresh admission, walk to the same server-clock-owned
//!    destination, and repeat `hi`/`trade`; the NPC placement and availability
//!    must survive the player session boundary (there is no player-owned state
//!    to persist for this world schedule).
//!
//! 5. Walk to Fitzmaurice's placement and observe his presence change with
//!    a native day/night clock transition.

use super::*;
use oteryn_dev_client::{
    DevClientSession, EntityKind, EntityRef, SessionEvent, WorldSpatialEntity,
};
use oteryn_protocol_oteryn::npc_service as npc;
use std::collections::{BTreeMap, VecDeque};

// Crystal 201189f8, data-global/scripts/world_changes/spawns_npc_by_time.lua:101-106.
// The served clock-driven placement owner is bundle_npcs.rs:238.
const FITZMAURICE: TilePos = TilePos {
    x: 32_411,
    y: 32_486,
    floor: -7,
};
const DAY_NIGHT_ACTION: &str = "day-night-spawn-transition";
const DAY_NIGHT_OWNER: &str = "apps/game-server/src/node/serve/bundle_npcs.rs:238";
const W05C_BUDGET_SECONDS: u64 = 3_600;

const ACTIONS: &[&str] = &[
    "walk-weekday-destination",
    "weekday-dialogue-trade",
    "out-of-range-refusal",
    "fresh-admission-weekday-trade",
    DAY_NIGHT_ACTION,
];

struct Row {
    passed: std::collections::BTreeSet<&'static str>,
    started: std::time::Instant,
    failure: Option<String>,
}

impl Row {
    fn new() -> Self {
        Self {
            passed: Default::default(),
            started: std::time::Instant::now(),
            failure: None,
        }
    }

    fn pass(&mut self, action: &'static str) {
        if ACTIONS.contains(&action) && self.passed.insert(action) {
            evidence(&format!("W05c_CHECK {action} PASS server-frames-validated"));
        } else {
            self.failure = Some(format!("invalid-or-duplicate-step:{action}"));
        }
    }

    fn is_complete(&self) -> bool {
        self.failure.is_none() && ACTIONS.iter().all(|action| self.passed.contains(action))
    }

    fn finish(mut self, result: TestResult) {
        if let Err(error) = result {
            self.failure = Some(error.to_string());
        }
        let complete = self.is_complete();
        if let Some(failure) = &self.failure {
            evidence(&format!("W05c_GAP {failure}"));
        }
        let evidence_suffix = self
            .failure
            .as_deref()
            .map(|failure| format!(" evidence={}", compact(failure)))
            .unwrap_or_default();
        evidence(&format!(
            "e2e_all scenario=W05c.full status={} seconds={:.3}{evidence_suffix}",
            if complete { "PASS" } else { "FAIL" },
            self.started.elapsed().as_secs_f64(),
        ));
    }
}

fn compact(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_whitespace() {
                '-'
            } else {
                character
            }
        })
        .collect()
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result = verdict::indexed_timeout(
        Duration::from_secs(W05C_BUDGET_SECONDS),
        verdict::observe(journey(&mut row, clients)),
    )
    .await;
    row.finish(match result {
        Ok(result) => result,
        Err(_) => Err(format!("step=journey observed=timeout-{W05C_BUDGET_SECONDS}s").into()),
    });
}

fn qualification_world(clients: &SeamClients<'_>) -> TestResult<&'static BundleWorld> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )
}

fn today() -> TestResult<oteryn_gameplay_entities::rashid_schedule::RashidDestination> {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| format!("server clock before epoch: {error}"))?;
    Ok(oteryn_gameplay_entities::rashid_schedule::destination_at(
        i64::try_from(elapsed.as_millis())?,
    ))
}

pub(super) fn destination_tile(
    destination: oteryn_gameplay_entities::rashid_schedule::RashidDestination,
) -> TilePos {
    let placement = destination.placement;
    TilePos {
        x: placement.x,
        y: placement.y,
        floor: placement.floor,
    }
}

fn full_route(world: &BundleWorld, from: TilePos, goal: TilePos) -> Option<Vec<StepDirection>> {
    let mut queue = VecDeque::from([from]);
    let mut parents = BTreeMap::from([(from, None)]);
    while let Some(at) = queue.pop_front() {
        if at.floor == goal.floor && at.x.abs_diff(goal.x).max(at.y.abs_diff(goal.y)) <= 3 {
            let mut path = Vec::new();
            let mut cursor = at;
            while let Some(Some((previous, direction))) = parents.get(&cursor) {
                path.push(*direction);
                cursor = *previous;
            }
            path.reverse();
            return Some(path);
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
            if let std::collections::btree_map::Entry::Vacant(entry) = parents.entry(next) {
                entry.insert(Some((at, direction)));
                queue.push_back(next);
            }
        }
    }
    None
}

fn rashid_wander_area(at: TilePos, target: TilePos) -> bool {
    at.floor == target.floor && at.x.abs_diff(target.x).max(at.y.abs_diff(target.y)) <= 3
}

fn rashid_at(entity: &WorldSpatialEntity, target: TilePos) -> bool {
    entity.kind == EntityKind::Npc
        && tile(entity.position).is_ok_and(|at| rashid_wander_area(at, target))
        && entity
            .display_name
            .is_some_and(|name| name.as_str() == Ok("Rashid"))
}

pub(super) async fn approach(
    client: &mut DevClientSession,
    world: &BundleWorld,
    target: TilePos,
) -> TestResult<EntityRef> {
    let start = tile(client.world_spatial().actor_position)?;
    if world.base().tile(target.x, target.y, target.floor).is_none() {
        return Err(format!(
            "step=walk-weekday-destination observed=Rashid-placement-outside-served-map to={target:?} source=tools/qualification/node_boot/run.sh:139"
        ).into());
    }
    let path = full_route(world, start, target).ok_or_else(|| {
        format!("step=walk-weekday-destination observed=no-route from={start:?} to={target:?} need=player-reachable-Rashid-placement")
    })?;
    walk(client, world, &path).await.map_err(|error| { verdict::record_error(verdict::IndexedError::as_error(&error)); verdict::context(format!(
            "step=walk-weekday-destination observed={error} need=accepted-STEP-and-spatial-delta"
        ), error) })?;
    for _ in 0..8 {
        let actor = {
            let entities = client.world_entities().ok_or("step=observe-weekday-spawn observed=missing-world-entities")?;
            let mut matches = entities.iter().filter(|entry| rashid_at(entry, target));
            let actor = matches.next().copied().ok_or_else(|| {
                format!("step=observe-weekday-spawn observed=Rashid-not-visible within-three-tiles-of={target:?}")
            })?;
            if matches.next().is_some() {
                return Err("step=observe-weekday-spawn observed=duplicate-Rashid-actors".into());
            }
            actor
        };
        let here = tile(client.world_spatial().actor_position)?;
        let at = tile(actor.position)?;
        if rashid_wander_area(here, at) {
            return Ok(actor.entity);
        }
        let path = full_route(world, here, at)
            .ok_or("step=approach-Rashid observed=no-route-to-visible-actor")?;
        let direction = path.first().ok_or("step=approach-Rashid observed=no-approach-step")?;
        walk(client, world, &[*direction]).await?;
    }
    Err("step=approach-Rashid observed=actor-remained-out-of-talk-range".into())
}

async fn talk(client: &mut DevClientSession, actor: EntityRef, text: &str) -> TestResult<u64> {
    let payload = npc::encode_npc_talk_intent(&npc::NpcTalkIntent {
        npc_actor: actor,
        text: text.into(),
    })
    .map_err(|error| format!("NPC intent encode: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            npc::CAPABILITY_NPC_SERVICE_V1,
            npc::COMMAND_TYPE_NPC_TALK_INTENT,
            &payload,
        )
        .await?;
    let disposition = npc::decode_npc_intent_result(&reply.payload)
        .map_err(|error| format!("NPC intent reply: {error:?}"))?;
    if reply.status != CommandStatus::Accepted || disposition != npc::NpcIntentDisposition::Ok {
        return Err(format!(
            "step=talk-{text} observed=status:{:?},reply:{disposition:?} need=Accepted/Ok",
            reply.status
        )
        .into());
    }
    Ok(reply.server_sequence)
}

async fn open_trade(client: &mut DevClientSession, actor: EntityRef) -> TestResult {
    let greeting = talk(client, actor, "hi").await?;
    let delta = client
        .session_mut()
        .read_gated_delta(npc::STATE_DOMAIN_NPC_CONVERSATION, 1)
        .await?;
    let conversation = npc::decode_npc_conversation(&delta.payload)
        .map_err(|error| format!("conversation delta: {error:?}"))?;
    if delta.server_sequence <= greeting || conversation.open.map(|open| open.0) != Some(actor) {
        return Err(format!(
            "step=talk-hi observed=conversation:{conversation:?} need=new-open-conversation-delta"
        )
        .into());
    }
    let trade = talk(client, actor, "trade").await?;
    let delta = client
        .session_mut()
        .read_gated_delta(npc::STATE_DOMAIN_NPC_TRADE_WINDOW, 1)
        .await?;
    let window = npc::decode_npc_trade_window(&delta.payload)
        .map_err(|error| format!("trade window delta: {error:?}"))?;
    if delta.server_sequence <= trade
        || window.open.map(|open| open.0) != Some(actor)
        || window.offers.is_empty()
    {
        return Err(format!(
            "step=talk-trade observed=window:{window:?} need=new-nonempty-Rashid-trade-window"
        )
        .into());
    }
    Ok(())
}

fn outside_rashid_range(at: TilePos, anchor: TilePos) -> bool {
    at.floor == anchor.floor
        && u32::from(at.x.abs_diff(anchor.x).max(at.y.abs_diff(anchor.y))) > npc::TALK_RANGE_TILES + 3
}

fn range_refused(status: CommandStatus, disposition: npc::NpcIntentDisposition) -> bool {
    status == CommandStatus::Rejected && disposition == npc::NpcIntentDisposition::NotInRange
}

async fn refusal(
    client: &mut DevClientSession,
    world: &BundleWorld,
    actor: EntityRef,
    npc_position: TilePos,
) -> TestResult {
    let origin = tile(client.world_spatial().actor_position)?;
    let path = route(world, origin, |at| outside_rashid_range(at, npc_position))
    .ok_or("step=leave-range observed=no-route need=walkable-tile-beyond-NPC-range")?;
    walk(client, world, &path).await?;
    client.take_events();
    let payload = npc::encode_npc_talk_intent(&npc::NpcTalkIntent {
        npc_actor: actor,
        text: "trade".into(),
    })
    .map_err(|error| format!("refusal intent encode: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            npc::CAPABILITY_NPC_SERVICE_V1,
            npc::COMMAND_TYPE_NPC_TALK_INTENT,
            &payload,
        )
        .await?;
    let disposition = npc::decode_npc_intent_result(&reply.payload)
        .map_err(|error| format!("refusal reply: {error:?}"))?;
    if !range_refused(reply.status, disposition) {
        return Err(format!("step=out-of-range-refusal observed=status:{:?},reply:{disposition:?} need=Rejected/NotInRange", reply.status).into());
    }
    client.service_liveness(Duration::from_millis(250)).await?;
    let trade_window = client
        .take_events()
        .into_iter()
        .find_map(|event| match event {
            SessionEvent::NpcTradeWindow(delta)
                if delta.server_sequence > reply.server_sequence =>
            {
                Some(delta)
            }
            _ => None,
        });
    if let Some(delta) = trade_window {
        return Err(format!(
            "step=out-of-range-refusal observed=status:{:?},reply:{disposition:?},new-trade-window:{:?} need=Rejected/NotInRange-without-new-trade-window",
            reply.status, delta.value
        )
        .into());
    }
    Ok(())
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=logout observed={reply:?} need=Accepted").into());
    }
    Ok(())
}

fn fitzmaurice_at(
    entities: impl Iterator<Item = WorldSpatialEntity>,
) -> TestResult<Option<EntityRef>> {
    let mut matching = entities.filter(|entity| {
        entity.kind == EntityKind::Npc
            && entity
                .display_name
                .is_some_and(|name| name.as_str() == Ok("Fitzmaurice"))
    });
    let Some(resident) = matching.next() else {
        return Ok(None);
    };
    let at = tile(resident.position)?;
    if matching.next().is_some()
        || at.floor != FITZMAURICE.floor
        || at
            .x
            .abs_diff(FITZMAURICE.x)
            .max(at.y.abs_diff(FITZMAURICE.y))
            > 2
    {
        return Err(format!("step={DAY_NIGHT_ACTION} observed=duplicate-or-misplaced-Fitzmaurice at={at:?} source={DAY_NIGHT_OWNER}").into());
    }
    Ok(Some(resident.entity))
}

async fn observe_day_night_transition(
    client: &mut DevClientSession,
    world: &BundleWorld,
) -> TestResult {
    use oteryn_gameplay_world::world_clock::{SUNRISE_START, SUNSET_START, in_world_time};
    use oteryn_protocol_oteryn::{
        entity_names::CAPABILITY_WORLD_SPATIAL_ENTITIES_V2, world_clock::CAPABILITY_WORLD_LIGHT_V1,
    };

    verdict::Harness::global().progress(DAY_NIGHT_ACTION);
    let from = tile(client.world_spatial().actor_position)?;
    let path = full_route(world, from, FITZMAURICE).ok_or_else(|| {
        format!("step={DAY_NIGHT_ACTION} observed=no-walkable-route from={from:?} to={FITZMAURICE:?} need=player-access-to-Fitzmaurice source={DAY_NIGHT_OWNER}")
    })?;
    walk(client, world, &path).await?;
    if ![
        CAPABILITY_WORLD_LIGHT_V1,
        CAPABILITY_WORLD_SPATIAL_ENTITIES_V2,
    ]
    .iter()
    .all(|capability| client.selected_capabilities().contains(capability))
    {
        return Err(format!("step={DAY_NIGHT_ACTION} observed=missing-world-clock-or-entity-name-capability source={DAY_NIGHT_OWNER}").into());
    }

    client.take_events();
    let mut is_day = None;
    let mut initial = None;
    // The clock publishes a delta at least once per Tibian hour (150 real seconds).
    let now = tokio::time::Instant::now();
    let mut deadline = now + Duration::from_secs(180);
    let activity_interval = Duration::from_secs(600);
    let mut next_activity = now + activity_interval;
    verdict::Harness::global().bounded_wait(deadline, DAY_NIGHT_ACTION);
    while tokio::time::Instant::now() < deadline {
        if tokio::time::Instant::now() >= next_activity {
            // A night can outlast the gameplay idle limit even while transport pings succeed.
            let cleared = client.session_mut().attack_target(None).await?;
            if cleared.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
                return Err(format!("step={DAY_NIGHT_ACTION} observed=attack-clear-refused reply={cleared:?} source={DAY_NIGHT_OWNER}").into());
            }
            next_activity = tokio::time::Instant::now() + activity_interval;
        }
        client.service_liveness(Duration::from_secs(1)).await?;
        for event in client.take_events() {
            if let SessionEvent::WorldClock(delta) = event {
                is_day = Some(in_world_time(
                    u32::from(delta.value.tibian_minute),
                    SUNRISE_START,
                    SUNSET_START,
                ));
            }
        }
        let entities = client
            .world_entities()
            .ok_or("day-night observation has no spatial entities")?;
        let observed = fitzmaurice_at(entities.iter().copied())?;
        if let Some(day) = is_day
            && observed.is_some() != day
        {
            if let Some(first_day) = initial {
                if day != first_day {
                    return Ok(());
                }
            } else {
                initial = Some(day);
                // Sunset to sunrise is 750 world minutes, or 31m15s in real time.
                deadline = tokio::time::Instant::now() + Duration::from_secs(1_920);
                verdict::Harness::global().bounded_wait(deadline, DAY_NIGHT_ACTION);
            }
        }
    }
    Err(format!(
        "step={DAY_NIGHT_ACTION} observed=clock-day:{is_day:?},initial-day:{initial:?} need=Fitzmaurice-present-at-night-and-absent-by-day source={DAY_NIGHT_OWNER}"
    ).into())
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let world = qualification_world(clients)?;
    let mut client = connect_world(clients, 0xe5).await?;
    let target = destination_tile(today()?);
    // A missing weekday destination does not prevent observing the timed NPC.
    if world.base().tile(target.x, target.y, target.floor).is_none() {
        let weekday = approach(&mut client, world, target).await;
        let timed = observe_day_night_transition(&mut client, world).await;
        if timed.is_ok() {
            row.pass(DAY_NIGHT_ACTION);
        } else if let Err(error) = timed {
            evidence(&format!("W05c_GAP step={DAY_NIGHT_ACTION} observed={error}"));
        }
        if let Err(error) = logout(&mut client).await {
            evidence(&format!("W05c_GAP step=logout observed={error}"));
        }
        return weekday.map(|_| ());
    }
    let first_session = client.join_snapshot().game_session_id;
    let actor = approach(&mut client, world, target).await?;
    row.pass("walk-weekday-destination");
    open_trade(&mut client, actor).await?;
    row.pass("weekday-dialogue-trade");
    refusal(&mut client, world, actor, target).await?;
    row.pass("out-of-range-refusal");
    logout(&mut client).await?;
    drop(client);

    let mut fresh = connect_world(clients, 0xe6).await?;
    if fresh.join_snapshot().game_session_id == first_session {
        return Err("step=fresh-admission observed=reused-game-session need=new-session".into());
    }
    let fresh_target = destination_tile(today()?);
    let actor = approach(&mut fresh, world, fresh_target).await?;
    open_trade(&mut fresh, actor).await?;
    row.pass("fresh-admission-weekday-trade");
    let timed = observe_day_night_transition(&mut fresh, world).await;
    let cleanup = logout(&mut fresh).await;
    timed?;
    row.pass(DAY_NIGHT_ACTION);
    cleanup
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn review_rashid_refusal_covers_the_whole_wander_area() {
        let anchor = TilePos { x: 100, y: 100, floor: -7 };
        assert!(!outside_rashid_range(TilePos { x: 107, ..anchor }, anchor));
        assert!(outside_rashid_range(TilePos { x: 108, ..anchor }, anchor));
        assert!(range_refused(CommandStatus::Rejected, npc::NpcIntentDisposition::NotInRange));
        assert!(!range_refused(CommandStatus::Rejected, npc::NpcIntentDisposition::RateLimited));
    }

    #[test]
    fn rashid_can_wander_three_tiles_on_the_scheduled_floor() {
        let target = TilePos { x: 32328, y: 31782, floor: -6 };
        assert!(rashid_wander_area(TilePos { x: 32331, y: 31779, ..target }, target));
        assert!(!rashid_wander_area(TilePos { x: 32332, ..target }, target));
        assert!(!rashid_wander_area(TilePos { floor: -7, ..target }, target));
    }

    #[test]
    fn rashid_route_uses_the_authored_signed_floor() {
        use oteryn_gameplay_entities::rashid_schedule::destination_at;

        let thursday = destination_at(9 * 3_600_000);
        assert_eq!(
            destination_tile(thursday),
            TilePos {
                x: 33_066,
                y: 32_879,
                floor: -6,
            }
        );
    }

    #[test]
    fn weekday_trade_alone_does_not_qualify_day_night_spawns() {
        let mut row = Row::new();
        for action in ACTIONS
            .iter()
            .copied()
            .filter(|action| *action != DAY_NIGHT_ACTION)
        {
            row.pass(action);
        }
        assert!(!row.is_complete());
        row.pass(DAY_NIGHT_ACTION);
        assert!(row.is_complete());
    }

    #[test]
    fn verdict_requires_each_lifecycle_action() {
        let mut row = Row::new();
        for action in ACTIONS {
            row.pass(action);
        }
        assert!(row.failure.is_none());
        assert!(ACTIONS.iter().all(|action| row.passed.contains(action)));
        row.pass(ACTIONS[0]);
        assert!(row.failure.is_some());
    }

    #[test]
    fn evidence_is_one_token() {
        assert_eq!(
            compact("step=x observed=timed out"),
            "step=x-observed=timed-out"
        );
    }
}
