//! C16c replays visibility captured by the single real companion journey.
use super::*;

pub(super) async fn run_packet(_clients: &SeamClients<'_>, _world: &BundleWorld) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    let result: TestResult = (|| {
        for stage in ["convince", "animate"] {
            let root = Path::new(&root);
            let before = std::fs::read(root.join(format!("c16c-{stage}-before.bin")))?;
            let delta = std::fs::read(root.join(format!("c16c-{stage}.bin")))?;
            oteryn_protocol_oteryn::world_spatial_entities::decode_world_spatial_entities_snapshot(
                &before,
            )
            .map_err(|error| format!("C16c snapshot: {error:?}"))?;
            oteryn_protocol_oteryn::world_spatial_entities::decode_world_spatial_entities_delta(
                &delta,
            )
            .map_err(|error| format!("C16c delta: {error:?}"))?;
        }
        Ok(())
    })();
    match result {
        Ok(()) => evidence(
            "e2e_all scenario=C16c.client status=CAPTURED evidence=real-companion-visibility",
        ),
        Err(error) => evidence(&format!(
            "e2e_all scenario=C16c.client status=FAIL evidence={error}"
        )),
    }
}
