use super::super::capabilities::{BUNDLE_WORLD_OFFERED_CAPABILITIES, OfferedCapability};
use super::*;
use crate::foundation::{ChannelId, WorldId};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn id(last: u8) -> [u8; 16] {
    let mut bytes = [0; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes[15] = last;
    bytes
}

fn scope(world: u8, channel: u8) -> TestResult<RuntimeScopeRefV1> {
    Ok(RuntimeScopeRefV1::channel(
        WorldId::decode(&id(world)).map_err(|error| format!("{error:?}"))?,
        ChannelId::decode(&id(channel)).map_err(|error| format!("{error:?}"))?,
    ))
}

// Test-only offer graph with the exact registered dependency edges. This deliberately does
// not open the production interaction offer. The real boot constructor validates it against
// its build registry; no test-only BoundRequiredCapabilities constructor or as_slice is used.
const TEST_INTERACTION_OFFER: &[OfferedCapability] = &[
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
];

#[test]
fn exact_current_binding_with_interaction_satisfies_only_the_capability_predicate() -> TestResult {
    let required = BoundRequiredCapabilities::for_channel_boot(
        &[11],
        scope(1, 2)?,
        7,
        "policy-7",
        TEST_INTERACTION_OFFER,
    )
    .map_err(|error| format!("{error:?}"))?;
    assert_eq!(required.effective(), &[4, 6, 11, 15, 18]);
    assert_eq!(
        check_player_field_capabilities(&required, scope(1, 2)?, 7, "policy-7"),
        Ok(())
    );
    Ok(())
}

#[test]
fn actual_bundle_boot_without_interaction_refuses_even_with_spatial_and_map() -> TestResult {
    let required = BoundRequiredCapabilities::for_channel_boot(
        &[18],
        scope(1, 2)?,
        7,
        "policy-7",
        BUNDLE_WORLD_OFFERED_CAPABILITIES,
    )
    .map_err(|error| format!("{error:?}"))?;
    assert_eq!(required.effective(), &[4, 6, 18]);
    assert_eq!(
        check_player_field_capabilities(&required, scope(1, 2)?, 7, "policy-7"),
        Err(PlayerFieldCapabilityRefusal::WorldInteractionRequired),
    );
    Ok(())
}

#[test]
fn each_independently_current_binding_dimension_is_required() -> TestResult {
    let required = BoundRequiredCapabilities::for_channel_boot(
        &[11],
        scope(1, 2)?,
        7,
        "policy-7",
        TEST_INTERACTION_OFFER,
    )
    .map_err(|error| format!("{error:?}"))?;
    // Current facts are supplied independently of the immutable binding. Change exactly
    // one dimension per case, preserving a sufficient effective capability set throughout.
    for (current_scope, current_generation, current_policy) in [
        (scope(3, 2)?, 7, "policy-7"),
        (scope(1, 3)?, 7, "policy-7"),
        (scope(1, 2)?, 6, "policy-7"),
        (scope(1, 2)?, 8, "policy-7"),
        (scope(1, 2)?, 7, "policy-8"),
    ] {
        assert_eq!(
            check_player_field_capabilities(
                &required,
                current_scope,
                current_generation,
                current_policy
            ),
            Err(PlayerFieldCapabilityRefusal::StaleScopeBinding),
        );
    }
    let instance = RuntimeScopeRefV1::instance(
        WorldId::decode(&id(1)).map_err(|error| format!("{error:?}"))?,
        id(3),
    )
    .map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        check_player_field_capabilities(&required, instance, 7, "policy-7"),
        Err(PlayerFieldCapabilityRefusal::StaleScopeBinding),
    );
    Ok(())
}

#[test]
fn a_successor_binding_never_requalifies_the_predecessor() -> TestResult {
    let old = BoundRequiredCapabilities::for_channel_boot(
        &[11],
        scope(1, 2)?,
        7,
        "policy-7",
        TEST_INTERACTION_OFFER,
    )
    .map_err(|error| format!("{error:?}"))?;
    let successor = BoundRequiredCapabilities::for_channel_boot(
        &[18],
        scope(1, 2)?,
        8,
        "policy-8",
        BUNDLE_WORLD_OFFERED_CAPABILITIES,
    )
    .map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        check_player_field_capabilities(&old, scope(1, 2)?, 8, "policy-8"),
        Err(PlayerFieldCapabilityRefusal::StaleScopeBinding)
    );
    assert_eq!(
        check_player_field_capabilities(&successor, scope(1, 2)?, 8, "policy-8"),
        Err(PlayerFieldCapabilityRefusal::WorldInteractionRequired)
    );
    assert_eq!(old.effective(), &[4, 6, 11, 15, 18]);
    Ok(())
}
