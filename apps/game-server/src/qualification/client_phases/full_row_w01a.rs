//! W01a full-row player scenario, derived from the matrix/re-audit and the
//! `ai_think` owner that they name:
//! 1. Freshly admit the node-boot character and walk from town to the authored
//!    `oteryn:creature.rat` spawn; expect native spatial snapshots/deltas for a
//!    live creature, without manufacturing either actor or position.
//! 2. Walk within perception and do not send ATTACK; expect successive think
//!    cycles to select the visible player and publish creature position deltas
//!    that close the distance (targeting/chase).
//! 3. Send native ATTACK for that server-issued creature reference; expect an
//!    `Ok` attack reply and health-percent deltas.  Stop attacking at the rat's
//!    authored low-health boundary and expect later think/movement deltas to
//!    increase its distance from the player (runAwayHealth fleeing).
//! 4. Send ATTACK again with the now out-of-range target; this is the refusal
//!    path: expect the command reply not to claim a successful damaging hit and
//!    expect no decreasing-health delta while the rat flees.  Then send the
//!    ordinary clear-target command and require its `Ok` reply.
//! 5. Walk back to town, logout, obtain a fresh Platform admission, and assert
//!    the saved player cell survived.  Walk to the rat spawn again and require
//!    fresh think/movement deltas: monster target/think state is ephemeral and
//!    must neither be persisted as player state nor prevent reacquisition.

use super::*;
use oteryn_dev_client::{DevClientSession, EntityDetail, EntityKind, SessionEvent};
use oteryn_protocol_oteryn::attack::{AttackIntentDisposition, ChaseMode, FightMode, FightModes};
use std::collections::BTreeSet;

const STEPS: &[&str] = &[
    "authored-rat",
    "target-and-chase",
    "attack-and-damage",
    "low-health-flee",
    "out-of-range-refusal",
    "clear-target",
    "logout-fresh-admission",
    "think-reacquires",
];

struct Row {
    passed: BTreeSet<&'static str>,
    started: std::time::Instant,
}

impl Row {
    fn new() -> Self {
        Self {
            passed: BTreeSet::new(),
            started: std::time::Instant::now(),
        }
    }

    fn pass(&mut self, step: &'static str, observed: impl std::fmt::Debug) {
        if STEPS.contains(&step) && self.passed.insert(step) {
            evidence(&format!("W01A_CHECK {step} PASS observed={observed:?}"));
        }
    }

    fn finish(self, result: TestResult) {
        let complete = STEPS.iter().all(|step| self.passed.contains(step));
        if let Err(error) = &result {
            evidence(&format!("W01A_CHECK journey FAIL observed={error}"));
            evidence(&format!("W01A_GAP {error}"));
        }
        for step in STEPS {
            if !self.passed.contains(step) {
                evidence(&format!("W01A_CHECK {step} FAIL missing-delta-or-reply"));
            }
        }
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=W01a.full status={} seconds={:.3}",
            if result.is_ok() && complete {
                "PASS"
            } else {
                "FAIL"
            },
            self.started.elapsed().as_secs_f64(),
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result = verdict::indexed_timeout(Duration::from_secs(180), verdict::observe(journey(&mut row, clients))).await;
    row.finish(match result {
        Ok(result) => result,
        Err(_) => {
            Err("step=journey observed=timeout-180s need=server-think-or-movement-delta".into())
        }
    });
}

fn distance(a: ActorPosition, b: ActorPosition) -> u32 {
    a.x.abs_diff(b.x).max(a.y.abs_diff(b.y))
}

fn adjacent_rat(
    entry: &oteryn_dev_client::WorldSpatialEntity,
    here: ActorPosition,
) -> bool {
    entry.kind == EntityKind::Creature
        && entry.display_name.is_some_and(|name| name.as_str() == Ok("Rat"))
        && entry.position.floor == here.floor
        && distance(entry.position, here) <= 1
}

fn visible_adjacent_rat(
    client: &DevClientSession,
) -> TestResult<oteryn_dev_client::WorldSpatialEntity> {
    let here = client.world_spatial().actor_position;
    client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entry| adjacent_rat(entry, here))
        })
        .copied()
        .ok_or_else(|| {
            "step=authored-rat observed=no-adjacent-rat need=authored-rat-spawn".into()
        })
}

fn actor_health(detail: EntityDetail) -> Option<u8> {
    match detail {
        EntityDetail::Actor { health_percent, .. }
        | EntityDetail::ActorWithIcons { health_percent, .. } => Some(health_percent),
        _ => None,
    }
}

async fn wait_for_motion(
    client: &mut DevClientSession,
    entity: oteryn_dev_client::EntityRef,
    before: ActorPosition,
    toward: bool,
) -> TestResult<ActorPosition> {
    let player = client.world_spatial().actor_position;
    let initial = distance(before, player);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(12);
    loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        let mut latest = None;
        for event in client.take_events() {
            if let SessionEvent::WorldSpatialEntities(delta) = event {
                latest = delta
                    .value
                    .update
                    .iter()
                    .find(|entry| entry.entity == entity)
                    .map(|entry| entry.position)
                    .or(latest);
            }
        }
        if let Some(position) = latest {
            let now = distance(position, player);
            if position != before && ((toward && now < initial) || (!toward && now > initial)) {
                return Ok(position);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step={} observed=no-qualifying-spatial-delta entity={entity:?} from={before:?} player={player:?} need=monster-think-movement", if toward { "target-and-chase" } else { "low-health-flee" }).into());
        }
    }
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0xa1).await?;

    travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await?;
    let target = visible_adjacent_rat(&client)?;
    row.pass("authored-rat", (target.entity, target.position));

    // Create room for the AI to chase using only ordinary cardinal movement.
    let origin = tile(client.world_spatial().actor_position)?;
    let target_tile = tile(target.position)?;
    let retreat = route(world, origin, |at| {
        at.floor == target_tile.floor
            && at
                .x
                .abs_diff(target_tile.x)
                .max(at.y.abs_diff(target_tile.y))
                >= 3
    })
    .ok_or("step=target-and-chase observed=no-walkable-retreat need=reachable-rat-lane")?;
    walk(&mut client, world, &retreat).await?;
    let chased = wait_for_motion(&mut client, target.entity, target.position, true).await?;
    row.pass("target-and-chase", (target.position, chased));

    // Re-approach before ATTACK; this preserves the same real creature generation.
    let chased_tile = tile(chased)?;
    let approach = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at.floor == chased_tile.floor
            && at
                .x
                .abs_diff(chased_tile.x)
                .max(at.y.abs_diff(chased_tile.y))
                <= 1
    })
    .ok_or("step=attack-and-damage observed=target-unreachable need=movement-route")?;
    walk(&mut client, world, &approach).await?;
    let modes = FightModes {
        fight_mode: FightMode::Offensive,
        chase: ChaseMode::Chase,
        secure: true,
    };
    let mode_reply = client.session_mut().set_fight_modes(&modes).await?;
    if mode_reply.disposition != AttackIntentDisposition::Ok {
        return Err(format!("step=attack-and-damage observed=fight-mode-reply-{mode_reply:?} need=accepted-chase-mode").into());
    }
    client.take_events();
    let reply = client
        .session_mut()
        .attack_target(Some(&target.entity))
        .await?;
    if reply.disposition != AttackIntentDisposition::Ok {
        return Err(format!(
            "step=attack-and-damage observed=attack-reply-{reply:?} need=accepted-native-attack"
        )
        .into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(45);
    let mut low = None;
    let mut last_position = chased;
    while low.is_none() {
        client.service_liveness(Duration::from_millis(50)).await?;
        for event in client.take_events() {
            if let SessionEvent::WorldSpatialEntities(delta) = event {
                for update in delta
                    .value
                    .update
                    .iter()
                    .filter(|entry| entry.entity == target.entity)
                {
                    last_position = update.position;
                    if let Some(health_percent) = actor_health(update.detail)
                        && health_percent < 100
                    {
                        row.pass("attack-and-damage", health_percent);
                        // The authored rat has 20 HP and runAwayHealth=5, so
                        // its observable percentage boundary is 25%.
                        if health_percent <= 25 {
                            low = Some(health_percent);
                        }
                    }
                }
            }
        }
        if client
            .world_entities()
            .is_some_and(|view| view.get(&target.entity).is_none())
        {
            return Err("step=low-health-flee observed=rat-died-before-flee-delta need=nonlethal-player-damage-window".into());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(
                "step=attack-and-damage observed=no-low-health-delta-45s need=actor-health-deltas"
                    .into(),
            );
        }
    }
    let low = low.ok_or(
        "step=low-health-flee observed=missing-low-health-snapshot need=observed-health-boundary",
    )?;
    let stopped = client.session_mut().attack_target(None).await?;
    if stopped.disposition != AttackIntentDisposition::Ok {
        return Err(format!(
            "step=low-health-flee observed=stop-attack-reply-{stopped:?} need=ok-stop-attack-reply"
        )
        .into());
    }
    let fled = wait_for_motion(&mut client, target.entity, last_position, false).await?;
    row.pass("low-health-flee", (low, last_position, fled));

    let home = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at == TOWN
    })
    .ok_or("step=out-of-range-refusal observed=no-town-route need=walkable-return")?;
    walk(&mut client, world, &home).await?;
    let refused = client
        .session_mut()
        .attack_target(Some(&target.entity))
        .await?;
    if refused.disposition != AttackIntentDisposition::TargetNotVisible {
        return Err(format!(
            "step=out-of-range-refusal observed=attack-reply-{refused:?} need=TargetNotVisible"
        )
        .into());
    }
    let cleared = client.session_mut().attack_target(None).await?;
    if cleared.disposition != AttackIntentDisposition::Ok {
        return Err(
            format!("step=clear-target observed={cleared:?} need=ok-clear-target-reply").into(),
        );
    }
    row.pass("clear-target", cleared);

    // The refused target is outside interest. Walk back into interest before
    // checking the same generation's health, then return to the saved town cell.
    let last_seen = tile(fled)?;
    let reobserve = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at.floor == last_seen.floor
            && at.x.abs_diff(last_seen.x).max(at.y.abs_diff(last_seen.y)) <= 2
    })
    .ok_or("step=out-of-range-refusal observed=no-return-route need=same-rat-health-observation")?;
    walk(&mut client, world, &reobserve).await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let health = loop {
        if let Some(health) = client.world_entities().and_then(|entities| {
            entities.get(&target.entity).and_then(|entry| actor_health(entry.detail))
        }) {
            break health;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step=out-of-range-refusal observed=target-generation-not-visible-after-return need=same-rat-health-observation".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    };
    if health < low {
        return Err(format!(
            "step=out-of-range-refusal observed=attack-reply-{refused:?},health-before={low},health-after={health} need=nondecreasing-health-after-refusal"
        )
        .into());
    }
    row.pass("out-of-range-refusal", (refused, low, health));

    let home = route(world, tile(client.world_spatial().actor_position)?, |at| at == TOWN)
        .ok_or("step=logout-fresh-admission observed=no-town-route need=walkable-return")?;
    walk(&mut client, world, &home).await?;
    let saved = client.world_spatial().actor_position;
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err(
            "step=logout-fresh-admission observed=logout-refused need=accepted-logout".into(),
        );
    }
    client = connect_world(clients, 0xa2).await?;
    if client.world_spatial().actor_position != saved {
        return Err(format!("step=logout-fresh-admission observed=saved={saved:?},fresh={:?} need=durable-player-cell", client.world_spatial().actor_position).into());
    }
    row.pass("logout-fresh-admission", saved);
    travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await?;
    let fresh = visible_adjacent_rat(&client)?;
    let fresh_tile = tile(fresh.position)?;
    let retreat = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at.floor == fresh_tile.floor
            && at.x.abs_diff(fresh_tile.x).max(at.y.abs_diff(fresh_tile.y)) >= 3
    })
    .ok_or("step=think-reacquires observed=no-retreat-route need=reachable-rat-lane")?;
    walk(&mut client, world, &retreat).await?;
    let reacquired = wait_for_motion(&mut client, fresh.entity, fresh.position, true).await?;
    row.pass("think-reacquires", (fresh.position, reacquired));
    let _ = client.session_mut().logout().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targeting_requires_the_named_rat() -> TestResult {
        use oteryn_protocol_oteryn::entity_names::EntityDisplayName;

        let here = ActorPosition { x: 10, y: 20, floor: 7 };
        let cave_rat = oteryn_dev_client::WorldSpatialEntity {
            motion: None,
            display_name: Some(EntityDisplayName::new("Cave Rat").map_err(|error| format!("{error:?}"))?),
            kind: EntityKind::Creature,
            entity: oteryn_dev_client::EntityRef {
                identity: v7(10, 1),
                generation: 1,
            },
            position: here,
            detail: EntityDetail::Actor {
                direction: StepDirection::East,
                appearance_ref: 1,
                health_percent: 100,
            },
        };
        let rat = oteryn_dev_client::WorldSpatialEntity {
            display_name: Some(EntityDisplayName::new("Rat").map_err(|error| format!("{error:?}"))?),
            ..cave_rat
        };
        assert!(!adjacent_rat(&cave_rat, here));
        assert!(adjacent_rat(&rat, here));
        Ok(())
    }

    #[test]
    fn distance_is_chebyshev_for_visible_tiles() {
        let a = ActorPosition {
            x: 10,
            y: 20,
            floor: 7,
        };
        let b = ActorPosition {
            x: 13,
            y: 21,
            floor: 7,
        };
        assert_eq!(distance(a, b), 3);
    }
}
