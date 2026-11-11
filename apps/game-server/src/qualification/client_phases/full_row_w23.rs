//! W23 — Town citizenship / home temple.
//!
//! Player scenario derived from the W23 matrix/audit row and the movement owner:
//! 1. Log in an ordinarily provisioned node-boot character, walk to a reachable non-Svargrond
//!    citizen tile, and step onto it; expect a moved STEP reply,
//!    spatial delta, temple teleport, and the town-specific citizenship confirmation from Game.
//! 2. Log out with the native logout command, obtain a fresh admission, and verify that its
//!    authoritative initial position is the selected town's temple (durable home-town state).
//! 3. Walk, using native STEP commands, to the shipped Svargrond citizen tile (unique 9066).
//! 4. Step onto that tile without Barbarian Test progress; expect a moved STEP reply and spatial
//!    delta followed by the exact Game refusal, with the actor returned to the approach tile.

use super::*;
use super::{
    row_connect_world as connect_world,
    row_finish as row_logout,
};
use oteryn_dev_client::{ChatLine, DevClientSession, SessionEvent};

const SVARGROND_UNIQUE: u16 = 9066;
const REFUSAL: &str = "You first need to absolve the Barbarian Test Quest to become citizen!";

#[derive(Clone, Copy, Debug)]
struct CitizenTile {
    at: TilePos,
    unique: u16,
    town: u32,
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = row_complete(journey(clients)).await;
    let result = match result {
        Ok(result) => result,
        Err(_) => {
            Err("step=journey observed=timeout-after-240s need=reachable-citizen-route".into())
        }
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
        .map(|character| {
            if character.is_ascii_alphanumeric() || "-_:./".contains(character) {
                character
            } else {
                '-'
            }
        })
        .collect()
}

fn town_for(unique: u16) -> Option<u32> {
    Some(match unique {
        9056 => 6,
        9057 => 8,
        9058 => 9,
        9059 => 5,
        9060 => 7,
        9061 => 13,
        9062 => 10,
        9063 => 11,
        9064 => 14,
        9065 => 15,
        9066 => 16,
        9067 => 17,
        9068 => 12,
        9240 => 18,
        9500 => 21,
        9510 => 20,
        9515 => 22,
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
                        at: TilePos {
                            x: tile.x,
                            y: tile.y,
                            floor: sector.floor,
                        },
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

fn approach(
    world: &BundleWorld,
    from: TilePos,
    target: TilePos,
) -> Option<(Vec<StepDirection>, StepDirection)> {
    for direction in [
        StepDirection::North,
        StepDirection::East,
        StepDirection::South,
        StepDirection::West,
    ] {
        let path = route(world, from, |at| {
            destination(world, at, direction) == Some(target)
        });
        if let Some(path) = path {
            return Some((path, direction));
        }
    }
    None
}

async fn messages_until(client: &mut DevClientSession, expected: &str) -> TestResult {
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
            return Err(format!(
                "step=server-message observed=timeout expected={expected:?} need=Game-chat-delta"
            )
            .into());
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
        return Err(format!(
            "step=citizen-step observed={outcome:?} need=moved-reply-and-spatial-delta"
        )
        .into());
    }
    Ok(outcome)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    row_complete(async {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(240);
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
    let mut row_needs_logout_0 = true;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {

    let from = tile(client.world_spatial().actor_position)?;
    let (selected, path, direction) = tiles.iter()
        .filter(|entry| entry.unique != SVARGROND_UNIQUE)
        .filter_map(|entry| approach(world, from, entry.at).map(|(path, direction)| (*entry, path, direction)))
        .min_by_key(|(_, path, _)| path.len())
        .ok_or("step=citizenship-route observed=no-walking-route need=reachable-shipped-citizen-tile")?;
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

    let logout = client.row_logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=logout observed={logout:?} need=accepted").into());
    }
    row_needs_logout_0 = false;
    let mut fresh = connect_world(clients, 0xd9).await?;
    let row_needs_logout_1 = true;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    if fresh.world_spatial().actor_position != expected_position {
        return Err(format!("step=fresh-admission observed={:?} need=durable-home-temple-{expected_position:?}", fresh.world_spatial().actor_position).into());
    }
    evidence(&format!("W23_CHECK persistence PASS fresh={expected_position:?}"));

    let svargrond = tiles.iter().find(|entry| entry.unique == SVARGROND_UNIQUE)
        .ok_or("step=svargrond-content observed=unique-9066-absent need=shipped-citizen-tile")?;
    let from = tile(fresh.world_spatial().actor_position)?;
    let (path, direction) = approach(world, from, svargrond.at)
        .ok_or("step=svargrond-route observed=no-walking-route need=legitimate-player-route-to-unique-9066")?;
    walk(&mut fresh, world, &path).await?;
    let before = fresh.world_spatial().actor_position;
    fresh.take_events();
    let outcome = step_onto(&mut fresh, direction).await?;
    messages_until(&mut fresh, REFUSAL).await?;
    if fresh.world_spatial().actor_position != before {
        return Err(format!("step=svargrond-refusal observed=position-{:?} need=rollback-to-{before:?}", fresh.world_spatial().actor_position).into());
    }
    evidence(&format!("W23_CHECK refusal PASS reply={outcome:?} position={before:?}"));
    Ok(())

    }).await.unwrap_or_else(|error| Err(error.into()));
    if row_needs_logout_1 { tokio::time::timeout(Duration::from_secs(10), row_logout(&mut fresh)).await??; }
    row_result

    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    if row_needs_logout_0 {
        tokio::time::timeout(Duration::from_secs(10), row_logout(&mut client)).await??;
    }
    row_result

    }).await?
}

#[cfg(test)]
mod tests {
    #[test]
    fn evidence_is_one_report_token() {
        assert_eq!(
            super::evidence_token("step=x observed=a b"),
            "step-x-observed-a-b"
        );
    }
}
