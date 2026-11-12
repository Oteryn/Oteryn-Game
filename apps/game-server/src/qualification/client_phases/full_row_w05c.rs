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
//! The audit also names sunrise/sunset NPCs as an unimplemented remainder.
//! This journey therefore qualifies the row's implemented weekday lifecycle;
//! it never changes the clock or claims day/night coverage from source text.

use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind, EntityRef, SessionEvent};
use oteryn_protocol_oteryn::npc_service as npc;
use std::collections::{BTreeMap, VecDeque};

const ACTIONS: &[&str] = &[
    "walk-weekday-destination",
    "weekday-dialogue-trade",
    "out-of-range-refusal",
    "fresh-admission-weekday-trade",
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

    fn finish(mut self, result: TestResult) {
        if let Err(error) = result {
            self.failure = Some(error.to_string());
        }
        let complete =
            self.failure.is_none() && ACTIONS.iter().all(|action| self.passed.contains(action));
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
    let result = verdict::indexed_timeout(Duration::from_secs(1_800), verdict::observe(journey(&mut row, clients))).await;
    row.finish(match result {
        Ok(result) => result,
        Err(_) => {
            Err("step=journey observed=timeout-1800s need=reachable-weekday-NPC-route".into())
        }
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

async fn approach(
    client: &mut DevClientSession,
    world: &BundleWorld,
    target: TilePos,
) -> TestResult<EntityRef> {
    let start = tile(client.world_spatial().actor_position)?;
    let path = full_route(world, start, target).ok_or_else(|| {
        format!("step=walk-weekday-destination observed=no-route from={start:?} to={target:?} need=player-reachable-Rashid-placement")
    })?;
    walk(client, world, &path).await.map_err(|error| { verdict::record_error(verdict::IndexedError::as_error(&error)); verdict::context(format!(
            "step=walk-weekday-destination observed={error} need=accepted-STEP-and-spatial-delta"
        ), error) })?;
    let entities = client.world_entities().ok_or_else(|| {
        format!("step=observe-weekday-spawn observed=missing-world-entities need=exactly-one-NPC-at-{target:?}")
    })?;
    let mut matches = entities
        .iter()
        .filter(|entry| entry.kind == EntityKind::Npc && tile(entry.position).ok() == Some(target));
    let actor = matches.next().map(|entry| entry.entity);
    let extra = matches.count();
    match (actor, extra) {
        (Some(actor), 0) => Ok(actor),
        (actor, extra) => Err(format!(
            "step=observe-weekday-spawn observed=npc-count-{}-at-{target:?} need=exactly-one-Rashid-actor",
            usize::from(actor.is_some()) + extra
        )
        .into()),
    }
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

async fn refusal(
    client: &mut DevClientSession,
    world: &BundleWorld,
    actor: EntityRef,
    npc_position: TilePos,
) -> TestResult {
    let origin = tile(client.world_spatial().actor_position)?;
    let path = route(world, origin, |at| {
        at.floor == npc_position.floor
            && at
                .x
                .abs_diff(npc_position.x)
                .max(at.y.abs_diff(npc_position.y))
                > 4
    })
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
    if reply.status != CommandStatus::Rejected || disposition == npc::NpcIntentDisposition::Ok {
        return Err(format!("step=out-of-range-refusal observed=status:{:?},reply:{disposition:?} need=Rejected/non-Ok", reply.status).into());
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
            "step=out-of-range-refusal observed=status:{:?},reply:{disposition:?},new-trade-window:{:?} need=Rejected/non-Ok-without-new-trade-window",
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

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let world = qualification_world(clients)?;
    let destination = today()?.placement;
    let target = TilePos {
        x: destination.x,
        y: destination.y,
        floor: -destination.floor,
    };
    let mut client = connect_world(clients, 0xe5).await?;
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
    let actor = approach(&mut fresh, world, target).await?;
    open_trade(&mut fresh, actor).await?;
    row.pass("fresh-admission-weekday-trade");
    logout(&mut fresh).await
}

#[cfg(test)]
mod tests {
    use super::*;

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
