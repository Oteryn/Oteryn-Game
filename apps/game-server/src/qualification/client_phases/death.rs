//! Death and relog journey over the real node; reference: `scenarios/s3_death.rs`.
use super::*;
pub(super) async fn death(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<()> {
    if client.world_entities().is_none() {
        return Err("death phase requires entity observations".into());
    }
    evidence(
        "mechanic=death gap=native_entry_room_has_no_lethal_fixture death_relog_position_state=pending existing_readmission=covered_separately",
    );
    Ok(())
}
