//! Client replay capture for SCENARIO_MATRIX C16d.

use super::*;

pub(super) async fn run_packet(clients: &SeamClients<'_>, world: &BundleWorld) {
    let observed = verdict::observe(full_e2ef5::client_tile_items(clients, world));
    let result = verdict::indexed_timeout(Duration::from_secs(400), observed)
        .await
        .unwrap_or_else(|_| Err("C16d client tile item journey timed out".into()));
    match result {
        Ok(()) => evidence("e2e_all scenario=C16d.client status=PASS evidence=chameleon-and-disintegrate-observed"),
        Err(error) => evidence(&format!("e2e_all scenario=C16d.client status=FAIL evidence={error}")),
    }
}

pub(super) fn capture(client: &oteryn_dev_client::DevClientSession) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial_entities as wire;

    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    let entities = client.world_entities().ok_or("C16d entities absent")?;
    let mut own = entities.iter()
        .find(|entity| &entity.entity.identity == entities.own_identity())
        .copied()
        .ok_or("C16d own actor absent")?;
    let wire::EntityDetail::Actor { appearance_ref, .. } = own.detail else {
        return Err("C16d own actor appearance absent".into());
    };
    // The appearance replay selects core entity visibility only.
    own.display_name = None;
    let snapshot = wire::WorldSpatialEntitiesSnapshot {
        content_generation: client.world_spatial().content_generation,
        actor_position: client.world_spatial().actor_position,
        own_identity: *entities.own_identity(),
        entities: vec![own],
    };
    let payload = wire::encode_world_spatial_entities_snapshot(&snapshot)
        .map_err(|error| format!("C16d appearance snapshot: {error:?}"))?;
    let path = Path::new(&path);
    std::fs::write(path.join("c16d-entities.bin"), payload)?;
    std::fs::write(path.join("c16d-expected.txt"), format!("{appearance_ref}\n"))?;
    Ok(())
}
