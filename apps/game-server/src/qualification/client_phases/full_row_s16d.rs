//! S16D (vocation room teleport/doors and mainland item removal), driven like a player:
//! 1. Scan the compiled world for Dawnport vocation doors (quest callbacks 22001-22004).
//! 2. Admit the node-boot character and search a walkable route from the spawn to a tile next
//!    to a door.
//! 3. With a route: walk it, USE the door and require the committed reply, the actor teleported
//!    away and the new position kept by a fresh admission (the inventory counts are recorded).
//! 4. Without a route: send the nearest real command, USE the door placement from the spawn,
//!    require the server's exact refusal (TooFar) and report the missing precondition.
use super::*;
use crate::map::view::{MapFacts, placement_key};
use oteryn_dev_client::DevClientSession;
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

fn door_tiles(world: &BundleWorld) -> Vec<(TilePos, u8)> {
    let mut found = Vec::new();
    for (floor, x, y, view) in world.base().tiles() {
        let at = TilePos { x, y, floor };
        for ordinal in 0..view.depths().iter().filter(|depth| **depth == 0).count() {
            let Some((ordinal, key)) = u8::try_from(ordinal)
                .ok()
                .and_then(|ordinal| placement_key(at, ordinal).map(|key| (ordinal, key)))
            else {
                continue;
            };
            if world.facts().base_hidden(key) {
                continue;
            }
            if (22001..=22004).any(|unique| {
                world
                    .facts()
                    .quest_callback_matches(key, Some(unique), None, None)
            }) {
                found.push((at, ordinal));
            }
        }
    }
    found
}

fn placement(world: &BundleWorld, at: TilePos, ordinal: u8) -> TestResult<Vec<u8>> {
    let key = placement_key(at, ordinal).ok_or("step=placement;observed=no-placement-key")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(placement)
}

fn items(client: &DevClientSession) -> usize {
    client
        .inventory()
        .map_or(0, |inventory| inventory.entries.len())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<Verdict> {
    let world = load(clients)?;
    let doors = door_tiles(world);
    if doors.is_empty() {
        return Ok((
            "SKIPPED",
            "content:no-dawnport-vocation-door-quest-callback-22001-22004-in-compiled-world"
                .to_owned(),
        ));
    }
    let mut client = connect_world(clients, 0xf3).await?;
    let from = tile(client.world_spatial().actor_position)?;
    let near = |at: TilePos| {
        doors.iter().any(|(door, _)| {
            door.floor == at.floor && door.x.abs_diff(at.x).max(door.y.abs_diff(at.y)) <= 1
        })
    };
    let Some(path) = far_route(world, from, near) else {
        let (door, ordinal) = doors[0];
        let reply = client
            .use_object(&placement(world, door, ordinal)?, 0)
            .await?
            .disposition;
        logout(&mut client).await?;
        let reply = format!("{reply:?}");
        let exact = reply == "TooFar";
        return Ok((
            "FAIL",
            format!(
                "step=use-door;observed=no-walkable-route-to-{}-doors,use-from-spawn-reply:{reply},expected-TooFar:{exact};needed=player-route-to-dawnport-vocation-door",
                doors.len()
            ),
        ));
    };
    walk(&mut client, world, &path).await?;
    let here = tile(client.world_spatial().actor_position)?;
    let (door, ordinal) = doors
        .iter()
        .copied()
        .find(|(door, _)| {
            door.floor == here.floor && door.x.abs_diff(here.x).max(door.y.abs_diff(here.y)) <= 1
        })
        .ok_or("step=walk;observed=route-ended-away-from-a-door")?;
    let position = client.world_spatial().actor_position;
    let carried = items(&client);
    let reply = client
        .use_object(&placement(world, door, ordinal)?, 0)
        .await?
        .disposition;
    let reply = format!("{reply:?}");
    expect(reply == "Committed", "door-use", &reply)?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while client.world_spatial().actor_position == position
        && tokio::time::Instant::now() < deadline
    {
        client.service_liveness(Duration::from_millis(100)).await?;
    }
    let moved = client.world_spatial().actor_position;
    expect(moved != position, "door-teleport", (position, moved))?;
    let after = items(&client);
    logout(&mut client).await?;

    let mut fresh = connect_world(clients, 0xf4).await?;
    expect(
        fresh.world_spatial().actor_position == moved,
        "position-after-relog",
        (moved, fresh.world_spatial().actor_position),
    )?;
    logout(&mut fresh).await?;
    Ok((
        "PASS",
        format!(
            "door-committed;teleported:{position:?}->{moved:?};items:{carried}->{after};position-durable"
        ),
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
        "e2e_all scenario=S16D.full status={status} seconds={:.3} evidence={detail}",
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
