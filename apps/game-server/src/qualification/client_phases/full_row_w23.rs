//! W23 — Town citizenship / home temple.
//!
//! Player scenario derived from the W23 matrix/audit row and the movement owner:
//! 1. Log in an ordinarily provisioned node-boot character, walk to a reachable non-Svargrond
//!    citizen tile, and step onto it; expect a moved STEP reply,
//!    spatial delta, temple teleport, and the town-specific citizenship confirmation from Game.
//! 2. Log out with the native logout command, obtain a fresh admission, and verify that its
//!    authoritative initial position is the selected town's temple (durable home-town state).
//! 3. Take the qualification transfer to Svargrond, then walk to its shipped citizen tile (9066).
//! 4. Step onto that tile without Barbarian Test progress; expect a moved STEP reply and spatial
//!    delta followed by the exact Game refusal and a teleport to the Svargrond temple
//!    (Crystal 201189f8 citizen.lua:39-43).

use super::*;
use oteryn_dev_client::{ChatLine, DevClientSession, SessionEvent};

const THAIS_UNIQUE: u16 = 9057;

const SVARGROND_UNIQUE: u16 = 9066;
const REFUSAL: &str =
    "You first need to absolve the Barbarian Test Quest to become citizen!";

#[derive(Clone, Copy, Debug)]
struct CitizenTile {
    at: TilePos,
    unique: u16,
    town: u32,
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(journey(clients))).await;
    let result = match result {
        Ok(result) => result,
        Err(_) => Err("step=journey observed=timeout-after-240s need=reachable-citizen-route".into()),
    };
    if let Err(error) = &result {
        evidence(&format!("W23_GAP {error}"));
    }
    evidence(&format!(
        "e2e_all scenario=W23.full status={} seconds={:.3}{}",
        if result.is_ok() { "PASS" } else { "FAIL" },
        started.elapsed().as_secs_f64(),
        result
            .err()
            .map(|error| format!(" evidence={}", evidence_token(&error.to_string())))
            .unwrap_or_default(),
    ));
}

fn evidence_token(value: &str) -> String {
    value
        .chars()
        .map(|character| if character.is_ascii_alphanumeric() || "-_:./".contains(character) { character } else { '-' })
        .collect()
}

fn town_for(unique: u16) -> Option<u32> {
    Some(match unique {
        9056 => 6, 9057 => 8, 9058 => 9, 9059 => 5, 9060 => 7, 9061 => 13,
        9062 => 10, 9063 => 11, 9064 => 14, 9065 => 15, 9066 => 16, 9067 => 17,
        9068 => 12, 9240 => 18, 9500 => 21, 9510 => 20, 9515 => 22,
        _ => return None,
    })
}

fn citizen_tiles(bytes: &[u8]) -> TestResult<Vec<CitizenTile>> {
    let mut found = Vec::new();
    oteryn_world_bundle::bundle::visit(bytes, |_, sector| {
        for tile in sector.tiles {
            for item in tile.items.iter().filter(|item| item.depth == 0) {
                if let Some(unique) = item.attrs.unique
                    && let Some(town) = town_for(unique)
                {
                    found.push(CitizenTile {
                        at: TilePos { x: tile.x, y: tile.y, floor: sector.floor },
                        unique,
                        town,
                    });
                }
            }
        }
        Ok(())
    })?;
    found.sort_by_key(|entry| (entry.unique, entry.at));
    Ok(found)
}

fn enters_citizen_tile(
    from: TilePos,
    direction: StepDirection,
    trigger: TilePos,
    landing: Option<TilePos>,
) -> bool {
    let (dx, dy) = direction.delta();
    landing.is_some()
        && from.floor == trigger.floor
        && i32::from(from.x) + dx == i32::from(trigger.x)
        && i32::from(from.y) + dy == i32::from(trigger.y)
}

fn cardinal_approach(
    world: &BundleWorld,
    from: TilePos,
    goal: impl Fn(TilePos, StepDirection) -> bool,
) -> Option<(Vec<StepDirection>, StepDirection)> {
    shortest_approach(|direction| route(world, from, |at| goal(at, direction)))
}

fn approach(
    world: &BundleWorld,
    from: TilePos,
    target: TilePos,
) -> Option<(Vec<StepDirection>, StepDirection)> {
    cardinal_approach(world, from, |at, direction| {
        enters_citizen_tile(at, direction, target, destination(world, at, direction))
    })
}

fn shortest_approach(
    mut route_for: impl FnMut(StepDirection) -> Option<Vec<StepDirection>>,
) -> Option<(Vec<StepDirection>, StepDirection)> {
    [StepDirection::North, StepDirection::East, StepDirection::South, StepDirection::West]
        .into_iter()
        .filter_map(|direction| route_for(direction).map(|path| (path, direction)))
        .min_by_key(|(path, _)| path.len())
}

/// The town fixture supplies intercity travel. The citizen portal itself is
/// still reached and triggered by native STEP commands in its real temple.
async fn travel_to_temple(
    client: &mut DevClientSession,
    world: &BundleWorld,
    temple: ActorPosition,
) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let target = tile(temple)?;
    let (path, direction) = cardinal_approach(world, from, |at, direction| {
        destination(world, at, direction) == Some(target)
    })
    .ok_or_else(|| format!("no qualification transfer from {from:?} to {target:?}"))?;
    walk(client, world, &path).await?;
    step_onto(client, direction).await?;
    if client.world_spatial().actor_position != temple {
        return Err(format!("qualification transfer did not arrive at {temple:?}").into());
    }
    Ok(())
}

async fn messages_until(
    client: &mut DevClientSession,
    expected: &str,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    loop {
        for event in client.take_events() {
            if let SessionEvent::ChatLine(delta) = event
                && let ChatLine::Private { speaker_name, text } = delta.value
                && speaker_name == "Game"
                && text == expected
            {
                return Ok(());
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step=server-message observed=timeout expected={expected:?} need=Game-chat-delta").into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}

async fn step_onto(
    client: &mut DevClientSession,
    direction: StepDirection,
) -> TestResult<oteryn_dev_client::StepOutcome> {
    let outcome = client.step_retrying(direction).await?;
    if outcome.disposition != StepDisposition::Moved || outcome.world_spatial_delta.is_none() {
        return Err(format!("step=citizen-step observed={outcome:?} need=moved-reply-and-spatial-delta").into());
    }
    Ok(outcome)
}

async fn home_town(database: &mut sqlx::PgConnection, character: &[u8; 16]) -> TestResult<Option<i32>> {
    Ok(sqlx::query_scalar(
        "SELECT town_id FROM game_character_home_towns WHERE character_id=encode($1,'hex')::uuid",
    ).bind(character.as_slice()).fetch_optional(database).await?)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes.clone(),
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let tiles = citizen_tiles(&bytes)?;
    let mut client = connect_world(clients, 0xd8).await?;

    use sqlx::Connection as _;
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let previous_home = home_town(&mut database, &clients.characters[1]).await?;
    let from = tile(client.world_spatial().actor_position)?;
    let (selected, path, direction) = tiles.iter()
        .filter(|entry| entry.unique == THAIS_UNIQUE && previous_home.and_then(|town| u32::try_from(town).ok()) != Some(entry.town))
        .filter_map(|entry| approach(world, from, entry.at).map(|(path, direction)| (*entry, path, direction)))
        .min_by_key(|(_, path, _)| path.len())
        .ok_or("step=citizenship-route observed=no-walking-route need=reachable-shipped-citizen-tile-for-different-home-town")?;
    walk(&mut client, world, &path).await?;
    client.take_events();
    let outcome = step_onto(&mut client, direction).await?;
    let area = world.base().subareas.town(selected.town)
        .ok_or("step=town-catalogue observed=selected-town-absent need=town-area")?;
    let temple = area.temple.ok_or("step=town-catalogue observed=temple-absent need=home-temple")?;
    let expected_position = ActorPosition { x: i32::from(temple.x), y: i32::from(temple.y), floor: -i16::from(temple.floor) };
    let expected_message = format!("You are now a citizen of {}.", area.name);
    messages_until(&mut client, &expected_message).await?;
    if client.world_spatial().actor_position != expected_position {
        return Err(format!("step=temple-teleport observed={:?} need={expected_position:?}", client.world_spatial().actor_position).into());
    }
    evidence(&format!("W23_CHECK citizenship PASS town={} reply={outcome:?} temple={expected_position:?}", selected.town));

    let logout = client.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=logout observed={logout:?} need=accepted").into());
    }
    drop(client);
    let mut fresh = connect_world(clients, 0xd9).await?;
    if fresh.world_spatial().actor_position != expected_position {
        return Err(format!("step=fresh-admission observed={:?} need=durable-home-temple-{expected_position:?}", fresh.world_spatial().actor_position).into());
    }
    let home = home_town(&mut database, &clients.characters[1]).await?;
    if home != Some(i32::try_from(selected.town)?) {
        return Err(format!("step=fresh-admission-home-town observed={home:?} need={}", selected.town).into());
    }
    evidence(&format!("W23_CHECK persistence PASS town={} fresh={expected_position:?}", selected.town));

    let svargrond = tiles.iter().find(|entry| entry.unique == SVARGROND_UNIQUE)
        .ok_or("step=svargrond-content observed=unique-9066-absent need=shipped-citizen-tile")?;
    let temple = world.base().subareas.town(svargrond.town).and_then(|area| area.temple)
        .ok_or("step=svargrond-refusal observed=temple-absent need=Svargrond-temple")?;
    let temple = ActorPosition {
        x: i32::from(temple.x),
        y: i32::from(temple.y),
        floor: -i16::from(temple.floor),
    };
    travel_to_temple(&mut fresh, world, temple).await?;
    let from = tile(fresh.world_spatial().actor_position)?;
    let (path, direction) = approach(world, from, svargrond.at)
        .ok_or("step=svargrond-route observed=no-walking-route need=legitimate-player-route-to-unique-9066")?;
    walk(&mut fresh, world, &path).await?;
    fresh.take_events();
    let outcome = step_onto(&mut fresh, direction).await?;
    messages_until(&mut fresh, REFUSAL).await?;
    if fresh.world_spatial().actor_position != temple {
        return Err(format!("step=svargrond-refusal observed=position-{:?} need=temple-{temple:?}", fresh.world_spatial().actor_position).into());
    }
    evidence(&format!("W23_CHECK refusal PASS reply={outcome:?} position={temple:?}"));
    if home_town(&mut database, &clients.characters[1]).await? != home {
        return Err("Svargrond refusal changed citizenship".into());
    }
    travel_to_temple(&mut fresh, world, expected_position).await?;
    let logout = fresh.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=final-logout observed={logout:?} need=accepted").into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn journey_regression_uses_citizen_trigger_even_when_step_lands_at_temple() {
        let from = super::TilePos { x: 10, y: 10, floor: -7 };
        let trigger = super::TilePos { x: 11, ..from };
        let temple = super::TilePos { x: 20, ..from };
        assert!(super::enters_citizen_tile(from, super::StepDirection::East, trigger, Some(temple)));
        assert!(!super::enters_citizen_tile(from, super::StepDirection::East, trigger, None));
        assert!(!super::enters_citizen_tile(from, super::StepDirection::North, trigger, Some(temple)));
    }

    #[test]
    fn journey_regression_uses_the_shortest_citizen_tile_approach() {
        use super::StepDirection::{East, North};
        let picked = super::shortest_approach(|direction| match direction {
            North => Some(vec![North, North]),
            East => Some(vec![East]),
            _ => None,
        });
        assert_eq!(picked, Some((vec![East], East)));
    }

    #[test]
    fn evidence_is_one_report_token() {
        assert_eq!(super::evidence_token("step=x observed=a b"), "step-x-observed-a-b");
    }
}
