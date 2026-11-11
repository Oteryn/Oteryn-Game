//! Client replay capture for SCENARIO_MATRIX C17b.
use super::*;
use super::{
    row_connect_character as connect_character,
    row_finish as row_logout,
};
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::actor_spell::SpellTarget;
use oteryn_protocol_oteryn::world_spatial_entities::{
    EntityFields, EntityLights, WorldSpatialEntitiesSnapshot,
    encode_world_spatial_entities_delta_with_fields,
    encode_world_spatial_entities_snapshot_with_fields,
};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() {
        return;
    }
    match journey(clients).await {
        Ok(verdict) => evidence(&verdict),
        Err(error) => evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=C17b.client status=FAIL evidence={error}"
        )),
    }
}

fn own(client: &DevClientSession) -> TestResult<oteryn_dev_client::EntityRef> {
    own_from(client.world_entities())
}

fn own_from(
    input: Option<&oteryn_dev_client::WorldEntities>,
) -> TestResult<oteryn_dev_client::EntityRef> {
    let entities = input.ok_or("C17b.client entity snapshot absent")?;
    entities
        .iter()
        .find(|entity| &entity.entity.identity == entities.own_identity())
        .map(|entity| entity.entity)
        .ok_or_else(|| "C17b.client own entity absent".into())
}

fn spell_index() -> TestResult<std::num::NonZeroU32> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = crate::content::native_gameplay::NativeGameplayInput::from_manifest(
        &root.join("content/spells.manifest.json"),
    )?;
    let catalog =
        crate::spell::executable_catalog::compile(&input.catalog.bytes, &input.catalog.sha256)?;
    let selections = crate::spell::executable_catalog::compile_source_selection(
        &input.source_selection.bytes,
        &input.source_selection.sha256,
        &catalog,
    )?;
    let book = catalog.into_spell_book_with_selections(&selections)?;
    (1..=book.source_len())
        .filter_map(|index| {
            u32::try_from(index)
                .ok()
                .and_then(std::num::NonZeroU32::new)
        })
        .find(|index| {
            book.indexed(*index)
                .is_some_and(|spell| spell.name.eq_ignore_ascii_case("Invisibility"))
        })
        .ok_or_else(|| "C17b.client Invisibility spell absent".into())
}

fn write_capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&directory).join(name), bytes)?;

    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    row_complete(async {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut caster = connect_character(clients, 0xc1, 1).await?;
    let row_needs_logout_0 = true;
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(300);
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        let mut observer = connect_character(clients, 0xc2, 0).await?;
        let row_needs_logout_1 = true;
        let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    travel_to_spawn(&mut caster, world, &metadata.spawns, "oteryn:creature.rat").await?;
    let caster_position = tile(caster.world_spatial().actor_position)?;
    let path = route(
        world,
        tile(observer.world_spatial().actor_position)?,
        |at| {
            at.floor == caster_position.floor
                && at.x.abs_diff(caster_position.x).max(at.y.abs_diff(caster_position.y)) <= 1
        },
    )
    .ok_or("C17b.client observer cannot reach caster")?;
    walk(&mut observer, world, &path).await?;
    let caster_ref = own(&caster)?;
    let entities = observer
        .world_entities()
        .ok_or("C17b.client observer entity snapshot absent")?;
    if entities.get(&caster_ref).is_none() {
        return Err("C17b.client caster was not initially visible".into());
    }
    let fields = EntityFields::selected(&[6, 286]);
    let snapshot = WorldSpatialEntitiesSnapshot {
        content_generation: observer.world_spatial().content_generation,
        actor_position: observer.world_spatial().actor_position,
        own_identity: *entities.own_identity(),
        entities: entities.iter().copied().collect(),
    };
    let snapshot_payload = encode_world_spatial_entities_snapshot_with_fields(
        &snapshot,
        fields,
        &EntityLights::new(),
    )
    .map_err(|error| format!("C17b.client snapshot encode: {error:?}"))?;
    observer.take_events();
    let cast = caster
        .cast_spell(spell_index()?, SpellTarget::None, false)
        .await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let removed = loop {
        observer.service_liveness(Duration::from_millis(50)).await?;
        let found = observer.take_events().into_iter().find_map(|event| match event {
            SessionEvent::WorldSpatialEntities(delta)
                if delta.value.leave.contains(&caster_ref) =>
            {
                Some(delta)
            }
            _ => None,
        });
        if let Some(delta) = found {
            break delta;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("C17b.client visibility leave delta absent".into());
        }
    };
    let delta_payload = encode_world_spatial_entities_delta_with_fields(
        &removed.value,
        fields,
        &EntityLights::new(),
    )
    .map_err(|error| format!("C17b.client delta encode: {error:?}"))?;
    write_capture("c17b-snapshot.bin", &snapshot_payload)?;
    write_capture("c17b-invisible.bin", &delta_payload)?;
    write_capture(
        "c17b-expected.txt",
        format!(
            "{} {} {} {} {}\n",
            removed.base_revision,
            removed.new_revision,
            removed.server_sequence,
            caster_ref
                .identity
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
            caster_ref.generation
        )
        .as_bytes(),
    )?;
    Ok(format!(
        "SEAM_EVIDENCE e2e_all scenario=C17b.client status=PASS cast_sequence={} leave_sequence={}",
        cast.result_server_sequence, removed.server_sequence
    ))

    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
        if row_needs_logout_1 {
            tokio::time::timeout(Duration::from_secs(10), row_logout(&mut observer)).await??;
        }
        row_result
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    if row_needs_logout_0 {
        tokio::time::timeout(Duration::from_secs(10), row_logout(&mut caster)).await??;
    }
    row_result

    }).await?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn absent_projection_cannot_supply_evidence() {
        assert!(own_from(None).is_err());
    }
}
