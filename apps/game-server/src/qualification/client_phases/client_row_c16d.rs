//! Client replay capture for SCENARIO_MATRIX C16d.

use super::*;

pub(super) async fn run_packet(clients: &SeamClients<'_>, _world: &BundleWorld) {
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() {
        return;
    }
    // world.rs runs the shared Chameleon/Disintegrate journey once before these captures.
    if let Err(error) = row_complete_with_timeout(Duration::from_secs(120), capture(clients))
        .await
        .and_then(|result| result)
    {
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=C16d.client status=FAIL evidence={error}"
        ));
    } else {
        evidence("e2e_all scenario=C16d.client status=CAPTURED evidence=observed-entity-snapshot");
    }
}

async fn capture(clients: &SeamClients<'_>) -> TestResult {
    let mut client = row_connect_world(clients, 0xd6).await?;
    let row_result: TestResult<_> = async {
        let expected = own_appearance_reference(&client)?;
        let entities = client.world_entities().ok_or("C16d entities absent")?;
        let snapshot =
            oteryn_protocol_oteryn::world_spatial_entities::encode_world_spatial_entities_snapshot(
                &oteryn_protocol_oteryn::world_spatial_entities::WorldSpatialEntitiesSnapshot {
                    content_generation: client.world_spatial().content_generation,
                    actor_position: client.world_spatial().actor_position,
                    own_identity: *entities.own_identity(),
                    entities: entities.iter().copied().collect(),
                },
            )
            .map_err(|error| format!("entity snapshot encode: {error:?}"))?;
        let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
            return Ok(());
        };
        let write = || -> TestResult {
            let path = Path::new(&path);
            std::fs::write(path.join("c16d-entities.bin"), &snapshot)?;
            std::fs::write(path.join("c16d-expected.txt"), format!("{expected}\n"))?;
            Ok(())
        };
        write()?;
        Ok(())
    }
    .await;
    let logout_result = row_finish(&mut client).await;
    logout_result?;
    row_result
}

fn own_appearance_reference(client: &oteryn_dev_client::DevClientSession) -> TestResult<u32> {
    own_appearance_reference_from(client.world_entities())
}

fn own_appearance_reference_from(
    input: Option<&oteryn_dev_client::WorldEntities>,
) -> TestResult<u32> {
    let entities = input.ok_or("C16d entities absent")?;
    let own = entities
        .iter()
        .find(|entity| &entity.entity.identity == entities.own_identity())
        .ok_or("C16d own actor absent")?;
    match own.detail {
        oteryn_dev_client::EntityDetail::Actor { appearance_ref, .. } => Ok(appearance_ref),
        _ => Err("C16d own actor appearance absent".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn absent_projection_cannot_supply_evidence() {
        assert!(own_appearance_reference_from(None).is_err());
    }
}
