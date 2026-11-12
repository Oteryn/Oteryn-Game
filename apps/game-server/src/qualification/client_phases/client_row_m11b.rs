//! M11b client replay capture: records the spatial and actor-vitals payloads produced by a
//! successful Magic Rope cast for `oteryn-session`'s `Session::apply_state_delta` reducer
//! (`crates/session/src/domain_dispatch.rs`) and its `world_spatial`/`actor_vitals` accessors
//! (`crates/session/src/session_access.rs`).
use super::*;
use oteryn_dev_client::{ChatDisposition, ChatIntent, ChatSpeechMode, DevClientSession};
use oteryn_protocol_oteryn::{actor_spell, world_spatial};

const WORDS: &str = "exani tera";
const ROPE_GROUNDS: &[u32] = &[386, 421, 7762, 12202, 12936, 14238, 17238, 23363, 21965, 21966, 21967, 21968];
const ROPE_TOPS: &[u32] = &[12935];

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(180), verdict::observe(journey(clients))).await;
    let status = match result {
        Ok(Ok(())) => "PASS",
        Ok(Err(error)) => { evidence(&format!("M11B_CLIENT_GAP observed={error}")); "FAIL" }
        Err(_) => { evidence("M11B_CLIENT_GAP observed=journey-timeout"); "FAIL" }
    };
    evidence(&format!("SEAM_EVIDENCE e2e_all scenario=M11b.client status={status} seconds={:.3}", started.elapsed().as_secs_f64()));
}

fn rope_spots(bytes: &[u8]) -> TestResult<Vec<TilePos>> {
    let mut spots = Vec::new();
    oteryn_world_bundle::bundle::visit(bytes, |manifest, sector| {
        for tile in sector.tiles {
            if tile.items.iter().any(|item| {
                let entry = &manifest.palette[item.palette as usize];
                crate::map::palette_appearance(&entry.key).is_some_and(|id| {
                    let id = u32::from(id);
                    if entry.terrain.is_some_and(|terrain| terrain.kind == oteryn_world_bundle::bundle::TerrainKind::Ground) {
                        ROPE_GROUNDS.contains(&id)
                    } else {
                        ROPE_TOPS.contains(&id)
                    }
                })
            }) { spots.push(TilePos { x: tile.x, y: tile.y, floor: sector.floor }); }
        }
        Ok(())
    })?;
    spots.sort(); spots.dedup(); Ok(spots)
}

async fn settle(client: &mut DevClientSession) -> TestResult {
    let until = tokio::time::Instant::now() + Duration::from_secs(3);
    while tokio::time::Instant::now() < until { client.service_liveness(Duration::from_millis(50)).await?; }
    Ok(())
}

fn write_capture(name: &str, bytes: &[u8]) {
    let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else { return; };
    if let Err(error) = std::fs::write(std::path::Path::new(&directory).join(name), bytes) {
        evidence(&format!("M11B_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(bytes.clone(), &pin, WorldId::decode(&clients.world)?, ChannelId::decode(&clients.channel)?)?;
    let spots = rope_spots(&bytes)?;
    let mut client = connect_character(clients, 0xbb, 1).await?;
    let start = tile(client.world_spatial().actor_position)?;
    let path = route(world, start, |at| spots.binary_search(&at).is_ok()).ok_or("no reachable rope spot")?;
    walk(&mut client, world, &path).await?;
    let before_position = *client.world_spatial();
    let before_vitals = client.actor_vitals().cloned().ok_or("missing pre-cast vitals")?;
    let cast = client.chat(&ChatIntent::Say { mode: ChatSpeechMode::Say, text: WORDS.into() }).await?;
    if cast.status != CommandStatus::Accepted || cast.disposition != ChatDisposition::Ok { return Err("rope cast refused".into()); }
    settle(&mut client).await?;
    let after_position = *client.world_spatial();
    let after_vitals = client.actor_vitals().cloned().ok_or("missing post-cast vitals")?;
    if after_position.actor_position.floor != before_position.actor_position.floor + 1 || before_vitals.mana.checked_sub(20) != Some(after_vitals.mana) {
        return Err("rope cast did not produce expected client state".into());
    }
    let spatial_before = world_spatial::encode_world_spatial(&before_position);
    let spatial_after = world_spatial::encode_world_spatial(&after_position);
    let vitals_before = actor_spell::encode_actor_vitals(&before_vitals).map_err(|e| format!("vitals codec: {e:?}"))?;
    let vitals_after = actor_spell::encode_actor_vitals(&after_vitals).map_err(|e| format!("vitals codec: {e:?}"))?;
    write_capture("m11b-spatial-before.bin", &spatial_before);
    write_capture("m11b-spatial-delta.bin", &spatial_after);
    write_capture("m11b-vitals-before.bin", &vitals_before);
    write_capture("m11b-vitals-delta.bin", &vitals_after);
    write_capture("m11b-expected.txt", format!("{} {} {} {} {}\n", after_position.actor_position.x, after_position.actor_position.y, after_position.actor_position.floor, after_vitals.health, after_vitals.mana).as_bytes());
    if client.session_mut().logout().await?.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) { return Err("cleanup logout refused".into()); }
    Ok(())
}
