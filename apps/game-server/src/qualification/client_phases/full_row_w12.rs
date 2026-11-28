//! W12 full-row player scenario on the production Cyclopedia routes.
//! Movement completes the automatically activated POIs. Full area discovery grants a
//! player speed contribution. Donations fund creature respawns for a future server save.
//! Read the server's speed contribution before and after donating, then after fresh
//! admission at the saved tile, alongside discovery and donation readback.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::cyclopedia as wire;
use std::collections::BTreeSet;

#[derive(Debug)]
struct Failure {
    step: u8,
    observed: String,
    source: Option<Box<dyn std::error::Error>>,
}

impl Failure {
    fn new(step: u8, observed: impl Into<String>) -> Box<Self> {
        Box::new(Self {
            step,
            observed: observed.into(),
            source: None,
        })
    }
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "step-{}:{}", self.step, self.observed)
    }
}

impl std::error::Error for Failure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_deref()
    }
}

fn intent(operation: u8, area: u32, amount: u32) -> [u8; wire::MAX_REQUEST_BYTES] {
    let mut bytes = [0; wire::MAX_REQUEST_BYTES];
    bytes[0] = operation;
    bytes[1..5].copy_from_slice(&area.to_le_bytes());
    bytes[5..9].copy_from_slice(&amount.to_le_bytes());
    bytes
}

async fn exchange(
    client: &mut DevClientSession,
    kind: u32,
    payload: &[u8],
) -> TestResult<oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse> {
    Ok(client
        .session_mut()
        .gated_command(wire::CAPABILITY_CYCLOPEDIA_MAP_V1, kind, payload)
        .await?)
}

async fn query(client: &mut DevClientSession, step: u8) -> Result<wire::MapResult, Box<Failure>> {
    let reply = exchange(client, wire::COMMAND_TYPE_CYCLOPEDIA_QUERY, &[])
        .await
        .map_err(|error| {
            verdict::record_error(verdict::IndexedError::as_error(&error));
            let mut failure = Failure::new(step, format!("query-error={error}"));
            failure.source = Some(error);
            failure
        })?;
    if reply.status != CommandStatus::Accepted {
        return Err(Failure::new(step, format!("query-reply={reply:?}")));
    }
    wire::decode_result(&reply.payload)
        .map_err(|error| Failure::new(step, format!("result-decode={error:?};reply={reply:?}")))
}

async fn discovery_speed(client: &mut DevClientSession, step: u8) -> Result<u16, Box<Failure>> {
    let reply = exchange(client, wire::COMMAND_TYPE_CYCLOPEDIA_SPEED_QUERY, &[])
        .await
        .map_err(|error| {
            verdict::record_error(verdict::IndexedError::as_error(&error));
            let mut failure = Failure::new(step, format!("speed-query={error}"));
            failure.source = Some(error);
            failure
        })?;
    match (reply.status, reply.payload.as_slice()) {
        (CommandStatus::Accepted, [low, high]) => Ok(u16::from_le_bytes([*low, *high])),
        _ => Err(Failure::new(step, format!("speed-reply={reply:?}"))),
    }
}

async fn action(
    client: &mut DevClientSession,
    step: u8,
    operation: u8,
    amount: u32,
) -> Result<wire::MapResult, Box<Failure>> {
    let reply = exchange(
        client,
        wire::COMMAND_TYPE_CYCLOPEDIA_INTENT,
        &intent(operation, 1, amount),
    )
    .await
    .map_err(|error| {
        verdict::record_error(verdict::IndexedError::as_error(&error));
        let mut failure = Failure::new(step, format!("command-error={error}"));
        failure.source = Some(error);
        failure
    })?;
    if reply.status != CommandStatus::Accepted {
        return Err(Failure::new(step, format!("command-reply={reply:?}")));
    }
    wire::decode_result(&reply.payload)
        .map_err(|error| Failure::new(step, format!("result-decode={error:?};reply={reply:?}")))
}

fn stable(before: &wire::MapResult, after: &wire::MapResult) -> bool {
    before.discovered == after.discovered
        && before.found == after.found
        && before.area_percent == after.area_percent
        && before.pool == after.pool
        && before.epoch == after.epoch
        && before.improved == after.improved
}

async fn logout(client: &mut DevClientSession, step: u8) -> Result<(), Box<Failure>> {
    let reply = client.session_mut().logout().await.map_err(|error| {
        verdict::record_error(verdict::IndexedError::as_error(&error));
        let mut failure = Failure::new(step, format!("logout-error={error}"));
        failure.source = Some(error.into());
        failure
    })?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(Failure::new(step, format!("logout-reply={reply:?}")));
    }
    Ok(())
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

fn membership(world: &BundleWorld, at: TilePos) -> Option<(u32, u32)> {
    world.base().subareas.resolve(at.x, at.y, at.floor)
}

async fn discover_area(
    client: &mut DevClientSession,
    world: &BundleWorld,
    area: u32,
    mut state: wire::MapResult,
) -> TestResult {
    let mut surveyed = BTreeSet::<(u32, TilePos)>::new();
    let mut completed = BTreeSet::new();
    while state.area_percent < 100 {
        let from = tile(client.world_spatial().actor_position)?;
        let (parent, subarea) = membership(world, from).ok_or("missing discovery membership")?;
        if parent != area {
            return Err(format!("discovery walk left area {area} at {from:?}").into());
        }
        if state.discovered {
            completed.insert(subarea);
        }
        surveyed.insert((subarea, from));

        let path = state.visible.iter().find_map(|&(x, y, floor)| {
            route(world, from, |at| {
                at == TilePos {
                    x,
                    y,
                    floor: -floor,
                }
            })
            .filter(|path| !path.is_empty())
        });
        let walked_to_poi = path.is_some();
        // The server prefers hidden POIs. Visit a fresh viewport in an unfinished
        // subarea when no revealed point is reachable, then query from that tile.
        let unfinished = |at| {
            membership(world, at).filter(|&(parent, child)| {
                parent == area && !completed.contains(&child) && !surveyed.contains(&(child, at))
            })
        };
        let path = path
            .or_else(|| {
                route(world, from, |at| {
                    unfinished(at).is_some_and(|(_, child)| {
                        !surveyed.iter().any(|&(seen_child, seen)| {
                            seen_child == child
                                && seen.floor == at.floor
                                && seen.x.abs_diff(at.x) <= 4
                                && seen.y.abs_diff(at.y) <= 3
                        })
                    })
                })
            })
            .or_else(|| route(world, from, |at| unfinished(at).is_some()));
        let path =
            path.ok_or_else(|| format!("no-reachable-poi-or-new-viewport;state={state:?}"))?;
        walk(client, world, &path).await?;
        let next = query(client, 4).await?;
        if next.code != 0 || (walked_to_poi && next.found <= state.found) {
            return Err(format!("discovery walk failed: before={state:?};after={next:?}").into());
        }
        state = next;
    }
    Ok(())
}

async fn paced_probe(
    client: &mut DevClientSession,
    direction: StepDirection,
    expected: ActorPosition,
    step: u8,
) -> Result<u32, Box<Failure>> {
    let outcome = client.step_retrying(direction).await.map_err(|error| {
        verdict::record_error(verdict::IndexedError::as_error(&error));
        let mut failure = Failure::new(step, format!("paced-step={error}"));
        failure.source = Some(error.into());
        failure
    })?;
    if outcome.status != CommandStatus::Accepted
        || outcome.disposition != StepDisposition::Moved
        || outcome.step_wait_ms == 0
        || !outcome
            .world_spatial_delta
            .is_some_and(|delta| delta.value.actor_position == expected)
    {
        return Err(Failure::new(step, "expected-paced-step-and-position-delta"));
    }
    Ok(outcome.step_wait_ms)
}

async fn journey(clients: &SeamClients<'_>) -> Result<(), Box<Failure>> {
    let world = qualification_world(clients).map_err(|error| Failure::new(1, error.to_string()))?;
    let mut client = connect_world(clients, 0xb8).await.map_err(|error| {
        verdict::record_error(verdict::IndexedError::as_error(&error));
        let mut failure = Failure::new(1, format!("admission={error}"));
        failure.source = Some(error);
        failure
    })?;
    let first_session = client.join_snapshot().game_session_id;
    let initial = query(&mut client, 1).await?;
    let initial_speed = discovery_speed(&mut client, 1).await?;
    if initial.code != 0 || initial.area_percent == 100 || initial_speed != 0 {
        return Err(Failure::new(
            1,
            format!(
                "expected-incomplete-area-and-zero-speed;state={initial:?};speed={initial_speed}"
            ),
        ));
    }

    let origin = tile(client.world_spatial().actor_position)
        .map_err(|error| Failure::new(1, error.to_string()))?;
    let (area, subarea) =
        membership(world, origin).ok_or_else(|| Failure::new(1, "missing entry subarea"))?;
    let (direction, reverse, neighbour) = [
        (StepDirection::North, StepDirection::South),
        (StepDirection::East, StepDirection::West),
        (StepDirection::South, StepDirection::North),
        (StepDirection::West, StepDirection::East),
    ]
    .into_iter()
    .find_map(|(direction, reverse)| {
        let next = destination(world, origin, direction)?;
        (next.floor == origin.floor
            && membership(world, next) == Some((area, subarea))
            && destination(world, next, reverse) == Some(origin))
        .then_some((direction, reverse, next))
    })
    .ok_or_else(|| Failure::new(1, "no reversible same-subarea pacing probe"))?;
    let basis =
        spatial_basis::current(&mut client).map_err(|error| Failure::new(1, error.to_string()))?;
    let baseline_wait = paced_probe(&mut client, direction, basis.position(neighbour), 1).await?;

    let progress = query(&mut client, 4).await?;
    discover_area(&mut client, world, area, progress)
        .await
        .map_err(|error| Failure::new(4, error.to_string()))?;
    let from = tile(client.world_spatial().actor_position)
        .map_err(|error| Failure::new(4, error.to_string()))?;
    let return_path = route(world, from, |at| at == origin)
        .ok_or_else(|| Failure::new(4, "no return route to pacing probe"))?;
    walk(&mut client, world, &return_path)
        .await
        .map_err(|error| Failure::new(4, error.to_string()))?;
    let completed_wait = paced_probe(&mut client, direction, basis.position(neighbour), 4).await?;
    if completed_wait >= baseline_wait {
        return Err(Failure::new(
            4,
            format!("discovery-did-not-reduce-pace;before={baseline_wait};after={completed_wait}"),
        ));
    }
    paced_probe(&mut client, reverse, basis.position(origin), 4).await?;
    let rules = oteryn_gameplay_domain::domain::cyclopedia::DiscoverySpeed::embedded()
        .ok_or_else(|| Failure::new(4, "missing discovery speed rules"))?;
    let speed = discovery_speed(&mut client, 4).await?;
    if !(rules.bonus(1, 2, true)..=rules.bonus(2, 2, true)).contains(&speed) {
        return Err(Failure::new(
            4,
            format!("completed-area-speed-out-of-range={speed}"),
        ));
    }

    let before_donation = query(&mut client, 5).await?;
    let donated = action(&mut client, 5, 4, 1).await?;
    if !((donated.code == 0 && donated.pool == before_donation.pool + 1)
        || (donated.code == 6 && stable(&before_donation, &donated)))
    {
        return Err(Failure::new(5, format!("one-gold-donation={donated:?}")));
    }
    let goal_amount = 10_000_000;
    let goal = action(&mut client, 5, 4, goal_amount).await?;
    if !((goal.code == 0
        && Some(goal.pool) == donated.pool.checked_add(u64::from(goal_amount))
        && goal.epoch == donated.epoch
        && goal.improved == donated.improved)
        || (goal.code == 6 && stable(&donated, &goal)))
    {
        return Err(Failure::new(5, format!("goal-donation={goal:?}")));
    }
    let donated_wait = paced_probe(&mut client, direction, basis.position(neighbour), 5).await?;
    paced_probe(&mut client, reverse, basis.position(origin), 5).await?;
    let donated_speed = discovery_speed(&mut client, 5).await?;
    if donated_speed != speed || donated_wait != completed_wait {
        return Err(Failure::new(
            5,
            format!(
                "donation-changed-player-movement;speed={speed}/{donated_speed};step_wait_ms={completed_wait}/{donated_wait}"
            ),
        ));
    }

    let malformed = exchange(
        &mut client,
        wire::COMMAND_TYPE_CYCLOPEDIA_INTENT,
        &intent(1, 2, 0),
    )
    .await
    .map_err(|error| {
        verdict::record_error(verdict::IndexedError::as_error(&error));
        let mut failure = Failure::new(6, format!("malformed-command-error={error}"));
        failure.source = Some(error);
        failure
    })?;
    if malformed.status != CommandStatus::Rejected || !malformed.payload.is_empty() {
        return Err(Failure::new(6, format!("malformed-reply={malformed:?}")));
    }
    let retained = query(&mut client, 6).await?;
    if !stable(&goal, &retained) {
        return Err(Failure::new(
            6,
            format!("before={goal:?};after={retained:?}"),
        ));
    }
    let saved_position = client.world_spatial().actor_position;
    logout(&mut client, 7).await?;
    drop(client);

    let mut fresh = connect_world(clients, 0xb9).await.map_err(|error| {
        verdict::record_error(verdict::IndexedError::as_error(&error));
        let mut failure = Failure::new(7, format!("fresh-admission={error}"));
        failure.source = Some(error);
        failure
    })?;
    if fresh.join_snapshot().game_session_id == first_session {
        return Err(Failure::new(7, "fresh-admission-reused-session"));
    }
    let observed_position = fresh.world_spatial().actor_position;
    if observed_position != saved_position {
        return Err(Failure::new(
            7,
            format!("saved-position={saved_position:?};fresh-position={observed_position:?}"),
        ));
    }
    // Observe STEP before any Cyclopedia query could refresh the live contribution.
    let fresh_wait = paced_probe(&mut fresh, direction, basis.position(neighbour), 7).await?;
    paced_probe(&mut fresh, reverse, basis.position(origin), 7).await?;
    let after = query(&mut fresh, 7).await?;
    let fresh_speed = discovery_speed(&mut fresh, 7).await?;
    logout(&mut fresh, 7).await?;
    if !stable(&retained, &after) {
        return Err(Failure::new(
            7,
            format!(
                "saved-position={saved_position:?};fresh-position={observed_position:?};before={retained:?};after={after:?}"
            ),
        ));
    }
    if fresh_speed != speed || fresh_wait != completed_wait {
        return Err(Failure::new(
            7,
            format!(
                "movement-changed-on-relog;speed={speed}/{fresh_speed};step_wait_ms={completed_wait}/{fresh_wait}"
            ),
        ));
    }
    evidence(&format!(
        "W12_SPEED step=7 discovery_speed_initial={initial_speed} discovery_speed_completed={speed} discovery_speed_after_donation={donated_speed} discovery_speed_relog={fresh_speed} step_wait_ms_initial={baseline_wait} step_wait_ms_completed={completed_wait} step_wait_ms_after_donation={donated_wait} step_wait_ms_relog={fresh_wait}"
    ));
    if goal.code != 0 {
        return Err(Failure::new(5, format!("goal-donation-refused={goal:?}")));
    }
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(360), verdict::observe(journey(clients)))
            .await;
    match result {
        Ok(Ok(())) => evidence(&format!(
            "e2e_all scenario=W12.full status=PASS seconds={:.3}",
            started.elapsed().as_secs_f64()
        )),
        Ok(Err(error)) => {
            evidence(&format!(
                "W12_CHECK step={} FAIL observed={}",
                error.step, error.observed
            ));
            evidence(&format!(
                "e2e_all scenario=W12.full status=FAIL seconds={:.3}",
                started.elapsed().as_secs_f64(),
            ));
        }
        Err(_) => evidence(&format!(
            "e2e_all scenario=W12.full status=FAIL seconds={:.3} evidence=step-timeout",
            started.elapsed().as_secs_f64()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intent_encoder_and_durable_comparison_cover_the_wire_facts() {
        assert_eq!(
            wire::decode_request(wire::COMMAND_TYPE_CYCLOPEDIA_INTENT, &intent(4, 1, 7)),
            Ok(wire::MapIntent {
                operation: 4,
                amount: 7
            })
        );
        assert!(
            wire::decode_request(wire::COMMAND_TYPE_CYCLOPEDIA_INTENT, &intent(1, 2, 0)).is_err()
        );
        let state = wire::MapResult {
            code: 0,
            revision: 1,
            discovered: false,
            found: 0,
            area_percent: 0,
            pool: 0,
            epoch: 0,
            improved: false,
            visible: Vec::new(),
        };
        let mut refusal = state.clone();
        refusal.code = 4;
        refusal.revision = 2;
        assert!(stable(&state, &refusal));
        refusal.pool = 1;
        assert!(!stable(&state, &refusal));
    }
}
