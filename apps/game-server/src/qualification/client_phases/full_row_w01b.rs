//! W01b — Monster spawn/respawn, derived from the matrix, Crystal reaudit, and
//! `world_respawns.rs`/the spawn owner named by those records.
//!
//! 1. Admit the provisioned node-boot character, load the exact served bundle, walk with native
//!    STEP commands to the authored `oteryn:creature.rat` point, and require its live entity.
//! 2. Send ATTACK_TARGET for that rat; require an accepted/Ok reply, health update(s), removal of
//!    that actor, and the corpse entity at its last position.
//! 3. Remain in player interest through the authored respawn interval. Require no early
//!    replacement, then a new Rat at the authored point during the non-blockable respawn.
//! 4. Walk outside interest with native STEP commands, wait the next authored interval, return,
//!    and require the same replacement generation at the authored point.
//! 5. Send LOGOUT and require Accepted, make a fresh admission, walk back to the point, and
//!    re-check the session-independent World state: the replacement remains live and the killed
//!    actor generation has not returned. Finish with another accepted LOGOUT.
use super::*;
use oteryn_dev_client::{
    DevClientSession, EntityDetail, EntityKind, EntityRef, SessionEvent, WorldSpatialEntity,
};

const ROW: &str = "W01b";
pub(super) const SPAWN: TilePos = TilePos {
    x: 32491,
    y: 32169,
    floor: -7,
};

struct Progress {
    step: &'static str,
    started: std::time::Instant,
}

impl Progress {
    fn new() -> Self {
        Self {
            step: "world-bundle",
            started: std::time::Instant::now(),
        }
    }

    fn begin(&mut self, step: &'static str) {
        self.step = step;
        verdict::Harness::global().progress(step);
    }

    fn pass(&mut self, step: &'static str, observed: impl std::fmt::Debug) {
        evidence(&format!(
            "W01B_CHECK step={step} status=PASS observed={observed:?}"
        ));
    }

    fn finish(self, result: &TestResult) {
        let status = if result.is_ok() { "PASS" } else { "FAIL" };
        if let Err(error) = result {
            evidence(&format!(
                "W01B_GAP step={} observed={error} need=server_reply_or_state_delta_for_this_step",
                self.step
            ));
        }
        evidence(&format!(
            "e2e_all scenario={ROW}.full status={status} seconds={:.3}",
            self.started.elapsed().as_secs_f64()
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut progress = Progress::new();
    progress.begin("world-bundle");
    let result =
        match verdict::indexed_timeout(Duration::from_secs(660), journey(clients, &mut progress))
            .await
        {
            Ok(result) => result,
            Err(_) => Err(format!("timeout while executing step={}", progress.step).into()),
        };
    progress.finish(&result);
}

fn distance(a: TilePos, b: TilePos) -> u16 {
    if a.floor == b.floor {
        a.x.abs_diff(b.x).max(a.y.abs_diff(b.y))
    } else {
        u16::MAX
    }
}

pub(super) fn rat_points<'a>(
    spawns: &'a oteryn_world_bundle::spawn::Table,
) -> impl Iterator<Item = (TilePos, u32)> + 'a {
    spawns.sources.iter().flat_map(move |source| {
        source.points.iter().filter_map(move |point| {
            spawns
                .creatures
                .get(point.creature as usize)
                .filter(|creature| creature.key == "oteryn:creature.rat")
                .map(|_| {
                    (
                        TilePos {
                            x: point.x,
                            y: point.y,
                            floor: source.floor,
                        },
                        point.respawn_ms,
                    )
                })
        })
    })
}

fn rat_at(
    mut entities: impl Iterator<Item = WorldSpatialEntity>,
    at: TilePos,
) -> Option<EntityRef> {
    entities
        .find(|entity| is_rat(entity) && tile(entity.position).ok() == Some(at))
        .map(|entity| entity.entity)
}

pub(super) fn is_rat(entity: &WorldSpatialEntity) -> bool {
    entity.kind == EntityKind::Creature
        && entity
            .display_name
            .is_some_and(|name| name.as_str() == Ok("Rat"))
}

fn visible_rat_at(client: &DevClientSession, at: TilePos) -> Option<EntityRef> {
    client
        .world_entities()
        .and_then(|entities| rat_at(entities.iter().copied(), at))
}

fn entered_rat_at(events: Vec<SessionEvent>, at: TilePos) -> Option<EntityRef> {
    events.into_iter().find_map(|event| {
        let SessionEvent::WorldSpatialEntities(delta) = event else {
            return None;
        };
        rat_at(delta.value.enter.into_iter(), at)
    })
}

async fn pump(client: &mut DevClientSession) -> TestResult<Vec<oteryn_dev_client::SessionEvent>> {
    client.service_liveness(Duration::from_millis(100)).await?;
    Ok(client.take_events())
}

async fn wait_until(
    client: &mut DevClientSession,
    deadline: tokio::time::Instant,
    mut predicate: impl FnMut(&DevClientSession) -> bool,
) -> TestResult {
    while !predicate(client) {
        if tokio::time::Instant::now() >= deadline {
            return Err("state delta deadline expired".into());
        }
        pump(client).await?;
    }
    Ok(())
}

pub(super) async fn walk_first_attempt(
    client: &mut DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
) -> TestResult {
    for &direction in path {
        let from = tile(client.world_spatial().actor_position)?;
        let expected = destination(world, from, direction)
            .filter(|to| *to != from)
            .ok_or("W01b route step has no distinct authored landing")?;
        let occupant_at_landing = |client: &DevClientSession| {
            client.world_entities().and_then(|view| {
                view.iter()
                    .find(|entity| {
                        entity.kind == EntityKind::Creature
                            && tile(entity.position).ok() == Some(expected)
                    })
                    .map(|entity| entity.entity)
            })
        };
        let before = occupant_at_landing(client);
        let first = client.step(direction).await?;
        let outcome = if first.disposition == StepDisposition::Blocked {
            if first.status != CommandStatus::Accepted
                || tile(client.world_spatial().actor_position)? != from
                || first.world_spatial_delta.is_some()
            {
                return Err(format!("W01b blocked STEP changed position: {first:?}").into());
            }
            let occupant = before.filter(|id| occupant_at_landing(client) == Some(*id))
                .ok_or_else(|| format!(
                    "BLOCKER-SERVER W01b STEP {direction:?} from={from:?} blocked={expected:?} without the same creature before STEP and at refusal reply={first:?}"
                ))?;
            evidence(&format!(
                "W01B_BLOCKED creature={occupant:?} at={expected:?} reply={first:?}"
            ));
            let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
            loop {
                let view = client
                    .world_entities()
                    .ok_or("W01b blocked STEP lost entity view")?;
                if let Some(creature) = view.get(&occupant) {
                    let at = tile(creature.position)?;
                    if at != expected
                        && !view.iter().any(|entity| {
                            matches!(
                                entity.kind,
                                EntityKind::Creature | EntityKind::Player | EntityKind::Npc
                            ) && tile(entity.position).ok() == Some(expected)
                        })
                    {
                        evidence(&format!(
                            "W01B_UNBLOCKED creature={occupant:?} moved_to={at:?}"
                        ));
                        break;
                    }
                }
                if tokio::time::Instant::now() >= deadline {
                    return Err(format!(
                        "W01b creature {occupant:?} did not visibly clear {expected:?}"
                    )
                    .into());
                }
                pump(client).await?;
            }
            client.step(direction).await?
        } else {
            first
        };
        let observed = tile(client.world_spatial().actor_position)?;
        if outcome.status != CommandStatus::Accepted
            || outcome.disposition != StepDisposition::Moved
            || outcome.world_spatial_delta.is_none()
            || observed != expected
        {
            let occupant = client.world_entities().and_then(|entities| {
                entities
                    .iter()
                    .find(|entity| tile(entity.position).ok() == Some(expected))
            });
            return Err(format!(
                "W01b first STEP {direction:?} from={from:?} expected={expected:?} observed={observed:?} occupant={occupant:?} reply={outcome:?}"
            )
            .into());
        }
        verdict::Harness::global().progress("walk");
        client
            .service_liveness(Duration::from_millis(u64::from(outcome.step_wait_ms)))
            .await?;
        client.take_events();
    }
    Ok(())
}

pub(super) async fn logout(client: &mut DevClientSession) -> TestResult<u64> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    while client
        .session_mut()
        .combat_state()
        .ok_or("W01b combat state absent")?
        .in_fight
    {
        if tokio::time::Instant::now() >= deadline {
            return Err("W01b combat lock did not clear in its 60-second interval".into());
        }
        pump(client).await?;
    }
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("logout reply={reply:?}").into());
    }
    Ok(reply.result_server_sequence)
}

pub(super) fn authored_spawn_cells(
    spawns: &oteryn_world_bundle::spawn::Table,
) -> std::collections::BTreeSet<TilePos> {
    spawns
        .sources
        .iter()
        .flat_map(|source| {
            source.points.iter().map(|point| TilePos {
                x: point.x,
                y: point.y,
                floor: source.floor,
            })
        })
        .collect()
}

async fn journey(clients: &SeamClients<'_>, progress: &mut Progress) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    progress.begin("admission");
    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_W01B_ACCOUNT").await?;
    let clients = &dedicated.clients(clients);
    let mut client = character_prep::w01b_rat_hunter("W01b.full", clients, 0xb1).await?;
    progress.begin("initial-spawn");
    let spawn = SPAWN;
    let mut authored = rat_points(&metadata.spawns).filter(|(point, _)| *point == spawn);
    let (_, respawn_ms) = authored
        .next()
        .ok_or("W01b authored Rat point is missing")?;
    if authored.next().is_some() {
        return Err("W01b authored Rat point is ambiguous".into());
    }
    // A spawn cell can already be occupied when the map itself permits walking.
    // Compute each route once with these authored cells excluded before sending STEP.
    let spawn_cells = authored_spawn_cells(&metadata.spawns);
    let available = |at| !spawn_cells.contains(&at);
    let entry = tile(client.world_spatial().actor_position)?;
    let path = route_avoiding(
        world,
        entry,
        |at| {
            distance(at, spawn) == 1
                && world
                    .base()
                    .tile(at.x, at.y, at.floor)
                    .is_some_and(|cell| !cell.protection_zone())
        },
        available,
    )
    .filter(|path| !path.is_empty())
    .ok_or("W01b has no distinct route from admission to its authored point")?;
    walk_first_attempt(&mut client, world, &path).await?;
    let target = visible_rat_at(&client, spawn).ok_or("no live Rat on the authored spawn cell")?;
    let view = client
        .world_entities()
        .ok_or("W01b entity view absent before attack")?;
    let pre_attack = view
        .get(&target)
        .ok_or("W01b target missing before attack")?;
    if !matches!(
        pre_attack.detail,
        EntityDetail::Actor {
            health_percent: 100,
            ..
        }
    ) {
        return Err(format!("W01b Rat must start at full health: {pre_attack:?}").into());
    }
    let prior_corpses: Vec<_> = view
        .iter()
        .filter(|entity| entity.kind == EntityKind::Corpse)
        .map(|entity| entity.entity)
        .collect();
    progress.pass("initial-spawn", (spawn, respawn_ms, target));

    progress.begin("kill-command");
    client.take_events();
    let attack = client.session_mut().attack_target(Some(&target)).await?;
    if attack.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
        return Err(format!("ATTACK_TARGET reply={attack:?}").into());
    }
    progress.pass("kill-command", attack);
    progress.begin("kill-deltas");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    let mut damaged = false;
    let mut removed = false;
    let mut last = spawn;
    let mut corpse = None;
    let corpse = loop {
        for event in pump(&mut client).await? {
            let SessionEvent::WorldSpatialEntities(delta) = event else {
                continue;
            };
            if delta.server_sequence <= attack.result_server_sequence {
                continue;
            }
            if let Some(updated) = delta
                .value
                .update
                .iter()
                .find(|entry| entry.entity == target)
            {
                last = tile(updated.position)?;
                damaged |= matches!(updated.detail, EntityDetail::Actor { health_percent, .. } if health_percent < 100);
            }
            removed |= delta.value.leave.contains(&target);
            if let Some(entered) = delta.value.enter.iter().find(|entity| {
                entity.kind == EntityKind::Corpse
                    && tile(entity.position).is_ok_and(|position| position == last)
                    && !prior_corpses.contains(&entity.entity)
            }) {
                corpse = Some(entered.entity);
            }
        }
        let view = client
            .world_entities()
            .ok_or("W01b entity view absent after attack")?;
        if damaged
            && removed
            && view.get(&target).is_none()
            && let Some(new_corpse) = corpse
            && view.get(&new_corpse).is_some()
        {
            break new_corpse;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "ATTACK_TARGET accepted but corpse/removal delta timed out: damaged={damaged} removed={removed} corpse={corpse:?} last={last:?} target={:?} attacker={:?}",
                view.get(&target),
                client.world_spatial().actor_position
            )
            .into());
        }
    };
    progress.pass("kill-deltas", (target, corpse, last));

    progress.begin("no-early-respawn");
    let nearby = tile(client.world_spatial().actor_position)?;
    if nearby == spawn {
        return Err("W01b respawn observer occupies the spawn cell".into());
    }
    let no_early_respawn_until =
        tokio::time::Instant::now() + Duration::from_millis(u64::from(respawn_ms) + 1_000);
    verdict::Harness::global().bounded_wait(no_early_respawn_until, "bounded-observation");
    while tokio::time::Instant::now() < no_early_respawn_until {
        let events = pump(&mut client).await?;
        if entered_rat_at(events, spawn).is_some() || visible_rat_at(&client, spawn).is_some() {
            return Err(
                "a replacement appeared before the non-blockable Rat warning elapsed".into(),
            );
        }
    }
    progress.pass("no-early-respawn", (spawn, nearby, respawn_ms));

    progress.begin("nearby-replacement");
    let deadline = no_early_respawn_until + Duration::from_secs(10);
    let nearby_replacement = loop {
        let events = pump(&mut client).await?;
        if let Some(replacement) = entered_rat_at(events, spawn) {
            if replacement == target {
                return Err("killed Rat generation re-entered at the authored point".into());
            }
            break replacement;
        }
        if tokio::time::Instant::now() >= deadline {
            let observed = client.world_entities().and_then(|entities| {
                entities
                    .iter()
                    .find(|entity| tile(entity.position).ok() == Some(spawn))
            });
            return Err(format!(
                "no replacement entry at non-blockable Rat spawn={spawn:?} observer={nearby:?} authored_ms={respawn_ms} on_spawn={observed:?}"
            )
            .into());
        }
    };
    progress.pass("nearby-replacement", nearby_replacement);

    progress.begin("leave-interest");
    let from = tile(client.world_spatial().actor_position)?;
    if distance(from, spawn) >= 20 {
        return Err("W01b observer must start inside the leave-interest boundary".into());
    }
    let away = route_avoiding(world, from, |at| distance(at, spawn) >= 20, available)
        .ok_or("no legitimate walking route outside spawn interest")?;
    walk_first_attempt(&mut client, world, &away).await?;
    let outside = tile(client.world_spatial().actor_position)?;
    if distance(outside, spawn) < 20 {
        return Err(format!("W01b STEP route never left spawn interest: {outside:?}").into());
    }
    progress.pass("leave-interest", outside);
    // Keep answering liveness while the interval runs out; a bare sleep lets
    // the server drop the session as unresponsive.
    let until = tokio::time::Instant::now() + Duration::from_millis(u64::from(respawn_ms) + 1_000);
    verdict::Harness::global().bounded_wait(until, "bounded-observation");
    while tokio::time::Instant::now() < until {
        client.service_liveness(Duration::from_millis(500)).await?;
    }
    progress.begin("return-to-spawn");
    let from = tile(client.world_spatial().actor_position)?;
    if from != outside {
        return Err(format!("W01b observer moved while waiting outside: {from:?}").into());
    }
    let back = route_avoiding(world, from, |at| distance(at, spawn) == 1, available)
        .ok_or("no legitimate return route to rat spawn")?;
    walk_first_attempt(&mut client, world, &back).await?;
    let returned = tile(client.world_spatial().actor_position)?;
    if distance(returned, spawn) != 1 {
        return Err(format!("W01b return did not land beside the Rat spawn: {returned:?}").into());
    }
    progress.begin("replacement-delta");
    wait_until(
        &mut client,
        tokio::time::Instant::now() + Duration::from_secs(10),
        |client| visible_rat_at(client, spawn).is_some_and(|entity| entity != target),
    )
    .await?;
    let replacement = visible_rat_at(&client, spawn).ok_or("replacement disappeared")?;
    if replacement != nearby_replacement {
        return Err("return to the spawn observed a different replacement generation".into());
    }
    progress.pass("replacement-delta", replacement);

    progress.begin("logout");
    let saved = tile(client.world_spatial().actor_position)?;
    let old_session = client.join_snapshot().game_session_id;
    if saved == entry {
        return Err("W01b logout position must differ from first admission".into());
    }
    logout(&mut client).await?;
    drop(client);
    wait_character_released(clients, 0).await?;
    progress.pass("logout", (old_session, saved));
    progress.begin("fresh-admission");
    let mut fresh = connect_character(clients, 0xb2, 0).await?;
    let restored = tile(fresh.world_spatial().actor_position)?;
    if fresh.join_snapshot().game_session_id == old_session || restored != saved {
        return Err(
            format!("W01b fresh admission position={restored:?} expected={saved:?}").into(),
        );
    }
    progress.begin("fresh-admission-route");
    let path = route(world, tile(fresh.world_spatial().actor_position)?, |at| {
        distance(at, spawn) <= 1
    })
    .ok_or("fresh admission cannot route to rat spawn")?;
    walk_first_attempt(&mut fresh, world, &path).await?;
    progress.begin("fresh-admission-state");
    wait_until(
        &mut fresh,
        tokio::time::Instant::now() + Duration::from_secs(10),
        |client| visible_rat_at(client, spawn) == Some(replacement),
    )
    .await?;
    if fresh
        .world_entities()
        .is_some_and(|entities| entities.get(&target).is_some())
    {
        return Err("killed actor generation returned after fresh admission".into());
    }
    progress.pass("fresh-admission-state", replacement);
    progress.begin("final-logout");
    logout(&mut fresh).await?;
    drop(fresh);
    wait_character_released(clients, 0).await?;
    progress.pass("final-logout", "released");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_check_requires_a_rat() -> TestResult {
        use oteryn_protocol_oteryn::entity_names::EntityDisplayName;

        let decoy = WorldSpatialEntity {
            motion: None,
            display_name: Some(
                EntityDisplayName::new("Cave Rat")
                    .map_err(|error| format!("display name: {error:?}"))?,
            ),
            kind: EntityKind::Creature,
            entity: EntityRef {
                identity: v7(10, 1),
                generation: 1,
            },
            position: ActorPosition {
                x: 0,
                y: 0,
                floor: 7,
            },
            detail: EntityDetail::Actor {
                direction: StepDirection::East,
                appearance_ref: 1,
                health_percent: 100,
            },
        };
        let rat = WorldSpatialEntity {
            display_name: Some(
                EntityDisplayName::new("Rat")
                    .map_err(|error| format!("display name: {error:?}"))?,
            ),
            entity: EntityRef {
                identity: v7(10, 2),
                generation: 1,
            },
            ..decoy
        };
        let at = tile(rat.position)?;
        assert_eq!(rat_at([decoy].into_iter(), at), None);
        assert_eq!(rat_at([decoy, rat].into_iter(), at), Some(rat.entity));
        Ok(())
    }

    #[test]
    fn distance_is_floor_scoped() {
        let at = TilePos {
            x: 10,
            y: 10,
            floor: -7,
        };
        assert_eq!(
            distance(
                at,
                TilePos {
                    x: 13,
                    y: 8,
                    floor: -7
                }
            ),
            3
        );
        assert_eq!(
            distance(
                at,
                TilePos {
                    x: 10,
                    y: 10,
                    floor: -6
                }
            ),
            u16::MAX
        );
    }
}
