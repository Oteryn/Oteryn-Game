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
