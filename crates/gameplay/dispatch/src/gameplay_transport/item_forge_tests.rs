use super::*;
use crate::gameplay_transport::capabilities::OfferedCapability;

#[test]
fn forge_availability_cannot_enable_it_outside_bundle_world_offers() {
    let fallback = [OfferedCapability {
        id: 14,
        requires: &[],
    }];
    assert_eq!(offered(&fallback, true), fallback);
    assert_eq!(offered(&fallback, false), fallback);
    let bundle = [
        fallback[0],
        OfferedCapability {
            id: CAPABILITY_ITEM_FORGE_V1,
            requires: &[14],
        },
    ];
    assert_eq!(offered(&bundle, true), bundle);
    assert_eq!(offered(&bundle, false), fallback);
}
