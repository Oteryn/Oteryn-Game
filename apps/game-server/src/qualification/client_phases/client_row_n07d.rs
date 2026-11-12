//! N07d client replay capture: the real admitted WORLD_MAP_VIEW snapshot that the persisted
//! explored minimap consumes. Minimap markers are client-local (the user edits them; no server
//! command carries them), so only the explored-terrain feed is server-driven. The consumer is
//! `Minimap::observe` in `apps/client/src/features/local-minimap/state.rs`, fed by the session's
//! admitted `world_map_snapshot()`.
use super::*;
use oteryn_protocol_oteryn::world_map::{STATE_DOMAIN_WORLD_MAP_VIEW, in_window};

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!("N07D_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_world(clients, 0x3d).await?;
    let row = client
        .session_mut()
        .gated_snapshot(STATE_DOMAIN_WORLD_MAP_VIEW)
        .ok_or("admission carried no world-map snapshot")?
        .clone();
    let map = client
        .session_mut()
        .world_map_snapshot()
        .ok_or("admitted world-map snapshot malformed")?;
    if row.payload.is_empty() {
        return Err("empty world-map snapshot payload".into());
    }
    capture("n07d-snapshot.bin", &row.payload);
    let header = map.header;
    // Server-observed walkable ground inside the visible window (the cells the minimap remembers).
    let walkable = map
        .tiles
        .iter()
        .filter(|tile| {
            tile.ground_speed > 0
                && tile.position.floor <= header.first_visible_floor
                && in_window(header.origin, tile.position)
        })
        .count();
    if walkable == 0 {
        return Err("admitted map snapshot has no walkable ground".into());
    }
    let expected = format!(
        "revision={}\norigin={},{},{}\nfirst_visible_floor={}\nreset_epoch={}\ntiles={}\nwalkable={walkable}\n",
        row.revision,
        header.origin.x,
        header.origin.y,
        header.origin.floor,
        header.first_visible_floor,
        header.reset_epoch,
        map.tiles.len()
    );
    capture("n07d-expected.txt", expected.as_bytes());
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(60), verdict::observe(journey(clients))).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=N07d.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=N07d.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=N07d.client status=FAIL evidence=timeout"),
    }
}
