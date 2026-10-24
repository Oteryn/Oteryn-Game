#![allow(clippy::unwrap_used)]
use super::*;
use crate::foundation::{ChannelId, WorldId};
use crate::gameplay_transport::capabilities::OfferedCapability;

// Explicit registered-edge test graph only; it does not widen a production offer.
const WITHOUT_INTERACTION: &[OfferedCapability] = &[
    OfferedCapability {
        id: 6,
        requires: &[],
    },
    OfferedCapability {
        id: 30,
        requires: &[6],
    },
];

const FIELD_OFFER: &[OfferedCapability] = &[
    OfferedCapability {
        id: 4,
        requires: &[6],
    },
    OfferedCapability {
        id: 6,
        requires: &[],
    },
    OfferedCapability {
        id: 11,
        requires: &[15, 18],
    },
    OfferedCapability {
        id: 15,
        requires: &[4],
    },
    OfferedCapability {
        id: 18,
        requires: &[4, 6],
    },
    OfferedCapability {
        id: 30,
        requires: &[6],
    },
];

fn id(last: u8) -> [u8; 16] {
    let mut bytes = [0; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes[15] = last;
    bytes
}

fn scope(world: u8, channel: u8) -> RuntimeScopeRefV1 {
    RuntimeScopeRefV1::channel(
        WorldId::decode(&id(world)).unwrap(),
        ChannelId::decode(&id(channel)).unwrap(),
    )
}

#[test]
fn world_required_field_guard_matches_current_owner_before_reading_both_members() {
    let current = scope(1, 2);
    let bound =
        BoundRequiredCapabilities::for_channel_boot(&[11, 30], current, 1, "policy-1", FIELD_OFFER)
            .unwrap();
    assert_eq!(check(&bound, current, 1, "policy-1"), Ok(()));
    for (scope, generation, policy) in [
        (scope(3, 2), 1, "policy-1"),
        (scope(1, 3), 1, "policy-1"),
        (current, 2, "policy-1"),
        (current, 1, "policy-2"),
        (
            RuntimeScopeRefV1::Instance {
                world_id: WorldId::decode(&id(1)).unwrap(),
                instance_id: id(4),
            },
            1,
            "policy-1",
        ),
    ] {
        assert_eq!(
            check(&bound, scope, generation, policy),
            Err(FieldCapabilityRefusal::StaleBinding)
        );
    }
    for (declared, offer, missing) in [
        (&[][..], &FIELD_OFFER[..2], 11),
        (&[30][..], WITHOUT_INTERACTION, 11),
        (&[11][..], &FIELD_OFFER[..5], 30),
    ] {
        let bound =
            BoundRequiredCapabilities::for_channel_boot(declared, current, 1, "policy-1", offer)
                .unwrap();
        assert_eq!(
            check(&bound, current, 1, "policy-1"),
            Err(FieldCapabilityRefusal::MissingCapability(missing))
        );
    }
}

#[test]
fn world_required_field_guard_production_offer_stays_closed_and_successor_does_not_mutate_predecessor()
 {
    let current = scope(1, 2);
    let old =
        BoundRequiredCapabilities::for_channel_boot(&[], current, 1, "policy-1", &[]).unwrap();
    let next =
        BoundRequiredCapabilities::for_channel_boot(&[11, 30], current, 2, "policy-2", FIELD_OFFER)
            .unwrap();
    assert_eq!(
        check(&old, current, 1, "policy-1"),
        Err(FieldCapabilityRefusal::MissingCapability(11))
    );
    assert_eq!(check(&next, current, 2, "policy-2"), Ok(()));
    // CRAFT-1 offers 11 to bundle Worlds; the field pair stays closed while 30 is unoffered.
    let offer = super::super::world_required_capabilities::offered_for_scope(true, None);
    assert!(!offer.iter().any(|entry| entry.id == 30));
    assert!(
        BoundRequiredCapabilities::for_channel_boot(&[11, 30], current, 3, "policy-3", offer)
            .is_err()
    );
    let production = super::super::world_required_capabilities::offered_for_scope(false, None);
    assert!(!production.iter().any(|entry| [11, 30].contains(&entry.id)));
}
