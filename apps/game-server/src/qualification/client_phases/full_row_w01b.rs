//! W01b — Monster spawn/respawn, derived from the matrix, Crystal reaudit, and
//! `world_respawns.rs`/the spawn owner named by those records.
//!
//! 1. Admit the provisioned node-boot character, load the exact served bundle, walk with native
//!    STEP commands to the authored `oteryn:creature.rat` point, and require its live entity.
//! 2. Send ATTACK_TARGET for that rat; require an accepted/Ok reply, health update(s), removal of
//!    that actor, and the corpse entity at its last position.
//! 3. Remain in player interest through the authored respawn interval. This is the refusal path:
//!    require the dead actor to stay absent and no new creature to enter at its authored point.
//! 4. Walk outside interest with native STEP commands, wait the next authored interval, return,
//!    and require a new live creature generation at the authored point (the successful respawn).
//! 5. Send LOGOUT and require Accepted, make a fresh admission, walk back to the point, and
//!    re-check the session-independent World state: the replacement remains live and the killed
//!    actor generation has not returned. Finish with another accepted LOGOUT.
use super::*;
use oteryn_dev_client::{
    DevClientSession, EntityDetail, EntityKind, EntityRef, WorldSpatialEntity,
};

use oteryn_protocol_oteryn::attack::{AttackIntentDisposition, ChaseMode, FightMode, FightModes};
use std::collections::BTreeSet;

const ROW: &str = "W01b";
const RAT_SITE: TilePos = TilePos {
    x: 32794,
    y: 32044,
    floor: -7,
};
const RAT_STAND: TilePos = TilePos {
    x: 32793,
    y: 32043,
    floor: -7,
};
const RAT_RETREAT: TilePos = TilePos {
    x: 32779,
    y: 32024,
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
        match verdict::indexed_timeout(Duration::from_secs(420), journey(clients, &mut progress))
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

fn rat_points<'a>(
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

fn is_rat(entity: &WorldSpatialEntity) -> bool {
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

async fn walk_observing_spawn(
    client: &mut DevClientSession,
    path: &[StepDirection],
    landing: impl Fn(TilePos, StepDirection) -> Option<TilePos>,
    spawn: TilePos,
) -> TestResult<Option<EntityRef>> {
    let mut observed = visible_rat_at(client, spawn);
    for (index, step) in path.iter().enumerate() {
        walk_with_landings(client, std::slice::from_ref(step), &landing).await?;
        observed = visible_rat_at(client, spawn).or(observed);
        if observed.is_some() {
            walk_with_landings(client, &path[index + 1..], &landing).await?;
            break;
        }
    }
    let live = |actor: &EntityRef| {
        client
            .world_entities()
            .is_some_and(|entities| entities.get(actor).is_some())
    };
    Ok(visible_rat_at(client, spawn).or_else(|| observed.filter(live)))
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

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("logout reply={reply:?}").into());
    }
    Ok(())
}

async fn seed_rat_retreat(clients: &SeamClients<'_>) -> TestResult {
    let mut initial = connect_character(clients, 0xb0, 0).await?;
    logout(&mut initial).await?;
    drop(initial);
    wait_character_released(clients, 0).await?;

    // Reuse the frame and generation recorded by the initial native logout.
    // Advance only the exact disposable character's last-position sequence.
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let updated = sqlx::query(
        "UPDATE game_character_last_positions
         SET tile_x=$3, tile_y=$4, tile_floor=$5, write_sequence=write_sequence+1
         WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid",
    )
    .bind(clients.characters[0].as_slice())
    .bind(clients.world.as_slice())
    .bind(i32::from(RAT_RETREAT.x))
    .bind(i32::from(RAT_RETREAT.y))
    .bind(-i16::from(RAT_RETREAT.floor))
    .execute(&mut database)
    .await?;
    database.close().await?;
    if updated.rows_affected() != 1 {
        return Err("seeded rat retreat requires one native last-position record".into());
    }
    Ok(())
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
    character_prep::declare_seeded(
        ROW,
        "character,starting-position",
        "dedicated level-one character starts at the verified Venore retreat tile",
    );
    let identity = character_prep::bootstrap_row_character(
        clients,
        "01934f10-7c00-7000-8000-000000000021",
        "01934f10-7c04-7021-805b-3b1122334421",
        "Venore Rat Hunter",
    )
    .await?;
    let clients = &identity.clients(clients);
    let spawn = RAT_SITE;
    let stand = RAT_STAND;
    let retreat = RAT_RETREAT;
    let mut exact_sites = rat_points(&metadata.spawns).filter(|(at, _)| *at == spawn);
    let (_, respawn_ms) = exact_sites.next().ok_or("authored Venore rat missing")?;
    if exact_sites.next().is_some() || distance(spawn, retreat) < 20 {
        return Err("Venore rat site or retreat does not match the qualification fixture".into());
    }
    const ROUTE_RADIUS: u16 = 56;
    const CREATURE_BUFFER: i16 = 6;
    let in_route_floors = |floor| matches!(floor, -8..=-6);
    let site_distance = |at: TilePos| at.x.abs_diff(spawn.x).max(at.y.abs_diff(spawn.y));
    let mut unsafe_tiles = BTreeSet::new();
    for source in &metadata.spawns.sources {
        if !in_route_floors(source.floor) {
            continue;
        }
        for point in &source.points {
            let at = TilePos {
                x: point.x,
                y: point.y,
                floor: source.floor,
            };
            if site_distance(at) > ROUTE_RADIUS + CREATURE_BUFFER.unsigned_abs()
                || metadata.spawns.creatures[point.creature as usize].key == "oteryn:creature.rat"
            {
                continue;
            }
            for dx in -CREATURE_BUFFER..=CREATURE_BUFFER {
                for dy in -CREATURE_BUFFER..=CREATURE_BUFFER {
                    if let Some((x, y)) =
                        at.x.checked_add_signed(dx).zip(at.y.checked_add_signed(dy))
                    {
                        unsafe_tiles.insert(TilePos {
                            x,
                            y,
                            floor: at.floor,
                        });
                    }
                }
            }
        }
    }
    let safe = |at: TilePos| {
        in_route_floors(at.floor)
            && site_distance(at) <= ROUTE_RADIUS
            && !unsafe_tiles.contains(&at)
    };
    if !safe(spawn)
        || !safe(stand)
        || !safe(retreat)
        || rat_points(&metadata.spawns)
            .any(|(other, _)| other != spawn && distance(other, spawn) <= 8)
        || world
            .base()
            .tile(stand.x, stand.y, stand.floor)
            .is_none_or(|tile| tile.protection_zone())
    {
        return Err("authored Venore rat site lost its safety clearance".into());
    }
    let landing = |at, direction| destination(world, at, direction).filter(|&next| safe(next));
    let plan =
        |from, goal: &dyn Fn(TilePos) -> bool| route_with_landings(from, goal, |_| true, landing);
    let approach = plan(retreat, &|at| at == stand)
        .ok_or("no safe STEP approach from the Venore retreat to the rat")?;
    plan(stand, &|at| at == retreat).ok_or("no safe STEP retreat from the Venore rat")?;

    seed_rat_retreat(clients).await?;
    let mut client = connect_character(clients, 0xb1, 0).await?;
    progress.begin("initial-spawn");
    if tile(client.world_spatial().actor_position)? != retreat {
        return Err("W01b admission did not observe the seeded retreat tile".into());
    }
    let target = walk_observing_spawn(&mut client, &approach, landing, spawn)
        .await?
        .ok_or("step=initial-spawn no live exact rat observed at the authored spawn")?;
    progress.pass("initial-spawn", (spawn, respawn_ms, target));

    let mut last = tile(
        client
            .world_entities()
            .and_then(|entities| entities.get(&target))
            .ok_or("authored rat vanished before attack")?
            .position,
    )?;
    let existing_corpses: BTreeSet<_> = client
        .world_entities()
        .into_iter()
        .flat_map(|entities| entities.iter())
        .filter(|entity| entity.kind == EntityKind::Corpse)
        .map(|entity| entity.entity)
        .collect();
    client.take_events();
    progress.begin("kill-command");
    let modes = client
        .session_mut()
        .set_fight_modes(&FightModes {
            fight_mode: FightMode::Offensive,
            chase: ChaseMode::Chase,
            secure: true,
        })
        .await?;
    if modes.status != CommandStatus::Accepted || modes.disposition != AttackIntentDisposition::Ok {
        return Err(format!("rat chase mode reply={modes:?}").into());
    }
    let attack = client.session_mut().attack_target(Some(&target)).await?;
    if attack.status != CommandStatus::Accepted || attack.disposition != AttackIntentDisposition::Ok
    {
        return Err(format!("ATTACK_TARGET reply={attack:?}").into());
    }
    progress.pass("kill-command", attack);
    progress.begin("kill-deltas");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    let mut damaged = false;
    let corpse = loop {
        let events = pump(&mut client).await;
        if client
            .actor_vitals()
            .is_some_and(|vitals| vitals.health == 0)
        {
            return Err(format!("player died before confirming rat kill target={target:?}").into());
        }
        for event in events? {
            if matches!(&event, oteryn_dev_client::SessionEvent::ActorVitals(delta) if delta.value.health == 0)
            {
                return Err(
                    format!("player died before confirming rat kill target={target:?}").into(),
                );
            }
            if let oteryn_dev_client::SessionEvent::WorldSpatialEntities(delta) = event
                && let Some(entity) = delta
                    .value
                    .update
                    .iter()
                    .find(|entry| entry.entity == target)
            {
                last = tile(entity.position)?;
                damaged |= matches!(entity.detail, EntityDetail::Actor { health_percent, .. } if health_percent < 100);
            }
        }
        let corpse = client.world_entities().and_then(|entities| {
            entities.iter().find(|entity| {
                entity.kind == EntityKind::Corpse
                    && !existing_corpses.contains(&entity.entity)
                    && tile(entity.position).is_ok_and(|position| position == last)
            })
        });
        if let Some(corpse) = corpse {
            break corpse.entity;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("ATTACK_TARGET accepted but corpse/removal delta timed out".into());
        }
    };
    if !damaged
        || client
            .world_entities()
            .is_some_and(|entities| entities.get(&target).is_some())
    {
        return Err("kill lacked damage delta or killed actor remained live".into());
    }
    progress.pass("kill-deltas", (target, corpse, last));

    progress.begin("nearby-refusal");
    let blocked_until =
        tokio::time::Instant::now() + Duration::from_millis(u64::from(respawn_ms) + 1_000);
    verdict::Harness::global().bounded_wait(blocked_until, "bounded-observation");
    while tokio::time::Instant::now() < blocked_until {
        pump(&mut client).await?;
        if visible_rat_at(&client, spawn).is_some() {
            return Err("nearby refusal observed an unexpected respawn entity enter delta".into());
        }
    }
    progress.pass("nearby-refusal", (spawn, respawn_ms));

    progress.begin("leave-interest");
    let from = tile(client.world_spatial().actor_position)?;
    let away = plan(from, &|at| at == retreat)
        .ok_or("no safe walking route outside spawn interest from the kill site")?;
    walk_with_landings(&mut client, &away, landing).await?;
    progress.pass(
        "leave-interest",
        tile(client.world_spatial().actor_position)?,
    );
    // Keep answering liveness while the interval runs out; a bare sleep lets
    // the server drop the session as unresponsive.
    let until = tokio::time::Instant::now() + Duration::from_millis(u64::from(respawn_ms) + 1_000);
    verdict::Harness::global().bounded_wait(until, "bounded-observation");
    while tokio::time::Instant::now() < until {
        client.service_liveness(Duration::from_millis(500)).await?;
    }
    progress.begin("return-to-spawn");
    let back = plan(tile(client.world_spatial().actor_position)?, &|at| {
        at == stand
    })
    .ok_or("no legitimate return route to rat spawn")?;
    let observed = walk_observing_spawn(&mut client, &back, landing, spawn).await?;
    progress.begin("replacement-delta");
    let mut replacement = observed;
    wait_until(
        &mut client,
        tokio::time::Instant::now() + Duration::from_secs(10),
        |client| {
            replacement = visible_rat_at(client, spawn).or(replacement);
            replacement.is_some_and(|actor| {
                actor != target
                    && client
                        .world_entities()
                        .is_some_and(|entities| entities.get(&actor).is_some())
            })
        },
    )
    .await?;
    let replacement = replacement.ok_or("replacement disappeared")?;
    progress.pass("replacement-delta", replacement);

    progress.begin("logout");
    let previous_session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    progress.pass("logout", "Accepted");
    drop(client);
    progress.begin("fresh-admission");
    let mut fresh = connect_character(clients, 0xb2, 0).await?;
    if fresh.join_snapshot().game_session_id == previous_session {
        return Err("fresh W01b admission reused the original session".into());
    }
    progress.begin("fresh-admission-route");
    let path = plan(tile(fresh.world_spatial().actor_position)?, &|at| {
        at == stand
    })
    .ok_or("fresh admission cannot route to rat spawn")?;
    walk_with_landings(&mut fresh, &path, landing).await?;
    progress.begin("fresh-admission-state");
    wait_until(
        &mut fresh,
        tokio::time::Instant::now() + Duration::from_secs(10),
        |client| {
            client
                .world_entities()
                .is_some_and(|entities| entities.get(&replacement).is_some())
        },
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
    progress.pass("final-logout", "Accepted");
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
