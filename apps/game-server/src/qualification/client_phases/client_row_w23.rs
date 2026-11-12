//! W23 client replay capture: stepping onto a shipped citizen tile makes the Character a citizen
//! and the server sends the private `Game` line "You are now a citizen of <town>." together with a
//! temple teleport. The chat line is a session-owned `SessionEvent::ChatLine`; the row re-encodes
//! it with the production `chat::encode_chat_line` (the C17c practice) and
//! `crates/session/src/tests/e2e_w23.rs` replays it through `Session::apply_state_delta`.
use super::*;
use oteryn_dev_client::{ChatLine, DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::chat;

const SVARGROND_UNIQUE: u16 = 9066;

#[derive(Clone, Copy)]
struct CitizenTile {
    at: TilePos,
    town: u32,
}

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!("W23_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
}

fn town_for(unique: u16) -> Option<u32> {
    Some(match unique {
        9056 => 6, 9057 => 8, 9058 => 9, 9059 => 5, 9060 => 7, 9061 => 13,
        9062 => 10, 9063 => 11, 9064 => 14, 9065 => 15, 9067 => 17,
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
                    && unique != SVARGROND_UNIQUE
                    && let Some(town) = town_for(unique)
                {
                    found.push(CitizenTile { at: TilePos { x: tile.x, y: tile.y, floor: sector.floor }, town });
                }
            }
        }
        Ok(())
    })?;
    Ok(found)
}

async fn game_line(client: &mut DevClientSession, wanted: &str) -> TestResult<ChatLine> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    loop {
        for event in client.take_events() {
            if let SessionEvent::ChatLine(delta) = event
                && let ChatLine::Private { speaker_name, text } = &delta.value
                && speaker_name == "Game"
                && text == wanted
            {
                return Ok(delta.value);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step=message timeout wanted={wanted:?}").into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}

async fn journey(clients: &SeamClients<'_>, world: &BundleWorld) -> TestResult<String> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let tiles = citizen_tiles(&bytes)?;
    let mut client = connect_character(clients, 0x9c, 1).await?;
    let from = tile(client.world_spatial().actor_position)?;
    let mut best: Option<(CitizenTile, Vec<StepDirection>, StepDirection)> = None;
    for entry in &tiles {
        for direction in [StepDirection::North, StepDirection::East, StepDirection::South, StepDirection::West] {
            if let Some(path) = route(world, from, |at| destination(world, at, direction) == Some(entry.at))
                && best.as_ref().is_none_or(|(_, shortest, _)| path.len() < shortest.len())
            {
                best = Some((*entry, path, direction));
            }
        }
    }
    let (selected, path, direction) = best.ok_or("step=route no walking route to a shipped citizen tile")?;
    walk(&mut client, world, &path).await?;
    client.take_events();
    let reply = client.step_retrying(direction).await?;
    if reply.disposition != StepDisposition::Moved {
        return Err(format!("step=citizen-step disposition={:?}", reply.disposition).into());
    }
    let area = world.base().subareas.town(selected.town).ok_or("step=town selected town absent")?;
    let wanted = format!("You are now a citizen of {}.", area.name);
    let line = game_line(&mut client, &wanted).await?;
    let payload = chat::encode_chat_line(&line).map_err(|error| format!("step=encode {error:?}"))?;
    capture("w23-line.bin", &payload);
    capture("w23-expected.txt", format!("speaker=Game\ntext={wanted}\n").as_bytes());
    let _ = client.session_mut().logout().await;
    Ok(format!("town={}", selected.town))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>, world: &BundleWorld) {
    match verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(journey(clients, world))).await {
        Ok(Ok(observed)) => evidence(&format!("e2e_all scenario=W23.client status=CAPTURED evidence={observed}")),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W23.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=W23.client status=FAIL evidence=timeout"),
    }
}
