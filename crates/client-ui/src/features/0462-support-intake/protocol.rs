use oteryn_client_runtime::{raw_feature::FeatureCommand, support_intake as wire};
/// Encode exactly the preview approved by the user. No consent means no command.
pub fn submission(kind: u32, preview: &str, consent: bool) -> Option<FeatureCommand> {
    if !matches!(kind, wire::CRASH | wire::SURVEY) {
        return None;
    }
    Some(FeatureCommand {
        capability: wire::CAPABILITY_SUPPORT_INTAKE_V1,
        kind,
        payload: wire::encode_submission(preview, consent).ok()?,
    })
}
