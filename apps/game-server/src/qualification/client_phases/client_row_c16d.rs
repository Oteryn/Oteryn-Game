//! Client replay capture for SCENARIO_MATRIX C16d.

use super::*;

pub(super) async fn run_packet(clients: &SeamClients<'_>, _world: &BundleWorld) {
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() {
        return;
    }
    // Capture this admission only; full F5 journeys retain their own dispatch and verdicts.
    if let Err(error) = verdict::observe(capture(clients)).await {
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=C16d.client capture_error={error}"
        ));
    }
}

async fn capture(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0xd6).await?;
    let expected = own_appearance_reference(&client)?;
    let snapshot = client
        .session_mut()
        .gated_snapshot(
            oteryn_protocol_oteryn::world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
        )
        .ok_or("C16d capture entity snapshot absent")?;
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    let write = || -> TestResult {
        let path = Path::new(&path);
        std::fs::write(path.join("c16d-entities.bin"), &snapshot.payload)?;
        std::fs::write(path.join("c16d-expected.txt"), format!("{expected}\n"))?;
        Ok(())
    };
    if let Err(error) = write() {
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=C16d.client write_error={error}"
        ));
    }
    Ok(())
}

fn own_appearance_reference(client: &oteryn_dev_client::DevClientSession) -> TestResult<u32> {
    let entities = client.world_entities().ok_or("C16d entities absent")?;
    let own = entities
        .iter()
        .find(|entity| &entity.entity.identity == entities.own_identity())
        .ok_or("C16d own actor absent")?;
    match own.detail {
        oteryn_dev_client::EntityDetail::Actor { appearance_ref, .. } => Ok(appearance_ref),
        _ => Err("C16d own actor appearance absent".into()),
    }
}
