//! E2E M10 — bounded drowning evidence; the full swimming/trap/special-tile row stays incomplete.
//!
//! Concrete player scenario, derived from the M10 matrix/re-audit row and the
//! paced-movement underwater owner:
//! 1. Log in a provisioned node-boot character and walk, using ordinary STEP
//!    commands, from the temple to a reachable authored underwater ground.
//!    Every accepted step must carry a spatial delta and match the production
//!    floor resolver.
//! 2. Step onto the underwater ground.  The STEP reply must be `Moved` with a
//!    spatial delta; after the four-second interval the server must publish an
//!    ActorVitals delta for exactly 20 health of environmental drown damage.
//! 3. Attempt an ordinary STEP into a blocked neighbour while still underwater.
//!    The server must refuse movement without a spatial delta or position
//!    change, while the next drown tick still publishes a 20-health delta.
//! 4. Step off underwater ground.  The reply must move the player with a spatial
//!    delta, and no further drown-health delta may arrive in the next interval.
//! 5. Log out and obtain a fresh admission.  The game-session id must change,
//!    while the last legitimate position and damaged health remain durable.
//!
//! This journey uses only native player movement/logout commands.  It never
//! manufactures equipment, tiles, health, or character state.
use super::*;
use crate::map::view::MapFacts;
use oteryn_dev_client::{DevClientSession, SessionEvent};

const UNDERWATER: [u16; 12] = [
    5404, 5405, 5406, 5407, 5408, 5409, 5743, 5764, 8755, 8756, 8757, 9291,
];

struct Row {
    started: std::time::Instant,
    step: usize,
    error: Option<String>,
}

impl Row {
    fn pass(&mut self, step: usize, detail: &str) {
        self.step = step;
        evidence(&format!("M10_CHECK step={step} PASS {detail}"));
    }

    fn finish(self) {
        let drowning_proven = self.step == 5 && self.error.is_none();
        if let Some(error) = &self.error {
            evidence(&format!(
                "M10_CHECK step={} FAIL observed={error}",
                self.step + 1
            ));
        }
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=M10.full status=FAIL seconds={:.3} evidence={}",
            self.started.elapsed().as_secs_f64(),
            if drowning_proven {
                "observed-five-native-drowning-steps;needed=unproven-swimming-traps-special-movement"
            } else {
                "see-M10_CHECK;needed=unproven-swimming-traps-special-movement"
            },
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row {
        started: std::time::Instant::now(),
        step: 0,
        error: None,
    };
    let result = verdict::indexed_timeout(Duration::from_secs(150), verdict::observe(journey(&mut row, clients))).await;
    row.error = match result {
        Ok(Ok(())) => None,
        Ok(Err(error)) => Some(error.to_string()),
        Err(_) => Some("timeout waiting for movement/drowning server frames".into()),
    };
    row.finish();
}

fn is_underwater(world: &BundleWorld, at: TilePos) -> bool {
    world.base().tile(at.x, at.y, at.floor).is_some_and(|tile| {
        tile.ground()
            .and_then(|(ordinal, id)| world.facts().base_entry(at, ordinal, id))
            .is_some_and(|entry| UNDERWATER.contains(&entry.appearance_id))
    })
}

async fn wait_damage(client: &mut DevClientSession, before: u32) -> TestResult<u32> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(6);
    loop {
        client.service_liveness(Duration::from_millis(100)).await?;
        for event in client.take_events() {
            if let SessionEvent::ActorVitals(delta) = event
                && delta.value.health < before
            {
                return Ok(before - delta.value.health);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("timeout: no ActorVitals damage delta; vitals={:?}", client.actor_vitals()).into());
        }
    }
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0x9a).await?;
    let start = tile(client.world_spatial().actor_position)?;
    let mut path = route(world, start, |at| is_underwater(world, at))
        .ok_or("no reachable authored underwater ground in player movement window")?;
    let enter = path.pop().ok_or("character already starts underwater; entry STEP unavailable")?;
    walk(&mut client, world, &path).await?;
    row.pass(1, "ordinary STEP route reached underwater approach");

    let before = client.actor_vitals().copied().ok_or("entry vitals snapshot absent")?;
    let outcome = client.step_retrying(enter).await?;
    if outcome.disposition != StepDisposition::Moved || outcome.world_spatial_delta.is_none() {
        return Err(format!("underwater STEP reply={outcome:?}").into());
    }
    let wet = tile(client.world_spatial().actor_position)?;
    if !is_underwater(world, wet) {
        return Err(format!("STEP moved to non-underwater tile {wet:?}").into());
    }
    let damage = wait_damage(&mut client, before.health).await?;
    if damage != 20 {
        return Err(format!("first drown ActorVitals delta damage={damage}, expected=20").into());
    }
    row.pass(2, "moved-with-spatial-delta drown-damage=20");

    let (blocked, direction) = [
        StepDirection::North,
        StepDirection::East,
        StepDirection::South,
        StepDirection::West,
    ]
    .into_iter()
    .find_map(|direction| destination(world, wet, direction).is_none().then_some((wet, direction)))
    .ok_or("underwater tile has no blocked neighbour for refusal STEP")?;
    let health = client.actor_vitals().ok_or("refusal vitals absent")?.health;
    let refused = client.step_retrying(direction).await?;
    if refused.disposition == StepDisposition::Moved
        || refused.world_spatial_delta.is_some()
        || tile(client.world_spatial().actor_position)? != blocked
    {
        return Err(format!("blocked STEP was not side-effect-free: {refused:?}").into());
    }
    let damage = wait_damage(&mut client, health).await?;
    if damage != 20 {
        return Err(format!("post-refusal drown delta damage={damage}, expected=20").into());
    }
    row.pass(3, "blocked STEP refused without movement; drowning continued");

    let exit = [StepDirection::North, StepDirection::East, StepDirection::South, StepDirection::West]
        .into_iter()
        .find(|direction| destination(world, wet, *direction).is_some_and(|at| !is_underwater(world, at)))
        .ok_or("underwater tile has no ordinary dry STEP exit")?;
    let left = client.step_retrying(exit).await?;
    if left.disposition != StepDisposition::Moved || left.world_spatial_delta.is_none() {
        return Err(format!("dry exit STEP reply={left:?}").into());
    }
    let saved_position = client.world_spatial().actor_position;
    let saved_vitals = *client.actor_vitals().ok_or("dry exit vitals absent")?;
    let deadline = tokio::time::Instant::now() + Duration::from_millis(4500);
    while tokio::time::Instant::now() < deadline {
        client.service_liveness(Duration::from_millis(100)).await?;
        if client.take_events().into_iter().any(|event| matches!(event, SessionEvent::ActorVitals(delta) if delta.value.health < saved_vitals.health)) {
            return Err("drown ActorVitals damage continued after dry exit".into());
        }
    }
    row.pass(4, "dry STEP moved and removed periodic drowning");

    let old_session = client.join_snapshot().game_session_id;
    if client.session_mut().logout().await?.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err("logout after dry exit was refused".into());
    }
    client = connect_world(clients, 0x9b).await?;
    if client.join_snapshot().game_session_id == old_session
        || client.world_spatial().actor_position != saved_position
        || client.actor_vitals().is_none_or(|v| v.health != saved_vitals.health)
    {
        return Err(format!("fresh admission lost durable position/health: position={:?} vitals={:?}", client.world_spatial().actor_position, client.actor_vitals()).into());
    }
    row.pass(5, "fresh admission retained dry position and damaged health");
    let _ = client.session_mut().logout().await?;
    Ok(())
}
