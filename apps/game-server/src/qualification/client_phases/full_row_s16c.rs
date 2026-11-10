//! S16C (vocation choice, trial tiles before level 20), driven like a player:
//! 1. Scan the compiled world for Dawnport trial tiles (action ids 25000, 25005-25008).
//! 2. Admit the node-boot character, read its durable vocation and level (read-only SQL) and
//!    search a walkable route from the spawn to a trial tile of another vocation.
//! 3. With a route: walk it with STEP commands onto the tile; below level 20 the durable
//!    vocation must become the tile's vocation and survive logout and a fresh admission; at
//!    level 20 or more the server's refusal leaves the vocation unchanged. The character is then
//!    walked back onto a tile of its original vocation (best effort).
//! 4. Without a route: send the nearest real command, a STEP toward the nearest trial tile,
//!    record the server's reply and report the missing precondition.
use super::*;
use crate::map::view::{MapFacts, placement_key};
use oteryn_dev_client::DevClientSession;
use sqlx::Connection;
use std::collections::{BTreeMap, VecDeque};

const MAX_SEARCH: usize = 600_000;

/// Like the shared `route`, without its 160-tile radius: Dawnport lies far from the spawn.
fn far_route(
    world: &BundleWorld,
    from: TilePos,
    goal: impl Fn(TilePos) -> bool,
) -> Option<Vec<StepDirection>> {
    let mut queue = VecDeque::from([from]);
    let mut parents: BTreeMap<TilePos, Option<(TilePos, StepDirection)>> =
        BTreeMap::from([(from, None)]);
    while let Some(at) = queue.pop_front() {
        if goal(at) {
            let mut path = Vec::new();
            let mut cursor = at;
            while let Some(Some((previous, direction))) = parents.get(&cursor) {
                path.push(*direction);
                cursor = *previous;
            }
            path.reverse();
            return Some(path);
        }
        if parents.len() > MAX_SEARCH {
            return None;
        }
        for direction in [
            StepDirection::North,
            StepDirection::East,
            StepDirection::South,
            StepDirection::West,
        ] {
            let Some(next) = destination(world, at, direction) else {
                continue;
            };
            if parents.contains_key(&next) {
                continue;
            }
            parents.insert(next, Some((at, direction)));
            queue.push_back(next);
        }
    }
    None
}

fn toward(from: TilePos, to: TilePos) -> StepDirection {
    let (dx, dy) = (
        i32::from(to.x) - i32::from(from.x),
        i32::from(to.y) - i32::from(from.y),
    );
    match (dx.abs() >= dy.abs(), dx >= 0, dy >= 0) {
        (true, true, _) => StepDirection::East,
        (true, false, _) => StepDirection::West,
        (false, _, true) => StepDirection::South,
        (false, _, false) => StepDirection::North,
    }
}

async fn text(clients: &SeamClients<'_>, sql: &'static str) -> TestResult<String> {
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let value: String = sqlx::query_scalar(sql)
        .bind(clients.characters[1].as_slice())
        .fetch_one(&mut connection)
        .await?;
    Ok(value)
}

async fn vocation(clients: &SeamClients<'_>) -> TestResult<String> {
    text(
        clients,
        "SELECT COALESCE((SELECT vocation::text FROM game_character_build_state WHERE character_id=encode($1,'hex')::uuid),'none')",
    )
    .await
}

async fn level(clients: &SeamClients<'_>) -> TestResult<u32> {
    let level = text(
        clients,
        "SELECT COALESCE((SELECT level::text FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid),'0')",
    )
    .await?;
    Ok(level.parse().unwrap_or(0))
}

fn load(clients: &SeamClients<'_>) -> TestResult<&'static BundleWorld> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )
}

const TRIALS: [(u16, &str); 5] = [
    (25000, "monk"),
    (25005, "sorcerer"),
    (25006, "druid"),
    (25007, "paladin"),
    (25008, "knight"),
];

fn trial_tiles(world: &BundleWorld) -> Vec<(TilePos, &'static str)> {
    let mut found = Vec::new();
    for (floor, x, y, view) in world.base().tiles() {
        let at = TilePos { x, y, floor };
        for ordinal in 0..view.depths().iter().filter(|depth| **depth == 0).count() {
            let Some(key) = u8::try_from(ordinal)
                .ok()
                .and_then(|ordinal| placement_key(at, ordinal))
            else {
                continue;
            };
            if world.facts().base_hidden(key) {
                continue;
            }
            if let Some(vocation) = world.facts().action_at(key).and_then(|action| {
                TRIALS
                    .iter()
                    .find(|(id, _)| *id == action)
                    .map(|(_, vocation)| *vocation)
            }) {
                found.push((at, vocation));
            }
        }
    }
    found
}

async fn wait_vocation(
    clients: &SeamClients<'_>,
    client: &mut DevClientSession,
    until: impl Fn(&str) -> bool,
) -> TestResult<String> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        client.service_liveness(Duration::from_millis(200)).await?;
        let now = vocation(clients).await?;
        if until(&now) || tokio::time::Instant::now() >= deadline {
            return Ok(now);
        }
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<Verdict> {
    let world = load(clients)?;
    let trials = trial_tiles(world);
    if trials.is_empty() {
        return Ok((
            "SKIPPED",
            "content:no-dawnport-trial-tile-action-25000-25008-in-compiled-world".to_owned(),
        ));
    }
    let mut client = connect_world(clients, 0xf1).await?;
    let before = vocation(clients).await?;
    let level = level(clients).await?;
    let from = tile(client.world_spatial().actor_position)?;
    let route = far_route(world, from, |at| {
        trials
            .iter()
            .any(|(tile, vocation)| *tile == at && *vocation != before)
    });
    let Some(path) = route else {
        let nearest = trials
            .iter()
            .filter(|(_, vocation)| *vocation != before)
            .map(|(at, _)| *at)
            .min_by_key(|at| at.x.abs_diff(from.x).max(at.y.abs_diff(from.y)))
            .ok_or("step=route;observed=no-trial-tile-of-another-vocation")?;
        let direction = toward(from, nearest);
        let outcome = client.step_retrying(direction).await?;
        logout(&mut client).await?;
        return Ok((
            "FAIL",
            format!(
                "step=route;observed=no-walkable-route-to-{}-trial-tiles,step-{direction:?}-reply:{:?},vocation:{before},level:{level};needed=player-route-to-dawnport-trial-tiles",
                trials.len(),
                outcome.disposition
            ),
        ));
    };
    walk(&mut client, world, &path).await?;
    let entered = tile(client.world_spatial().actor_position)?;
    let target = trials
        .iter()
        .find(|(at, _)| *at == entered)
        .map(|(_, vocation)| *vocation)
        .ok_or("step=walk;observed=route-ended-off-a-trial-tile")?;
    let now = wait_vocation(clients, &mut client, |now| now != before).await?;
    if level >= 20 {
        expect(now == before, "trial-refusal", &now)?;
        logout(&mut client).await?;
        return Ok((
            "FAIL",
            format!(
                "step=trial;observed=refused-at-level-{level},vocation-unchanged:{before};needed=character-below-level-20"
            ),
        ));
    }
    expect(now == target, "trial-vocation", (&before, &now, target))?;
    logout(&mut client).await?;

    let mut fresh = connect_world(clients, 0xf2).await?;
    expect(
        vocation(clients).await? == target,
        "vocation-after-relog",
        target,
    )?;
    // Best effort: return the character to its original vocation for later rows.
    let here = tile(fresh.world_spatial().actor_position)?;
    if let Some(back) = far_route(world, here, |at| {
        trials
            .iter()
            .any(|(tile, vocation)| *tile == at && *vocation == before)
    }) && walk(&mut fresh, world, &back).await.is_ok()
    {
        let _ = wait_vocation(clients, &mut fresh, |now| now == before).await;
    }
    logout(&mut fresh).await?;
    Ok((
        "PASS",
        format!("walked-onto-{target}-trial-tile;vocation:{before}->{target};durable-after-relog"),
    ))
}

type Verdict = (&'static str, String);

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(420), journey(clients)).await;
    let (status, detail) = match result {
        Ok(Ok((status, detail))) => (status, token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=S16c.full status={status} seconds={:.3} evidence={detail}",
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
