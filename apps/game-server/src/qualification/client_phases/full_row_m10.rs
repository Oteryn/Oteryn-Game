//! E2E M10 — native drowning, surface swimming, spike and pressure-tile journey.
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
//! 6. Swim from a dry shore and back out. Query the permanent outfit condition.
//! 7. Trigger a reachable spike with enough health to survive its maximum damage.
//! 8. Step on and off a pressure tile and verify both map appearances.
//!
//! This journey uses only native player movement/logout commands.  It never
//! manufactures equipment, tiles, health, or character state.
use super::*;
use crate::map::view::MapFacts;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::world_map::MapOrigin;
use std::collections::BTreeSet;

const UNDERWATER: [u16; 12] = [
    5404, 5405, 5406, 5407, 5408, 5409, 5743, 5764, 8755, 8756, 8757, 9291,
];

// Crystal swimming.lua: 1–3, 20–37, as implemented by special_movement.rs:23.
const SWIMMING: [u16; 12] = [
    629, 630, 631, 632, 633, 634, 4809, 4810, 4811, 4812, 4813, 4814,
];
const DIRECTIONS: [StepDirection; 4] = [
    StepDirection::North,
    StepDirection::East,
    StepDirection::South,
    StepDirection::West,
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
        let gap = gap_detail(self.step, self.error.as_deref());
        evidence(&format!("M10_GAP {gap}"));
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=M10.full status=FAIL seconds={:.3} evidence=see-M10_GAP",
            self.started.elapsed().as_secs_f64(),
        ));
    }
}

fn gap_detail(step: usize, error: Option<&str>) -> String {
    if let Some(error) = error {
        return format!("step={} observed={error}", step + 1);
    }
    if step != 8 {
        return format!("step={} observed=incomplete-journey", step + 1);
    }
    "step=swimming-look-type missing=served-look-type-267 condition-status exposes outfit icon and duration only owner=crates/gameplay/dispatch/src/gameplay_transport/world_spatial.rs:174".to_owned()
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

fn has_ground(world: &BundleWorld, at: TilePos, appearances: &[u16]) -> bool {
    world.base().tile(at.x, at.y, at.floor).is_some_and(|tile| {
        tile.ground()
            .and_then(|(ordinal, id)| world.facts().base_entry(at, ordinal, id))
            .is_some_and(|entry| appearances.contains(&entry.appearance_id))
    })
}

fn is_underwater(world: &BundleWorld, at: TilePos) -> bool {
    has_ground(world, at, &UNDERWATER)
}

async fn swimming_condition(client: &mut DevClientSession) -> TestResult<bool> {
    use oteryn_protocol_oteryn::condition_status::{
        CAPABILITY_CONDITION_STATUS_V1, COMMAND_TYPE_CONDITION_STATUS_QUERY, ConditionStatus,
    };
    let reply = client.session_mut().gated_command(
        CAPABILITY_CONDITION_STATUS_V1,
        COMMAND_TYPE_CONDITION_STATUS_QUERY,
        &[],
    ).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("surface-swimming condition query status={:?}", reply.status).into());
    }
    let status = ConditionStatus::decode(&reply.payload)
        .ok_or("surface-swimming malformed condition status")?;
    Ok(status.conditions.iter().any(|condition| condition.icon == 22 && condition.remaining_ms == -1))
}

async fn surface_swimming(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    let start = tile(client.world_spatial().actor_position)?;
    let wet = |at| has_ground(world, at, &SWIMMING);
    let dry = |at| !wet(at) && !is_underwater(world, at);
    let goal = |at| wet(at) && DIRECTIONS.iter().any(|direction|
        destination(world, at, *direction).is_some_and(dry)
    );
    let mut path = route_avoiding(world, start, goal, |at| dry(at) || goal(at))
        .ok_or("step=surface-swimming missing=reachable-swimming-ground-with-dry-exit special_movement.rs:23")?;
    let enter = path.pop().ok_or("step=surface-swimming observed=already-on-swimming-ground")?;
    walk(client, world, &path).await?;
    if swimming_condition(client).await? {
        return Err("step=surface-swimming observed=permanent-outfit-before-entry".into());
    }
    walk(client, world, &[enter]).await?;
    let arrived = tile(client.world_spatial().actor_position)?;
    if !wet(arrived) || !swimming_condition(client).await? {
        return Err("step=surface-swimming observed=no-permanent-outfit-after-entry".into());
    }
    let exit = DIRECTIONS.into_iter().find(|direction|
        destination(world, arrived, *direction).is_some_and(dry)
    ).ok_or("step=surface-swimming observed=no-dry-exit-after-entry")?;
    walk(client, world, &[exit]).await?;
    if swimming_condition(client).await? {
        return Err("step=surface-swimming observed=permanent-outfit-after-dry-exit".into());
    }
    Ok(())
}

fn placement_matches(origin: MapOrigin, ordinal: u8, actual: u16, expected: u16) -> bool {
    actual == expected && matches!(origin, MapOrigin::BaseOrdinal { ordinal: base, .. } if base == ordinal)
}

async fn visible_appearance(
    client: &mut DevClientSession,
    at: TilePos,
    ordinal: u8,
    expected: u16,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        let snapshot = client.session_mut().world_map_snapshot();
        if snapshot.is_some_and(|snapshot| snapshot.tiles.iter().any(|entry| {
            tile(entry.position).ok() == Some(at)
                && entry.items.iter().any(|item| placement_matches(item.origin, ordinal, item.appearance_id, expected))
        })) {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step=movement-tile-map-update position={at:?} ordinal={ordinal} missing=appearance-{expected}").into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}

#[derive(Clone, Copy)]
struct StepTile {
    idle: u16,
    occupied: u16,
    damage: Option<(u32, u32)>,
}

// Pinned Crystal trap.lua and special_tiles.lua as mounted by special_movement.rs.
const SPIKES: [StepTile; 3] = [
    StepTile { idle: 2145, occupied: 2146, damage: Some((50, 100)) },
    StepTile { idle: 2147, occupied: 2148, damage: Some((50, 100)) },
    StepTile { idle: 3944, occupied: 3945, damage: Some((15, 30)) },
];
const PRESSURE: [StepTile; 6] = [
    StepTile { idle: 419, occupied: 420, damage: None },
    StepTile { idle: 431, occupied: 430, damage: None },
    StepTile { idle: 452, occupied: 453, damage: None },
    StepTile { idle: 563, occupied: 564, damage: None },
    StepTile { idle: 549, occupied: 562, damage: None },
    StepTile { idle: 10145, occupied: 10146, damage: None },
];

async fn observe_step_tile(
    client: &mut DevClientSession,
    world: &BundleWorld,
    bytes: &[u8],
    rules: &[StepTile],
    name: &str,
) -> TestResult {
    let mut placements = BTreeMap::new();
    let mut spikes = BTreeSet::new();
    let start = tile(client.world_spatial().actor_position)?;
    oteryn_world_bundle::bundle::visit(bytes, |manifest, sector| {
        if start.floor.abs_diff(sector.floor) > 2 {
            return Ok(());
        }
        for placement in sector.tiles {
            if start.x.abs_diff(placement.x) > 160 || start.y.abs_diff(placement.y) > 160 {
                continue;
            }
            let at = TilePos { x: placement.x, y: placement.y, floor: sector.floor };
            for (ordinal, item) in placement.items.iter().filter(|item| item.depth == 0).enumerate() {
                let Some(id) = crate::map::palette_appearance(&manifest.palette[item.palette as usize].key) else {
                    continue;
                };
                if SPIKES.iter().any(|rule| rule.idle == id) {
                    spikes.insert(at);
                }
                if let Some(rule) = rules.iter().find(|rule| rule.idle == id)
                    && let Ok(ordinal) = u8::try_from(ordinal)
                {
                    placements.insert(at, (*rule, ordinal));
                }
            }
        }
        Ok(())
    })?;
    let dry = |at| !is_underwater(world, at) && !has_ground(world, at, &SWIMMING);
    let safe_exit = |at, direction| destination(world, at, direction)
        .is_some_and(|next| dry(next) && !placements.contains_key(&next) && !spikes.contains(&next));
    let goal = |at: TilePos| {
        placements.get(&at).is_some_and(|(rule, _)| {
            rule.damage.is_none() || world.base().tile(at.x, at.y, at.floor)
                .is_some_and(|tile| !tile.protection_zone())
        }) && DIRECTIONS.iter().any(|direction| safe_exit(at, *direction))
    };
    let mut path = route_avoiding(world, start, goal, |at| {
        dry(at) && ((!placements.contains_key(&at) && !spikes.contains(&at)) || goal(at))
    }).ok_or_else(|| format!("step={name}-route missing=reachable-authored-placement-with-safe-exit owner=crates/gameplay/dispatch/src/gameplay_transport/paced_movement/special_movement.rs:409"))?;
    let enter = path.pop().ok_or_else(|| format!("step={name}-route observed=already-on-placement"))?;
    walk(client, world, &path).await?;
    let health = client.actor_vitals().ok_or("step=tile-entry missing=pre-entry-vitals")?.health;
    let landing = destination(world, tile(client.world_spatial().actor_position)?, enter)
        .ok_or("step=tile-entry missing=resolved-landing")?;
    let &(rule, ordinal) = placements.get(&landing).ok_or("step=tile-entry observed=wrong-landing")?;
    if let Some((_, maximum)) = rule.damage
        && health <= maximum
    {
        return Err(format!("step={name}-entry missing=health-above-{maximum} observed={health}").into());
    }
    visible_appearance(client, landing, ordinal, rule.idle).await?;
    client.take_events();
    let moved = client.step_retrying(enter).await?;
    if moved.disposition != StepDisposition::Moved
        || moved.world_spatial_delta.is_none()
        || tile(client.world_spatial().actor_position)? != landing
    {
        return Err(format!("step={name}-entry observed={moved:?} expected={landing:?}").into());
    }
    if let Some((minimum, maximum)) = rule.damage {
        let damage = wait_damage(client, health).await?;
        if !(minimum..=maximum).contains(&damage) {
            return Err(format!("step={name}-entry observed=damage-{damage} expected={minimum}..={maximum}").into());
        }
    }
    visible_appearance(client, landing, ordinal, rule.occupied).await?;
    let exit = DIRECTIONS.into_iter().find(|direction| safe_exit(landing, *direction))
        .ok_or_else(|| format!("step={name}-exit missing=safe-exit"))?;
    walk(client, world, &[exit]).await?;
    visible_appearance(client, landing, ordinal, rule.idle).await
}

async fn wait_damage(client: &mut DevClientSession, before: u32) -> TestResult<u32> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(6);
    loop {
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
        client.service_liveness(Duration::from_millis(100)).await?;
    }
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes.clone(),
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0x9a).await?;
    let start = tile(client.world_spatial().actor_position)?;

    let test_tile = |at| {
        is_underwater(world, at)
            && DIRECTIONS.iter().any(|direction| destination(world, at, *direction).is_none())
            && DIRECTIONS.iter().any(|direction| {
                destination(world, at, *direction).is_some_and(|next| !is_underwater(world, next))
            })
    };
    // Any wet tile admitted by the filter is itself a goal. No earlier STEP
    // may start the drowning timer.
    let mut path = route_avoiding(world, start, test_tile, |at| {
        !is_underwater(world, at) || test_tile(at)
    }).ok_or("no reachable authored underwater tile with a blocked neighbour and dry exit")?;
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
    surface_swimming(&mut client, world).await?;
    row.pass(6, "surface-swimming STEP installed and removed a permanent outfit condition");
    observe_step_tile(&mut client, world, &bytes, &SPIKES, "spike").await?;
    row.pass(7, "spike STEP damaged actor and closed trap; exit reopened trap");
    observe_step_tile(&mut client, world, &bytes, &PRESSURE, "pressure-tile").await?;
    row.pass(8, "pressure tile changed on STEP entry and restored on exit");
    if logout_retrying_busy(&mut client).await?.0 != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err("step=cleanup observed=final-logout-refused".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn review_repair_category_only_swimming_evidence_cannot_pass() {
        assert!(gap_detail(8, None).contains("look-type-267"));
    }

    #[test]
    fn review_repair_appearance_on_another_ordinal_is_not_evidence() {
        let other = MapOrigin::BaseOrdinal { ordinal: 2, object_revision: 1 };
        assert!(!placement_matches(other, 1, 420, 420));
        assert!(placement_matches(other, 2, 420, 420));
        assert!(!placement_matches(other, 2, 419, 420));
    }


    #[test]
    fn verdict_requires_pressure_tile_after_the_trap_and_names_runtime_failure() {
        assert_eq!(gap_detail(7, None), "step=8 observed=incomplete-journey");
        assert_eq!(gap_detail(5, Some("missing outfit")), "step=6 observed=missing outfit");

    }
}
