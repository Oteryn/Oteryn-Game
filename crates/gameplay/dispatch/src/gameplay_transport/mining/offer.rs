//! A content-qualified extension of the bundle offer. Fixture/bare Worlds keep
//! their existing set; neither a tool without its target nor missing outputs
//! can activate the path.
use super::super::capabilities::{BUNDLE_WORLD_OFFERED_CAPABILITIES, OfferedCapability};
use crate::content::ActiveGeneration;

const COUNT: usize = BUNDLE_WORLD_OFFERED_CAPABILITIES.len() + 2;
const OFFERED: [OfferedCapability; COUNT] = offers();

const fn offers() -> [OfferedCapability; COUNT] {
    let mut result = [OfferedCapability {
        id: 0,
        requires: &[],
    }; COUNT];
    let mut source = 0;
    let mut target = 0;
    let mut interaction = false;
    let mut item_use = false;
    while target < COUNT {
        let next = if source < BUNDLE_WORLD_OFFERED_CAPABILITIES.len() {
            BUNDLE_WORLD_OFFERED_CAPABILITIES[source].id
        } else {
            u32::MAX
        };
        if !interaction && next > 11 {
            result[target] = OfferedCapability {
                id: 11,
                requires: &[15, 18],
            };
            interaction = true;
        } else if !item_use && next > 15 {
            result[target] = OfferedCapability {
                id: 15,
                requires: &[4],
            };
            item_use = true;
        } else {
            result[target] = BUNDLE_WORLD_OFFERED_CAPABILITIES[source];
            source += 1;
        }
        target += 1;
    }
    result
}

pub(in crate::gameplay_transport) fn offered(
    active: Option<&ActiveGeneration>,
) -> &'static [OfferedCapability] {
    let Some(native) = active.and_then(ActiveGeneration::native_gameplay) else {
        return BUNDLE_WORLD_OFFERED_CAPABILITIES;
    };
    let Some(index) = native.item_index() else {
        return BUNDLE_WORLD_OFFERED_CAPABILITIES;
    };
    if [3456, 10310, 3043, 3031, 3035, 3028].iter().all(|id| {
        let key = format!("oteryn:item.tibia.i{id}");
        index
            .revision(&key)
            .is_some_and(|revision| native.item_policy(&key, revision).is_some())
    }) {
        &OFFERED
    } else {
        BUNDLE_WORLD_OFFERED_CAPABILITIES
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    #[test]
    fn mining_offer_preserves_bundle_and_requires_the_whole_use_closure() {
        assert_eq!(offered(None), BUNDLE_WORLD_OFFERED_CAPABILITIES);
        assert!(OFFERED.windows(2).all(|pair| pair[0].id < pair[1].id));
        for base in BUNDLE_WORLD_OFFERED_CAPABILITIES {
            assert!(OFFERED.contains(base));
        }
        let select = |supported: &[u32]| {
            super::super::super::capabilities::SelectedCapabilities::select(&OFFERED, supported)
                .expect("bounded")
        };
        assert!(!select(&[4, 6, 11, 18]).contains(11));
        assert!(!select(&[4, 6, 11, 15]).contains(11));
        assert!(select(&[4, 6, 11, 15, 18]).contains(11));
    }
}
