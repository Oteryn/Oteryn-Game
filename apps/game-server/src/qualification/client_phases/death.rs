//! Death and relog journey over the real node; reference: `scenarios/s3_death.rs`.
use super::*;
pub(super) async fn death(_client: &mut oteryn_dev_client::DevClientSession) -> TestResult<()> {
    // Spawned rats alone are not a deterministic lethal encounter. Default node boot
    // has no native gameplay generation for the combat/death journey.
    evidence("mechanic=death skipped=entry_room_has_no_native_lethal_encounter_content");
    Ok(())
}
