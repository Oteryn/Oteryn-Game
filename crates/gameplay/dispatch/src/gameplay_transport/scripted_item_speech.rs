//! Passive source-owned presentation; it registers no command or domain.
use super::capabilities::OfferedCapability;
use oteryn_protocol_oteryn::chat::CAPABILITY_SCRIPTED_ITEM_SPEECH_V1;
pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    base.iter()
        .copied()
        .filter(|entry| available || entry.id != CAPABILITY_SCRIPTED_ITEM_SPEECH_V1)
        .collect()
}
macro_rules! authority_methods {
    () => {
        fn scripted_item_speech_available(&self) -> bool {
            false
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        fn scripted_item_speech_available(&self) -> bool {
            true
        }
    };
}
pub(super) use composed_methods;
