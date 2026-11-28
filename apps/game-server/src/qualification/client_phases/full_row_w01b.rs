//! W01b — Monster spawn/respawn, derived from the matrix, Crystal reaudit, and
//! `world_respawns.rs`/the spawn owner named by those records.
//!
//! 1. Admit the provisioned node-boot character, load the exact served bundle, walk with native
//!    STEP commands to the authored `oteryn:creature.rat` point, and require its live entity.
//! 2. Send ATTACK_TARGET for that rat; require an accepted/Ok reply, player kill credit,
//!    removal of that actor, and a new corpse at its last live position.
//! 3. Remain in player interest through the authored respawn interval. This is the refusal path:
//!    require the dead actor to stay absent and no new creature to enter at its authored point.
//! 4. Walk outside interest with native STEP commands, wait the next authored interval, return,
//!    and require a new live creature generation at the authored point (the successful respawn).
//! 5. Send LOGOUT and require Accepted, make a fresh admission, walk back to the point, and
//!    re-check the session-independent World state: the replacement remains live and the killed
//!    actor generation has not returned. Finish with another accepted LOGOUT.
use super::*;
use oteryn_dev_client::{
    DevClientSession, EntityKind, EntityRef, SessionEvent, WorldSpatialEntity,
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

async fn pump(client: &mut DevClientSession) -> TestResult<Vec<oteryn_dev_client::SessionEvent>> {
    client.service_liveness(Duration::from_millis(100)).await?;
    Ok(client.take_events())
}

/// The kill owner can remove a Rat on the first hit without publishing a live health update.
async fn wait_for_kill(
    target: WorldSpatialEntity,
    race: std::num::NonZeroU32,
    deadline: tokio::time::Instant,
    mut observe: impl AsyncFnMut() -> TestResult<Vec<SessionEvent>>,
) -> TestResult<(EntityRef, ActorPosition)> {
    use oteryn_protocol_oteryn::analyser::AnalyserFact;

    let is_rat_credit = |fact: &AnalyserFact| *fact == AnalyserFact::Kill { race: Some(race) };
    let mut credited = false;
    let mut removed = false;
    let mut last = target.position;
    let mut corpse = None;
    loop {
        for event in observe().await? {
            match event {
                SessionEvent::AnalyserFacts(delta) => {
                    credited |= delta.value.iter().any(is_rat_credit);
                }
                SessionEvent::AnalyserAttributedFacts(delta) => {
                    credited |= delta.value.iter().any(|entry| is_rat_credit(&entry.fact));
                }
                SessionEvent::WorldSpatialEntities(delta) => {
                    if let Some(live) = delta
                        .value
                        .update
                        .iter()
                        .chain(&delta.value.enter)
                        .find(|entry| entry.entity == target.entity)
                    {
                        last = live.position;
                        removed = false;
                        corpse = None;
                    }
                    removed |= delta.value.leave.contains(&target.entity);
                    if removed {
                        if corpse.is_some_and(|corpse| delta.value.leave.contains(&corpse)) {
                            corpse = None;
                        }
                        if let Some(fresh) = delta.value.enter.iter().find(|entry| {
                            entry.kind == EntityKind::Corpse && entry.position == last
                        }) {
                            corpse = Some(fresh.entity);
                        }
                    }
                }
                _ => {}
            }
        }
        if credited
            && removed
            && let Some(corpse) = corpse
        {
            return Ok((corpse, last));
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "rat kill observation expired: target={:?} credited={credited} removed={removed} corpse={corpse:?} last={last:?}",
                target.entity,
            ).into());
        }
    }
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
    let mut client = connect_world(clients, 0xb1).await?;
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

    use oteryn_protocol_oteryn::attack::{AttackIntentDisposition, ChaseMode, FightModes};

    progress.begin("kill-command");
    let modes = FightModes {
        chase: ChaseMode::Chase,
        ..FightModes::DEFAULT
    };
    let mode = client.session_mut().set_fight_modes(&modes).await?;
    if mode.disposition != AttackIntentDisposition::Ok {
        return Err(format!("chase mode reply={mode:?}").into());
    }
    let race = full_row_p05::race_reference("oteryn:creature.rat")?;
    let live_target = client
        .world_entities()
        .and_then(|entities| entities.get(&target))
        .copied()
        .ok_or("selected rat left the view before ATTACK_TARGET")?;
    client.take_events();
    let attack = client.session_mut().attack_target(Some(&target)).await?;
    if attack.disposition != AttackIntentDisposition::Ok {
        return Err(format!("ATTACK_TARGET reply={attack:?}").into());
    }
    progress.pass("kill-command", attack);
    progress.begin("kill-deltas");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    verdict::Harness::global().bounded_wait(deadline, "kill-deltas");
    let (corpse, at) = wait_for_kill(live_target, race, deadline, async || {
        pump(&mut client).await
    })
    .await?;
    progress.pass("kill-deltas", (target, corpse, at));

    // Chase may move the observer. Re-establish adjacency for the blocked respawn.
    progress.begin("return-to-interest");
    let near = route(world, tile(client.world_spatial().actor_position)?, |at| {
        distance(at, spawn) <= 1
    })
    .ok_or("no route from the kill back into spawn interest")?;
    walk(&mut client, world, &near).await?;
    progress.begin("nearby-refusal");
    let blocked_until =
        tokio::time::Instant::now() + Duration::from_millis(u64::from(respawn_ms) + 1_000);
    verdict::Harness::global().bounded_wait(blocked_until, "nearby-refusal");
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
    progress.begin("out-of-interest-respawn");
    verdict::Harness::global().bounded_wait(until, "out-of-interest-respawn");
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
    let mut fresh = connect_world(clients, 0xb2).await?;
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
    use oteryn_dev_client::EntityDetail;

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
    fn kill_wait_accepts_a_lethal_hit_away_from_spawn_without_a_live_update() -> TestResult {
        use oteryn_dev_client::AppliedDelta;
        use oteryn_protocol_oteryn::analyser::AnalyserFact;
        use oteryn_protocol_oteryn::world_spatial_entities::WorldSpatialEntitiesDelta;

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .start_paused(true)
            .build()?;
        runtime.block_on(async {
            let spawn = TilePos {
                x: 10,
                y: 10,
                floor: -7,
            };
            let last = ActorPosition {
                x: 12,
                y: 10,
                floor: 7,
            };
            assert_ne!(tile(last)?, spawn);
            let target = WorldSpatialEntity {
                detail: EntityDetail::Actor {
                    direction: StepDirection::East,
                    appearance_ref: 1,
                    health_percent: 100,
                },
                kind: EntityKind::Creature,
                entity: EntityRef {
                    identity: v7(10, 2),
                    generation: 1,
                },
                position: last,
                display_name: None,
                motion: None,
            };
            let race = full_row_p05::race_reference("oteryn:creature.rat")?;
            let corpse = WorldSpatialEntity {
                detail: EntityDetail::Object {
                    item_definition_ref: 1,
                    item_handle: None,
                    quantity: 1,
                },
                kind: EntityKind::Corpse,
                entity: EntityRef {
                    identity: v7(10, 3),
                    generation: 0,
                },
                position: last,
                display_name: None,
                motion: None,
            };
            let mut batches = [
                vec![SessionEvent::WorldSpatialEntities(AppliedDelta {
                    server_sequence: 1,
                    base_revision: 1,
                    new_revision: 2,
                    value: WorldSpatialEntitiesDelta {
                        content_generation: [1; 32],
                        actor_position: last,
                        enter: vec![corpse],
                        update: vec![],
                        leave: vec![target.entity],
                    },
                })],
                vec![SessionEvent::AnalyserFacts(AppliedDelta {
                    server_sequence: 2,
                    base_revision: 1,
                    new_revision: 2,
                    value: vec![AnalyserFact::Kill { race: Some(race) }],
                })],
            ]
            .into_iter();
            let observation = wait_for_kill(
                target,
                race,
                tokio::time::Instant::now() + Duration::from_secs(1),
                async || {
                    batches
                        .next()
                        .ok_or_else(|| "missing kill observation after both frames".into())
                },
            )
            .await?;
            assert_eq!(observation, (corpse.entity, last));
            assert_eq!(batches.len(), 0, "the waiter must observe the kill credit");
            Ok(())
        })
    }

    #[test]
    fn distance_is_floor_scoped() {
        let at = TilePos { x: 10, y: 10, floor: -7 };
        assert_eq!(distance(at, TilePos { x: 13, y: 8, floor: -7 }), 3);
        assert_eq!(distance(at, TilePos { x: 10, y: 10, floor: -6 }), u16::MAX);
    }
}
