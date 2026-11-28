//! P22d (live tracker refresh on kill progress):
//! 1. Admit the node-boot character, track a race and note its kill count.
//! 2. Walk to a compiled spawn of that race, kill one creature and wait for the live frames.
//! 3. Verify the pushed tracker update and read the same kill count after logout and relog.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind, SessionEvent};
use oteryn_protocol_oteryn::attack::AttackIntentDisposition;
use oteryn_protocol_oteryn::{CommandStatus, cyclopedia_tracker as tr};

const RAT: &str = "oteryn:creature.rat";

fn selected(client: &DevClientSession) -> TestResult {
    expect(
        client
            .selected_capabilities()
            .contains(&tr::CAPABILITY_CYCLOPEDIA_TRACKER_V1),
        "capability",
        "needed=capability-197-selected-at-join",
    )
}

/// One tracker command; the reply snapshot is `None` for an empty refusal payload.
async fn call(
    client: &mut DevClientSession,
    command: u32,
    request: &tr::TrackerRequest,
) -> TestResult<(CommandStatus, Option<tr::TrackerSnapshot>)> {
    let payload =
        tr::encode_request(request).ok_or("step=encode;observed=request-not-encodable")?;
    let reply = client
        .session_mut()
        .gated_command(tr::CAPABILITY_CYCLOPEDIA_TRACKER_V1, command, &payload)
        .await?;
    let snapshot = if reply.payload.is_empty() {
        None
    } else {
        Some(
            tr::decode_snapshot(&reply.payload)
                .ok_or("step=decode;observed=undecodable-tracker-snapshot")?,
        )
    };
    Ok((reply.status, snapshot))
}

fn change(action: tr::TrackerAction, key: &str) -> tr::TrackerRequest {
    tr::TrackerRequest::Change {
        action,
        creature_key: key.to_owned(),
    }
}

fn has(snapshot: &Option<tr::TrackerSnapshot>, key: &str) -> bool {
    snapshot
        .as_ref()
        .is_some_and(|list| list.entries.iter().any(|entry| entry.creature_key == key))
}

async fn read(
    client: &mut DevClientSession,
    command: u32,
) -> TestResult<Option<tr::TrackerSnapshot>> {
    let (status, list) = call(client, command, &tr::TrackerRequest::Read).await?;
    expect(status == CommandStatus::Accepted, "read", status)?;
    Ok(list)
}

fn kills(snapshot: &tr::TrackerSnapshot, key: &str) -> Option<u32> {
    snapshot
        .entries
        .iter()
        .find(|entry| entry.creature_key == key)
        .map(|entry| entry.kill_count)
}

fn adjacent_rat(client: &DevClientSession) -> Option<oteryn_dev_client::WorldSpatialEntity> {
    let here = client.world_spatial().actor_position;
    client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entry| {
                entry.kind == EntityKind::Creature
                    && entry
                        .display_name
                        .is_some_and(|name| name.as_str() == Ok("Rat"))
                    && entry.position.floor == here.floor
                    && entry
                        .position
                        .x
                        .abs_diff(here.x)
                        .max(entry.position.y.abs_diff(here.y))
                        <= 1
            })
        })
        .copied()
}

async fn walk_to_rat(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult<oteryn_dev_client::WorldSpatialEntity> {
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| !tile.protection_zone())
            && spawns.sources.iter().any(|source| {
                source.floor == at.floor
                    && source.points.iter().any(|point| {
                        at.x.abs_diff(point.x).max(at.y.abs_diff(point.y)) <= 1
                            && spawns
                                .creatures
                                .get(point.creature as usize)
                                .is_some_and(|entry| entry.key == RAT)
                    })
            })
    })
    .ok_or("step=route;observed=no-reachable-rat-spawn")?;
    let mut path = VecDeque::from(path);
    let mut detours = std::collections::BTreeSet::new();
    while let Some(direction) = path.pop_front() {
        let here = tile(client.world_spatial().actor_position)?;
        if world
            .base()
            .tile(here.x, here.y, here.floor)
            .is_some_and(|at| !at.protection_zone())
            && let Some(target) = adjacent_rat(client)
        {
            return Ok(target);
        }
        let before = tile(client.world_spatial().actor_position)?;
        let expected =
            destination(world, before, direction).ok_or("step=route;observed=no-landing")?;
        let mut outcome = client.step(direction).await?;
        if outcome.disposition == StepDisposition::Blocked {
            expect(
                tile(client.world_spatial().actor_position)? == before,
                "walk-blocked-position",
                outcome.command_id,
            )?;
            let blocker = client.world_entities().and_then(|entities| entities.iter().find(|entity| {
                entity.kind == EntityKind::Creature && tile(entity.position).ok() == Some(expected)
            })).map(|entity| entity.entity)
                .ok_or_else(|| format!("step=walk;observed=blocked-without-occupant;tile={expected:?};frame={outcome:?}"))?;
            evidence(&format!(
                "p22d blocked={expected:?} entity={blocker:?} frame={outcome:?}"
            ));
            client.service_liveness(Duration::from_millis(100)).await?;
            let occupied = client
                .world_entities()
                .and_then(|entities| entities.get(&blocker))
                .map(|entity| tile(entity.position))
                .transpose()?;
            if occupied == Some(expected)
                && detours.len() < 8
                && detours.insert(expected)
                && let Some(detour) = route_around_occupant(
                    before,
                    expected,
                    &path,
                    |at| !detours.contains(&at),
                    |at, step| destination(world, at, step),
                )
            {
                evidence(&format!(
                    "p22d detour entity={blocker:?} tile={expected:?} steps={}",
                    detour.len()
                ));
                path = detour.into();
                continue;
            }
            let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
            verdict::Harness::global().bounded_wait(deadline, "p22d-creature-step");
            loop {
                let position = client
                    .world_entities()
                    .and_then(|entities| entities.get(&blocker))
                    .map(|entity| tile(entity.position))
                    .transpose()?;
                if position != Some(expected) {
                    break;
                }
                expect(
                    tokio::time::Instant::now() < deadline,
                    "walk-blocker",
                    blocker,
                )?;
                client.service_liveness(Duration::from_millis(100)).await?;
                client.take_events();
            }
            outcome = client.step(direction).await?;
        }
        expect(
            outcome.disposition == StepDisposition::Moved
                && outcome.world_spatial_delta.is_some()
                && tile(client.world_spatial().actor_position)? == expected,
            "walk",
            (expected, &outcome),
        )?;
        verdict::Harness::global().progress("p22d-verified-step");
        client
            .service_liveness(Duration::from_millis(u64::from(outcome.step_wait_ms)))
            .await?;
        client.take_events();
    }
    adjacent_rat(client).ok_or_else(|| "step=hunt;observed=no-adjacent-rat".into())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<Verdict> {
    let best = tr::COMMAND_TYPE_BESTIARY_TRACKER;
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0xd6).await?;
    selected(&client)?;
    let initial = read(&mut client, best).await?;
    expect(
        initial
            .as_ref()
            .is_some_and(|list| !list.boss && kills(list, RAT).is_none()),
        "tracker-pre-state",
        initial,
    )?;
    let (status, list) = call(&mut client, best, &change(tr::TrackerAction::Add, RAT)).await?;
    expect(
        status == CommandStatus::Accepted && has(&list, RAT),
        "add",
        status,
    )?;
    let before = list
        .as_ref()
        .filter(|list| !list.boss)
        .and_then(|list| kills(list, RAT))
        .ok_or("step=add;observed=missing-rat-count")?;
    let expected = before
        .checked_add(1)
        .ok_or("step=add;observed=kill-count-overflow")?;

    let target = walk_to_rat(&mut client, world, &metadata.spawns).await?;
    evidence(&format!(
        "p22d target={:?} position={:?}",
        target.entity, target.position
    ));
    client.take_events();
    let attack = client
        .session_mut()
        .attack_target(Some(&target.entity))
        .await?;
    expect(
        attack.disposition == AttackIntentDisposition::Ok,
        "attack",
        attack.disposition,
    )?;

    let deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    let mut refreshed = false;
    while tokio::time::Instant::now() < deadline {
        for event in client.take_events() {
            let SessionEvent::GatedSnapshotReplaced(delta) = event else {
                continue;
            };
            if delta.value.domain_id != tr::STATE_DOMAIN_CYCLOPEDIA_TRACKER {
                continue;
            }
            let (bestiary, _) = tr::decode_domain_snapshot(&delta.value.payload)
                .ok_or("step=live-refresh;observed=undecodable-domain")?;
            let count = kills(&bestiary, RAT);
            expect(
                count == Some(expected),
                "live-refresh-count",
                (expected, count),
            )?;
            evidence(&format!(
                "p22d live_tracker sequence={} revision={} before={before} after={expected}",
                delta.server_sequence, delta.new_revision,
            ));
            verdict::Harness::global().progress("p22d-live-tracker");
            refreshed = true;
        }
        if refreshed
            && client
                .world_entities()
                .is_some_and(|entities| entities.get(&target.entity).is_none())
        {
            break;
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    expect(refreshed, "live-refresh", "no-domain-98-push")?;
    expect(
        client
            .world_entities()
            .is_some_and(|entities| entities.get(&target.entity).is_none()),
        "kill",
        "target-still-present",
    )?;
    // The PvE in-fight lock lasts 60 seconds after the last hit.
    let lock_duration = Duration::from_secs(60);
    let lock_deadline = tokio::time::Instant::now() + lock_duration;
    verdict::Harness::global().bounded_wait(lock_deadline, "p22d-combat-lock");
    while let Some(remaining) = lock_deadline.checked_duration_since(tokio::time::Instant::now()) {
        client
            .service_liveness(remaining.min(Duration::from_secs(1)))
            .await?;
        client.take_events();
    }
    let original_session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;

    let mut fresh = connect_world(clients, 0xd7).await?;
    selected(&fresh)?;
    expect(
        fresh.join_snapshot().game_session_id != original_session,
        "relog",
        "reused-session",
    )?;
    let list = read(&mut fresh, best).await?;
    let after = list
        .as_ref()
        .filter(|list| !list.boss)
        .and_then(|list| kills(list, RAT));
    expect(
        after == Some(expected),
        "durable-kill-count",
        (expected, after),
    )?;
    let (status, list) = call(&mut fresh, best, &change(tr::TrackerAction::Remove, RAT)).await?;
    expect(
        status == CommandStatus::Accepted
            && list
                .as_ref()
                .is_some_and(|list| !list.boss && kills(list, RAT).is_none()),
        "remove",
        (status, list),
    )?;
    logout(&mut fresh).await?;
    Ok((
        "PASS",
        format!("live-and-durable-kill-count:{before}->{expected}"),
    ))
}

type Verdict = (&'static str, String);

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(360), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = match result {
        Ok(Ok((status, detail))) => (status, token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=P22d.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

/// One evidence token: no whitespace, bounded length.
fn token(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || "-_.:/=<>,;".contains(character) {
                character
            } else {
                '-'
            }
        })
        .take(240)
        .collect()
}

fn expect(ok: bool, step: &str, observed: impl std::fmt::Debug) -> TestResult {
    if ok {
        Ok(())
    } else {
        Err(format!("step={step};observed={observed:?}").into())
    }
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    expect(
        reply.result == Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted),
        "logout",
        reply.result,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_has_no_whitespace_and_is_bounded() {
        let value = token(&format!("step=a b\tc\n{}", "x".repeat(400)));
        assert!(!value.contains(char::is_whitespace));
        assert!(value.len() <= 240);
        assert!(value.starts_with("step=a-b-c-"));
    }
}
