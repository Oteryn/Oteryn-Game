use super::*;

// Reference facts are deliberately separate from the constants under test.
// https://www.tibia.com/gameguides/?subtopic=manual&section=world
// https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/src/items/containers/depot/depotchest.cpp
#[test]
fn depot_reference_parity_limits() {
    for (rule, observed, reference) in [
        ("fixed depot boxes", u32::from(DEPOT_BOXES), 17),
        ("free account entries", DEPOT_FREE_ENTRIES, 2_000),
        ("Premium / database ceiling", DEPOT_HARD_ENTRIES, 15_000),
        ("one paginated chest page", DEPOT_PAGE_ENTRIES, 32),
        ("one whole item or stack", DEPOT_WHOLE_ITEM_ENTRIES, 1),
    ] {
        assert_eq!(observed, reference, "{rule}");
    }
}
