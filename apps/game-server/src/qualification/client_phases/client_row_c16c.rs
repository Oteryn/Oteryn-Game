//! C16c verifies companion transitions through the served client.
use super::*;
use oteryn_protocol_oteryn::world_spatial_entities as wire;

pub(super) fn actions_proven(completed: bool, required: usize) -> bool {
    completed && required > 0
}

pub(super) async fn run_packet(clients: &SeamClients<'_>, world: &BundleWorld) {
    let observed = verdict::observe(full_e2ef5::client_companions(clients, world));
    let result = verdict::indexed_timeout(Duration::from_secs(600), observed)
        .await
        .unwrap_or_else(|_| Err("C16c client companion journey timed out".into()));
    match result {
        Ok(()) => evidence("e2e_all scenario=C16c.client status=PASS evidence=convince-and-animate-observed"),
        Err(error) => evidence(&format!("e2e_all scenario=C16c.client status=FAIL evidence={error}")),
    }
}

pub(super) fn capture(
    client: &oteryn_dev_client::DevClientSession,
    phase: &str,
    companion: oteryn_dev_client::EntityRef,
    retired: Option<oteryn_dev_client::EntityRef>,
) -> TestResult {
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    let entities = client.world_entities().ok_or("C16c companion state absent")?;
    let name = if retired.is_some() { "Skeleton" } else { "Rat" };
    let observed = entities.get(&companion)
        .filter(|entry| entry.display_name.as_ref().is_some_and(|actual| actual.as_str().ok() == Some(name)))
        .ok_or("C16c exact named companion absent")?;
    if retired.is_some_and(|corpse| entities.get(&corpse).is_some()) {
        return Err("C16c animation left its original corpse visible".into());
    }
    let snapshot = wire::WorldSpatialEntitiesSnapshot {
        content_generation: client.world_spatial().content_generation,
        actor_position: client.world_spatial().actor_position,
        own_identity: *entities.own_identity(),
        entities: entities.iter().filter(|entry| matches!(entry.kind, wire::EntityKind::Player | wire::EntityKind::Creature)).copied().collect(),
    };
    let fields = wire::EntityFields { display_names: true, ..wire::EntityFields::selected(&[]) };
    let payload = wire::encode_world_spatial_entities_snapshot_with_fields(&snapshot, fields, &wire::EntityLights::new())
        .map_err(|error| format!("C16c native companion snapshot: {error:?}"))?;
    let root = Path::new(&path);
    let hex = observed.entity.identity.iter().map(|byte| format!("{byte:02x}")).collect::<String>();
    std::fs::write(root.join(format!("c16c-{phase}.bin")), payload)?;
    std::fs::write(root.join(format!("c16c-{phase}-expected.txt")), format!("{name} {hex} {}\n", observed.entity.generation))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admission_alone_cannot_qualify_runes_or_tools() {
        assert!(!actions_proven(false, 2));
        assert!(!actions_proven(false, 6));
        assert!(!actions_proven(true, 0));
        assert!(actions_proven(true, 2));
    }
}
