use oteryn_client_runtime::raw_feature::{FeatureCommand, staff_tools as wire};
pub use wire::decode_reply;

pub fn request(
    envelope: &wire::SignedEnvelope,
    receipt: bool,
) -> Result<FeatureCommand, wire::StaffToolsWireError> {
    Ok(FeatureCommand {
        capability: wire::CAPABILITY_STAFF_TOOLS_V1,
        kind: if receipt {
            wire::COMMAND_TYPE_STAFF_TOOLS_RECEIPT
        } else {
            wire::COMMAND_TYPE_STAFF_TOOLS_REQUEST
        },
        payload: wire::encode_envelope(envelope)?,
    })
}
