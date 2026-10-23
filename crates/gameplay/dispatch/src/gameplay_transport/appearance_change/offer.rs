//! Offer only the composed appearance path with admitted cosmetics available.
use super::super::capabilities::OfferedCapability;
use oteryn_protocol_oteryn::appearance::CAPABILITY_APPEARANCE_V1;

pub(in crate::gameplay_transport) fn offered(
    base: &[OfferedCapability],
    available: bool,
) -> Vec<OfferedCapability> {
    let mut result: Vec<_> = base
        .iter()
        .copied()
        .filter(|c| c.id != CAPABILITY_APPEARANCE_V1)
        .collect();
    if available {
        let at = result.partition_point(|c| c.id < CAPABILITY_APPEARANCE_V1);
        result.insert(
            at,
            OfferedCapability {
                id: CAPABILITY_APPEARANCE_V1,
                requires: &[6],
            },
        );
    }
    result
}
