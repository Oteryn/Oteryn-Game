//! P22d (live tracker refresh on kill progress):
//! 1. Admit the node-boot character, track a race and note its kill count.
//! 2. Walk to a compiled spawn of that race, kill one creature and wait for the live frames.
//! 3. Verify the pushed tracker update and read the same kill count after logout and relog.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind, SessionEvent};
use oteryn_protocol_oteryn::attack::{AttackIntentDisposition, ChaseMode, FightModes};
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
    walk_path(client, world, path, |client| {
        let here = tile(client.world_spatial().actor_position).ok()?;
        world
            .base()
            .tile(here.x, here.y, here.floor)
            .filter(|tile| !tile.protection_zone())?;
        adjacent_rat(client)
    })
    .await?
    .ok_or_else(|| "step=hunt;observed=no-adjacent-rat".into())
}

async fn walk_path<T>(
    client: &mut DevClientSession,
    world: &BundleWorld,
    path: Vec<StepDirection>,
    stop_when: impl Fn(&DevClientSession) -> Option<T>,
) -> TestResult<Option<T>> {
    let mut path = VecDeque::from(path);
    let mut detours = std::collections::BTreeSet::new();
    while let Some(direction) = path.pop_front() {
        if let Some(target) = stop_when(client) {
            return Ok(Some(target));
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
    Ok(stop_when(client))
}

async fn prepare_hunter(clients: &SeamClients<'_>) -> TestResult<DevClientSession> {
    character_prep::declare_seeded(
        "P22d.full",
        "sorcerer-level-100",
        "the authored rat route and combat-lock wait exceed the ordinary character budget",
    );
    let mut client = connect_world(clients, 0xd5).await?;
    selected(&client)?;
    let old = client.join_snapshot().game_session_id;
    request_fixture_control(uuid_text(old.as_bytes()), "--profile", "sorcerer".into()).await?;
    logout(&mut client).await?;
    let mut fresh = connect_world(clients, 0xd6).await?;
    selected(&fresh)?;
    expect(
        fresh.join_snapshot().game_session_id != old,
        "seeded-relog",
        "reused-session",
    )?;
    let level = full_e2ef7::player_level(&mut fresh).await?;
    let vitals = fresh
        .actor_vitals()
        .ok_or("step=seeded-stats;observed=no-vitals")?;
    expect(
        level == 100 && vitals.max_health == 645 && vitals.max_mana == 2850,
        "seeded-stats",
        (level, vitals.max_health, vitals.max_mana),
    )?;
    Ok(fresh)
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
    let mut client = prepare_hunter(clients).await?;
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

    let modes = FightModes {
        chase: ChaseMode::Chase,
        ..FightModes::DEFAULT
    };
    let prior_combat = client.session_mut().combat_state().copied();
    expect(
        prior_combat
            .is_some_and(|state| state.modes == FightModes::DEFAULT && state.target.is_none()),
        "combat-pre-state",
        prior_combat,
    )?;
    let mode = client.session_mut().set_fight_modes(&modes).await?;
    expect(
        mode.disposition == AttackIntentDisposition::Ok,
        "chase-mode",
        mode,
    )?;
    let target = walk_to_rat(&mut client, world, &metadata.spawns).await?;
    evidence(&format!(
        "p22d target={:?} position={:?}",
        target.entity, target.position
    ));
    let prior_corpses: std::collections::BTreeSet<_> = client
        .world_entities()
        .into_iter()
        .flat_map(|entities| entities.iter())
        .filter(|entity| entity.kind == EntityKind::Corpse)
        .map(|entity| entity.entity)
        .collect();
    let mut last_target_position = target.position;
    let mut corpse = None;
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

    client.service_liveness(Duration::from_millis(25)).await?;
    let combat = client.session_mut().combat_state().copied();
    expect(
        combat.is_some_and(|state| state.target == Some(target.entity) && state.modes == modes),
        "attack-target-state",
        combat,
    )?;
    let spell = client_spell_journey::spell_index(native_spell_book()?, "exori vis")?;
    let mana = client
        .actor_vitals()
        .ok_or("step=energy-strike;observed=no-vitals")?
        .mana;
    let expected_mana = mana
        .checked_sub(20)
        .ok_or("step=energy-strike;observed=missing-mana")?;
    let cast = client
        .cast_spell(
            spell,
            oteryn_protocol_oteryn::actor_spell::SpellTarget::AttackTarget,
            true,
        )
        .await?;
    expect(
        cast.disposition == oteryn_protocol_oteryn::actor_spell::SpellCastDisposition::Cast
            && cast
                .actor_vitals_delta
                .as_ref()
                .is_some_and(|delta| delta.value.mana == expected_mana),
        "energy-strike",
        (expected_mana, cast),
    )?;

    let deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    let mut refreshed = false;
    while tokio::time::Instant::now() < deadline {
        for event in client.take_events() {
            if let SessionEvent::WorldSpatialEntities(delta) = &event {
                if let Some(live) = delta
                    .value
                    .update
                    .iter()
                    .chain(&delta.value.enter)
                    .find(|entity| entity.entity == target.entity)
                {
                    last_target_position = live.position;
                }
                if corpse.is_none()
                    && client
                        .world_entities()
                        .is_some_and(|entities| entities.get(&target.entity).is_none())
                    && let Some(dead) = delta.value.enter.iter().find(|entity| {
                        entity.kind == EntityKind::Corpse
                            && entity.position == last_target_position
                            && !prior_corpses.contains(&entity.entity)
                    })
                {
                    evidence(&format!(
                        "p22d kill target={:?} corpse={:?} sequence={}",
                        target.entity, dead.entity, delta.server_sequence
                    ));
                    corpse = Some(dead.entity);
                    verdict::Harness::global().progress("p22d-kill");
                }
            }
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
        if refreshed && corpse.is_some() {
            break;
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    let surviving_target = client
        .world_entities()
        .and_then(|entities| entities.get(&target.entity));
    expect(
        corpse.is_some(),
        "kill",
        (
            target.entity,
            last_target_position,
            surviving_target.map(|entity| entity.detail),
        ),
    )?;
    expect(
        refreshed,
        "live-refresh",
        "confirmed-kill-without-domain-98-push",
    )?;
    leave_combat(&mut client, world).await?;
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

async fn leave_combat(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    let here = tile(client.world_spatial().actor_position)?;
    let creatures: Vec<_> = client
        .world_entities()
        .ok_or("step=leave-combat;observed=no-entities")?
        .iter()
        .filter(|entry| entry.kind == EntityKind::Creature)
        .map(|entry| entry.entity)
        .collect();
    let path = route(world, here, |at| {
        at.floor == here.floor && u32::from(at.x) + 25 <= u32::from(here.x)
    })
    .ok_or("step=leave-combat;observed=no-westward-retreat")?;
    evidence(&format!(
        "p22d retreat from={here:?} creatures={creatures:?} steps={}",
        path.len()
    ));
    let retreated = walk_path(client, world, path, |client| {
        let position = tile(client.world_spatial().actor_position).ok()?;
        let view = client.world_entities()?;
        (u32::from(position.x) + 15 <= u32::from(here.x)
            && creatures.iter().all(|entity| view.get(entity).is_none()))
        .then_some(position)
    })
    .await?;
    let view = client
        .world_entities()
        .ok_or("step=leave-combat;observed=no-entities")?;
    let remaining: Vec<_> = creatures
        .iter()
        .filter_map(|entity| view.get(entity))
        .map(|entry| (entry.entity, entry.position))
        .collect();
    expect(retreated.is_some(), "leave-combat", remaining)?;
    // PvE combat can remain locked for up to 60 seconds after the last hit.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    verdict::Harness::global().bounded_wait(deadline, "p22d-combat-lock");
    while client
        .session_mut()
        .combat_state()
        .is_none_or(|state| state.in_fight)
    {
        let remaining = deadline
            .checked_duration_since(tokio::time::Instant::now())
            .ok_or("step=combat-lock;observed=still-in-fight")?;
        client
            .service_liveness(remaining.min(Duration::from_secs(1)))
            .await?;
        client.take_events();
    }
    Ok(())
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
