use super::*;
pub(super) async fn hireling(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<()> {
    if !client
        .selected_capabilities()
        .contains(&oteryn_protocol_oteryn::hireling::CAPABILITY_HIRELING_V1)
    {
        evidence(
            "mechanic=hireling skipped=capability_165_not_selected source=crates/gameplay/dispatch/src/gameplay_transport/world_required_capabilities.rs:126",
        );
        return Ok(());
    }
    // The journey admits a Channel character. command220 resolves House custody before Query;
    // neither a town coordinate nor negotiating staff establishes that authority.
    evidence(
        "mechanic=hireling skipped=world_journey_has_no_house_admission_route_for_owned_hireling_lamp content_id=oteryn:item.tibia.i29432 source=crates/gameplay/dispatch/src/gameplay_transport/hireling.rs:39",
    );
    Ok(())
}
