use super::TestResult;
use super::fixture::id;
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    capabilities::{BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities},
    connection::AdmittedSession,
};
use oteryn_protocol_oteryn::channel_switch::{
    self as wire, CAPABILITY_CHANNEL_SWITCH_V1, SwitchLock,
};

pub(super) async fn assert_encounter_switch_lock(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
    active: bool,
) -> TestResult {
    use crate::gameplay_transport::channel_switch::ChannelSwitchOutcome;
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        BUNDLE_WORLD_OFFERED_CAPABILITIES, &[CAPABILITY_CHANNEL_SWITCH_V1],
    ).ok_or("switch capability")?;
    let Some(ChannelSwitchOutcome::Status(status)) = owner.channel_switch_status_wire(admitted, 2, &[]).await else {
        return Err("missing switch status".into());
    };
    let status = wire::decode_status(&status).ok_or("invalid switch status")?;
    assert_eq!(status.lock == SwitchLock::Event, active);
    if active {
        let Some(ChannelSwitchOutcome::Refused(status)) = owner.channel_switch_request_wire(
            admitted, 3, &wire::encode_request(id(44)),
        ).await else {
            return Err("active encounter must refuse the switch".into());
        };
        assert_eq!(wire::decode_status(&status).map(|status| status.lock), Some(SwitchLock::Event));
    }
    Ok(())
}
