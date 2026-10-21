//! Pure value construction for transaction-bound Wheel reads.

use crate::wheel::{WHEEL_SLOTS, WheelAllocation, WheelInputError, WheelSlots};
use std::collections::BTreeMap;

/// The stored revision against the active one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Currency {
    Current,
    NeedsReset,
    Unresolved,
}

/// Resolve registered revisions in forward order before a storage adapter reads capacities.
pub fn revision_range(
    order: &BTreeMap<String, (i32, String)>,
    stored: &str,
    active: &str,
) -> Option<(i32, i32)> {
    let (Some((stored_ordinal, _)), Some((active_ordinal, _))) =
        (order.get(stored), order.get(active))
    else {
        return None;
    };
    (*stored_ordinal <= *active_ordinal).then_some((*stored_ordinal, *active_ordinal))
}

/// Evaluate reset revisions after the stored revision up to and including the active revision.
pub fn derive_currency(
    order: &BTreeMap<String, (i32, String)>,
    stored_ordinal: i32,
    active_ordinal: i32,
    capacities_match: bool,
) -> Currency {
    if !capacities_match {
        return Currency::Unresolved;
    }
    let reset = order.values().any(|(ordinal, kind)| {
        *ordinal > stored_ordinal && *ordinal <= active_ordinal && kind == "RESET"
    });
    if reset {
        Currency::NeedsReset
    } else {
        Currency::Current
    }
}

/// Construct an allocation from independently read slots, capacities and revision currency.
pub fn read_allocation(
    stored: Option<(String, u64, WheelSlots, [u16; WHEEL_SLOTS])>,
    current: bool,
) -> Result<WheelAllocation, WheelInputError> {
    Ok(match stored {
        None => WheelAllocation {
            wheel_ruleset_revision: None,
            wheel_revision: 0,
            slots: WheelSlots::ZERO,
            current,
        },
        Some((revision, wheel_revision, slots, capacities)) => {
            if slots
                .points()
                .iter()
                .zip(capacities)
                .any(|(points, capacity)| *points > capacity)
            {
                return Err(WheelInputError);
            }
            WheelAllocation {
                wheel_ruleset_revision: Some(revision),
                wheel_revision,
                slots,
                current,
            }
        }
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn registered_range_and_reset_interval_are_exact() {
        let order = BTreeMap::from([
            ("r1".to_owned(), (1, "RESET".to_owned())),
            ("r2".to_owned(), (2, "VALUE".to_owned())),
            ("r3".to_owned(), (3, "RESET".to_owned())),
            ("r4".to_owned(), (4, "VALUE".to_owned())),
        ]);
        assert_eq!(revision_range(&order, "missing", "r4"), None);
        assert_eq!(revision_range(&order, "r1", "missing"), None);
        assert_eq!(revision_range(&order, "r4", "r1"), None);
        assert_eq!(revision_range(&order, "r1", "r2"), Some((1, 2)));
        assert_eq!(derive_currency(&order, 1, 2, true), Currency::Current);
        assert_eq!(derive_currency(&order, 1, 3, true), Currency::NeedsReset);
        assert_eq!(derive_currency(&order, 3, 4, true), Currency::Current);
        assert_eq!(derive_currency(&order, 3, 4, false), Currency::Unresolved);
    }

    #[test]
    fn allocation_checks_independent_capacities_and_retains_stale_values() {
        let empty = read_allocation(None, false).expect("no stored allocation");
        assert_eq!(empty.wheel_ruleset_revision, None);
        assert_eq!(empty.wheel_revision, 0);
        assert_eq!(empty.slots, WheelSlots::ZERO);
        assert!(!empty.current);
        let mut points = [0; WHEEL_SLOTS];
        points[35] = 50;
        let slots = WheelSlots::new(points).expect("bounded slots");
        let stored = Some(("r1".to_owned(), 7, slots, [50; WHEEL_SLOTS]));
        let allocation = read_allocation(stored, false).expect("valid stale allocation");
        assert_eq!(allocation.wheel_ruleset_revision.as_deref(), Some("r1"));
        assert_eq!(allocation.wheel_revision, 7);
        assert_eq!(allocation.slots, slots);
        assert!(!allocation.current);
        assert!(
            read_allocation(Some(("r1".to_owned(), 7, slots, [49; WHEEL_SLOTS])), true).is_err()
        );
        assert!(
            read_allocation(None, true)
                .expect("empty current allocation")
                .current
        );
    }
}
