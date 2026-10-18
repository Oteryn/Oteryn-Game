#![allow(clippy::unwrap_used)]

use super::super::capabilities::SELECTED_CAPACITY;
use super::*;

// Synthetic graph IDs only: these fixtures assign no production capability numbers.
const REGISTERED: &[u32] = &[10, 20, 30, 40];
const DIAMOND: &[OfferedCapability] = &[
    OfferedCapability {
        id: 40,
        requires: &[],
    },
    OfferedCapability {
        id: 10,
        requires: &[20, 30],
    },
    OfferedCapability {
        id: 30,
        requires: &[40],
    },
    OfferedCapability {
        id: 20,
        requires: &[40],
    },
];

fn error(
    declared: &[u32],
    registered: &[u32],
    offered: &[OfferedCapability],
    floor: &[u32],
) -> BootCapabilityError {
    RequiredCapabilities::for_boot(declared, registered, offered, floor).unwrap_err()
}

#[test]
fn boot_closure_is_transitive_sorted_and_deduplicates_shared_dependencies() {
    let scope = RequiredCapabilities::for_boot(&[10], REGISTERED, DIAMOND, &[20, 40]).unwrap();
    assert_eq!(scope.as_slice(), REGISTERED);
    let selected = scope
        .select_for_session(&[10, 20, 30, 40, 99], &[])
        .unwrap();
    assert_eq!(selected.as_slice(), REGISTERED); // Unknown supported IDs keep CAP-NEG-1 behavior.
}

#[test]
fn every_missing_transitive_requirement_refuses_without_degraded_selection() {
    let scope = RequiredCapabilities::for_boot(&[10], REGISTERED, DIAMOND, &[40]).unwrap();
    for missing in REGISTERED {
        let supported: Vec<_> = REGISTERED
            .iter()
            .copied()
            .filter(|id| id != missing)
            .collect();
        assert_eq!(
            scope.select_for_session(&supported, &[]),
            Err(SessionCapabilityError::MissingSupport(vec![*missing]))
        );
    }
    assert_eq!(
        scope.select_for_session(&[10], &[]),
        Err(SessionCapabilityError::MissingSupport(vec![20, 30, 40]))
    );
}

#[test]
fn floor_cannot_be_omitted_even_with_an_empty_declaration() {
    assert_eq!(
        error(&[], REGISTERED, DIAMOND, &[40]),
        BootCapabilityError::MissingFloor(vec![40])
    );
    assert_eq!(
        error(&[30], REGISTERED, DIAMOND, &[20, 40]),
        BootCapabilityError::MissingFloor(vec![20])
    );
    assert_eq!(
        error(&[], &[10], &[], &[10]),
        BootCapabilityError::Unoffered(10)
    );
    assert_eq!(
        error(&[], REGISTERED, DIAMOND, &[99]),
        BootCapabilityError::Unregistered(99)
    );
    let scope = RequiredCapabilities::for_boot(&[], &[], &[], &[]).unwrap();
    assert!(scope.as_slice().is_empty());
    assert!(
        scope
            .select_for_session(&[], &[])
            .unwrap()
            .as_slice()
            .is_empty()
    );
}

#[test]
fn unknown_closed_gate_duplicate_node_and_cycles_fail_boot() {
    assert_eq!(
        error(&[99], REGISTERED, DIAMOND, &[]),
        BootCapabilityError::Unregistered(99)
    );
    assert_eq!(
        error(
            &[10],
            &[10],
            &[OfferedCapability {
                id: 10,
                requires: &[99]
            }],
            &[]
        ),
        BootCapabilityError::Unregistered(99)
    );
    assert_eq!(
        error(
            &[10],
            &[10, 20],
            &[OfferedCapability {
                id: 10,
                requires: &[20]
            }],
            &[]
        ),
        BootCapabilityError::Unoffered(20)
    );
    assert_eq!(
        error(
            &[10],
            &[10],
            &[
                OfferedCapability {
                    id: 10,
                    requires: &[]
                },
                OfferedCapability {
                    id: 10,
                    requires: &[]
                }
            ],
            &[]
        ),
        BootCapabilityError::DuplicateOffered(10)
    );
    assert_eq!(
        error(
            &[10],
            &[10],
            &[OfferedCapability {
                id: 99,
                requires: &[]
            }],
            &[]
        ),
        BootCapabilityError::Unregistered(99)
    );
    for graph in [
        &[
            OfferedCapability {
                id: 10,
                requires: &[20],
            },
            OfferedCapability {
                id: 20,
                requires: &[10],
            },
        ][..],
        &[OfferedCapability {
            id: 10,
            requires: &[10],
        }][..],
    ] {
        assert_eq!(
            error(&[10], &[10, 20], graph, &[]),
            BootCapabilityError::Cycle(10)
        );
    }
}

#[test]
fn malformed_declared_registry_and_floor_lists_are_not_silently_normalized() {
    for invalid in [&[0][..], &[20, 10], &[10, 10]] {
        assert_eq!(
            error(invalid, REGISTERED, DIAMOND, &[]),
            BootCapabilityError::InvalidDeclaredList
        );
        assert_eq!(
            error(&[10], invalid, DIAMOND, &[]),
            BootCapabilityError::InvalidRegisteredList
        );
        assert_eq!(
            error(&[10], REGISTERED, DIAMOND, invalid),
            BootCapabilityError::InvalidFloorList
        );
    }
}

const fn synthetic_ids() -> [u32; MAX_CAPABILITY_COUNT + 1] {
    let mut ids = [0; MAX_CAPABILITY_COUNT + 1];
    let mut index = 0;
    while index < ids.len() {
        ids[index] = index as u32 + 1;
        index += 1;
    }
    ids
}
static IDS: [u32; MAX_CAPABILITY_COUNT + 1] = synthetic_ids();
fn chain(count: usize) -> Vec<OfferedCapability> {
    (0..count)
        .map(|index| OfferedCapability {
            id: IDS[index],
            requires: if index + 1 < count {
                &IDS[index + 1..index + 2]
            } else {
                &[]
            },
        })
        .collect()
}

#[test]
fn declared_maximum_and_maximum_plus_one_preserve_the_accepted_bound() {
    let mut graph = chain(MAX_DECLARED_REQUIRED_CAPABILITIES + 1);
    for node in &mut graph {
        node.requires = &[];
    }
    assert!(RequiredCapabilities::for_boot(&IDS[..16], &IDS[..17], &graph, &[]).is_ok());
    assert_eq!(
        error(&IDS[..17], &IDS[..17], &graph, &[]),
        BootCapabilityError::DeclaredLimitExceeded
    );
}

#[test]
fn closure_bound_and_existing_selection_capacity_are_not_raised() {
    let graph = chain(MAX_CAPABILITY_COUNT);
    assert_eq!(
        closure(&[1], &IDS, &graph).unwrap().len(),
        MAX_CAPABILITY_COUNT
    );
    assert_eq!(
        error(&[1], &IDS, &chain(MAX_CAPABILITY_COUNT + 1), &[]),
        BootCapabilityError::ClosureLimitExceeded
    );
    assert!(RequiredCapabilities::for_boot(&[1], &IDS, &chain(SELECTED_CAPACITY), &[]).is_ok());
    assert_eq!(
        error(&[1], &IDS, &chain(SELECTED_CAPACITY + 1), &[]),
        BootCapabilityError::SelectionUnavailable
    );
}

#[test]
fn profile_closure_is_checked_per_session_without_changing_the_boot_set() {
    let scope = RequiredCapabilities::for_boot(&[40], REGISTERED, DIAMOND, &[40]).unwrap();
    assert_eq!(scope.as_slice(), &[40]);
    // Preserve optional CAP-NEG-1 intersection/pruning outside the forced set.
    assert_eq!(
        scope.select_for_session(&[30, 40], &[]).unwrap().as_slice(),
        &[30, 40]
    );
    assert_eq!(
        scope.select_for_session(&[10, 40], &[]).unwrap().as_slice(),
        &[40]
    );
    assert_eq!(
        scope.select_for_session(&[40], &[10]),
        Err(SessionCapabilityError::MissingSupport(vec![10, 20, 30]))
    );
    assert_eq!(
        scope
            .select_for_session(REGISTERED, &[10])
            .unwrap()
            .as_slice(),
        REGISTERED
    );
    assert_eq!(scope.as_slice(), &[40]);
    assert_eq!(
        scope.select_for_session(REGISTERED, &[99]),
        Err(SessionCapabilityError::InvalidProfile(
            BootCapabilityError::Unregistered(99)
        ))
    );
}

#[test]
fn invalid_profile_graph_or_noncanonical_support_is_refused() {
    let graph = &[
        OfferedCapability {
            id: 10,
            requires: &[20],
        },
        OfferedCapability {
            id: 20,
            requires: &[10],
        },
        OfferedCapability {
            id: 40,
            requires: &[],
        },
    ];
    let scope = RequiredCapabilities::for_boot(&[40], REGISTERED, graph, &[40]).unwrap();
    assert_eq!(
        scope.select_for_session(REGISTERED, &[10]),
        Err(SessionCapabilityError::InvalidProfile(
            BootCapabilityError::Cycle(10)
        ))
    );
    for invalid in [
        &[0][..],
        &[20, 10],
        &[10, 10],
        &IDS[..MAX_CAPABILITY_COUNT + 1],
    ] {
        assert_eq!(
            scope.select_for_session(invalid, &[]),
            Err(SessionCapabilityError::InvalidSupportList)
        );
        assert_eq!(
            scope.select_for_session(REGISTERED, invalid),
            Err(SessionCapabilityError::InvalidProfileList)
        );
    }
}

#[test]
fn session_selection_capacity_and_union_limit_fail_closed() {
    let mut graph = chain(SELECTED_CAPACITY + 1);
    for node in &mut graph {
        node.requires = &[];
    }
    let scope = RequiredCapabilities::for_boot(&[1], &IDS, &graph, &[]).unwrap();
    assert_eq!(
        scope.select_for_session(&IDS[..SELECTED_CAPACITY + 1], &[]),
        Err(SessionCapabilityError::SelectionUnavailable)
    );
    let mut graph = chain(MAX_CAPABILITY_COUNT + 1);
    graph[0].requires = &[];
    let scope = RequiredCapabilities::for_boot(&[1], &IDS, &graph, &[]).unwrap();
    assert_eq!(
        scope.select_for_session(&IDS[..MAX_CAPABILITY_COUNT], &[2]),
        Err(SessionCapabilityError::RequiredLimitExceeded)
    );
}

fn channel_scope(world: u8, channel: u8) -> RuntimeScopeRefV1 {
    fn id(last: u8) -> [u8; 16] {
        let mut bytes = [0; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes[15] = last;
        bytes
    }
    RuntimeScopeRefV1::channel(
        crate::foundation::WorldId::decode(&id(world)).unwrap(),
        crate::foundation::ChannelId::decode(&id(channel)).unwrap(),
    )
}

#[test]
fn real_scope_offer_enforces_the_registered_d1083_floor() {
    use super::super::capabilities::{
        BUNDLE_WORLD_OFFERED_CAPABILITIES, PRODUCTION_OFFERED_CAPABILITIES,
    };
    let scope = channel_scope(1, 2);
    assert_eq!(
        BoundRequiredCapabilities::for_channel_boot(
            &[],
            scope,
            7,
            "policy-7",
            BUNDLE_WORLD_OFFERED_CAPABILITIES
        )
        .unwrap_err(),
        BootCapabilityError::MissingFloor(vec![18]),
    );
    let required = BoundRequiredCapabilities::for_channel_boot(
        &[18],
        scope,
        7,
        "policy-7",
        BUNDLE_WORLD_OFFERED_CAPABILITIES,
    )
    .unwrap();
    assert_eq!(required.required.as_slice(), &[4, 6, 18]);
    // 11 is registered but its actual scope offer remains closed. 6 never aliases FIELD.
    assert_eq!(
        BoundRequiredCapabilities::for_channel_boot(
            &[11, 18],
            scope,
            7,
            "policy-7",
            BUNDLE_WORLD_OFFERED_CAPABILITIES
        )
        .unwrap_err(),
        BootCapabilityError::Unoffered(11)
    );
    assert!(
        build_registry()
            .unwrap()
            .rows
            .iter()
            .all(|row| row.name != "WORLD_SPATIAL_FIELDS")
    );
    assert!(
        BoundRequiredCapabilities::for_channel_boot(
            &[],
            scope,
            7,
            "policy-7",
            PRODUCTION_OFFERED_CAPABILITIES
        )
        .is_ok()
    );
}

#[test]
fn real_required_members_are_forced_and_each_missing_member_refuses() {
    let required = BoundRequiredCapabilities::for_channel_boot(
        &[18],
        channel_scope(1, 2),
        7,
        "policy-7",
        super::super::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
    )
    .unwrap();
    assert_eq!(required.effective(), &[4, 6, 18]);
    for missing in [4, 6, 18] {
        let support: Vec<_> = [4, 6, 18].into_iter().filter(|id| *id != missing).collect();
        assert!(
            matches!(required.authenticate_support(&support), Err(SessionCapabilityError::MissingSupport(ids)) if ids == [missing])
        );
        assert!(
            matches!(required.authenticate_resume_support(&support, SelectedCapabilities::NONE), Err(SessionCapabilityError::MissingSupport(ids)) if ids == [missing])
        );
    }
    assert_eq!(
        required
            .authenticate_support(&[4, 6, 18])
            .unwrap()
            .selected()
            .as_slice(),
        &[4, 6, 18]
    );
    // Required additions get a replacement baseline; optional negotiation never widens a resume.
    assert_eq!(
        required
            .authenticate_resume_support(&[3, 4, 6, 7, 18], SelectedCapabilities::NONE)
            .unwrap()
            .selected()
            .as_slice(),
        &[4, 6, 18]
    );
}

#[test]
fn independently_current_binding_is_required_by_the_durable_callback() {
    let scope = channel_scope(1, 2);
    let required = BoundRequiredCapabilities::for_channel_boot(
        &[18],
        scope,
        7,
        "policy-7",
        super::super::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
    )
    .unwrap();
    let authenticated = required.authenticate_support(&[4, 6, 18]).unwrap();
    let check = authenticated.current_check();
    assert!(check(scope, 7, "policy-7"));
    for (current, generation, policy) in [
        (channel_scope(3, 2), 7, "policy-7"),
        (channel_scope(1, 3), 7, "policy-7"),
        (scope, 6, "policy-7"),
        (scope, 8, "policy-7"),
        (scope, 7, "policy-8"),
    ] {
        assert!(!check(current, generation, policy));
        assert_eq!(
            authenticated.revalidate(current, generation, policy),
            Err(SessionCapabilityError::StaleScopeBinding)
        );
    }
}

#[test]
fn the_declared_limit_is_registered_and_applies_before_closure() {
    let registry: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../docs/contracts/RESOURCE_LIMITS_REGISTRY.json"
    ))
    .unwrap();
    let row = registry["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == "ADMIT0-RL-01")
        .unwrap();
    assert_eq!(
        row["hard_maximum"].as_u64(),
        Some(MAX_DECLARED_REQUIRED_CAPABILITIES as u64)
    );
    assert_eq!(row["configurable_range"]["maximum"].as_u64(), Some(16));
    let over_bound: Vec<u32> = (1..=17).collect();
    assert_eq!(
        validate_declared(&over_bound),
        Err(BootCapabilityError::DeclaredLimitExceeded)
    );
    let protocol: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json"
    ))
    .unwrap();
    assert!(
        protocol["capabilities"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["requires"].is_array())
    );
}
