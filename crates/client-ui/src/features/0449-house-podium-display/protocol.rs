use oteryn_client_runtime::{house_podium_display as wire, raw_feature::FeatureCommand};
pub fn request(value: &wire::Request) -> Option<FeatureCommand> {
    Some(FeatureCommand {
        capability: wire::CAPABILITY_HOUSE_PODIUM_DISPLAY_V1,
        kind: wire::COMMAND_TYPE_HOUSE_PODIUM_DISPLAY,
        payload: value.encode()?,
    })
}
