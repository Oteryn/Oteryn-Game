use oteryn_client_runtime::raw_feature::{
    FeatureCommand, staff as wire, staff_tools::StaffToolsWireError,
};
pub use wire::decode_result;

pub fn request(
    envelope: &wire::SignedEnvelope,
    receipt: bool,
) -> Result<FeatureCommand, StaffToolsWireError> {
    Ok(FeatureCommand {
        capability: wire::CAPABILITY_STAFF_V1,
        kind: if receipt {
            wire::COMMAND_TYPE_STAFF_RECEIPT
        } else {
            wire::COMMAND_TYPE_STAFF_REQUEST
        },
        payload: wire::encode_envelope(envelope)?,
    })
}
