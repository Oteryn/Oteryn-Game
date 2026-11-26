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

const ROW: &str = "W01b";

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
    let result = match verdict::indexed_timeout(
        Duration::from_secs(420),
        journey(clients, &mut progress),
    )
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
    travel_to_spawn(
        &mut client,
        world,
        &metadata.spawns,
        "oteryn:creature.rat",
    )
    .await?;
    let here = tile(client.world_spatial().actor_position)?;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities
                .iter()
                .filter(|entity| is_rat(entity))
                .min_by_key(|entity| {
                    tile(entity.position)
                        .map_or(u16::MAX, |position| distance(position, here))
                })
        })
        .ok_or("step=initial-spawn no live creature reached by rat route")?;
    let target_at = tile(target.position)?;
    let target = target.entity;
    let (spawn, respawn_ms) = rat_points(&metadata.spawns)
        .filter(|(point, _)| distance(*point, target_at) <= 8)
        .min_by_key(|(point, _)| distance(*point, here))
        .ok_or("step=initial-spawn live routed creature has no nearby authored rat point")?;
    progress.pass("initial-spawn", (spawn, respawn_ms, target));

    progress.begin("kill-command");
    let attack = client.session_mut().attack_target(Some(&target)).await?;
    if attack.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
        return Err(format!("ATTACK_TARGET reply={attack:?}").into());
    }
    progress.pass("kill-command", attack);
    progress.begin("kill-deltas");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    let mut damaged = false;
    let mut last = target_at;
    let corpse = loop {
        for event in pump(&mut client).await? {
            if let oteryn_dev_client::SessionEvent::WorldSpatialEntities(delta) = event
                && let Some(entity) = delta.value.update.iter().find(|entry| entry.entity == target)
            {
                last = tile(entity.position)?;
                damaged |= matches!(entity.detail, EntityDetail::Actor { health_percent, .. } if health_percent < 100);
            }
        }
        let corpse = client.world_entities().and_then(|entities| {
            entities.iter().find(|entity| {
                entity.kind == EntityKind::Corpse
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
    let blocked_until = tokio::time::Instant::now()
        + Duration::from_millis(u64::from(respawn_ms) + 1_000);
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
    let away = route(world, from, |at| distance(at, spawn) >= 20)
        .ok_or("no legitimate walking route outside spawn interest")?;
    walk(&mut client, world, &away).await?;
    progress.pass("leave-interest", tile(client.world_spatial().actor_position)?);
    // Keep answering liveness while the interval runs out; a bare sleep lets
    // the server drop the session as unresponsive.
    let until = tokio::time::Instant::now() + Duration::from_millis(u64::from(respawn_ms) + 1_000);
    verdict::Harness::global().bounded_wait(until, "bounded-observation");
    while tokio::time::Instant::now() < until {
        client.service_liveness(Duration::from_millis(500)).await?;
    }
    progress.begin("return-to-spawn");
    let back = route(world, tile(client.world_spatial().actor_position)?, |at| {
        distance(at, spawn) <= 1
    })
    .ok_or("no legitimate return route to rat spawn")?;
    walk(&mut client, world, &back).await?;
    progress.begin("replacement-delta");
    wait_until(
        &mut client,
        tokio::time::Instant::now() + Duration::from_secs(10),
        |client| visible_rat_at(client, spawn).is_some_and(|entity| entity != target),
    )
    .await?;
    let replacement = visible_rat_at(&client, spawn).ok_or("replacement disappeared")?;
    progress.pass("replacement-delta", replacement);

    progress.begin("logout");
    logout(&mut client).await?;
    progress.pass("logout", "Accepted");
    drop(client);
    progress.begin("fresh-admission");
    let mut fresh = connect_character(clients, 0xb2, 0).await?;
    progress.begin("fresh-admission-route");
    let path = route(world, tile(fresh.world_spatial().actor_position)?, |at| {
        distance(at, spawn) <= 1
    })
    .ok_or("fresh admission cannot route to rat spawn")?;
    walk(&mut fresh, world, &path).await?;
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
        let at = TilePos { x: 10, y: 10, floor: -7 };
        assert_eq!(distance(at, TilePos { x: 13, y: 8, floor: -7 }), 3);
        assert_eq!(distance(at, TilePos { x: 10, y: 10, floor: -6 }), u16::MAX);
    }
}
