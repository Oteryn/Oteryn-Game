//! Passive native format negotiation; command402 keeps its existing owner.
use super::capabilities::OfferedCapability;
use oteryn_protocol_oteryn::house_wrap::CAPABILITY_HOUSE_FURNITURE_WRAP_OPAQUE_V1;
pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    base.iter()
        .copied()
        .filter(|entry| available || entry.id != CAPABILITY_HOUSE_FURNITURE_WRAP_OPAQUE_V1)
        .collect()
}
macro_rules! authority_methods {
    () => {
        fn house_wrap_opaque_available(&self) -> bool {
            false
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        fn house_wrap_opaque_available(&self) -> bool {
            self.house_wrap_available()
        }
    };
}
pub(super) use composed_methods;
