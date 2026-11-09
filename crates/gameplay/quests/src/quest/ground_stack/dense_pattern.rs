//! A larger readset is bounded by both its dense geometry and single-item cells.
use super::{GroundStackRequirement, GroundStackSnapshot};
use std::collections::BTreeSet;

const SIDE: u16 = 6;
const CELLS: usize = 36;

pub(super) fn admitted(
    requirements: &[GroundStackRequirement],
    snapshots: &[GroundStackSnapshot],
) -> bool {
    if requirements.len() != CELLS || snapshots.len() > CELLS {
        return false;
    }
    let Some(first) = requirements.first() else {
        return false;
    };
    if requirements.iter().any(|r| {
        r.position.floor != first.position.floor || r.items.len() != 1 || r.items[0].quantity != 1
    }) || snapshots
        .iter()
        .any(|s| s.entries.len() > 1 || s.entries.iter().any(|entry| entry.quantity != 1))
    {
        return false;
    }
    let positions: BTreeSet<_> = requirements.iter().map(|r| r.position).collect();
    let (min_x, max_x, min_y, max_y) = requirements.iter().fold(
        (
            first.position.x,
            first.position.x,
            first.position.y,
            first.position.y,
        ),
        |(min_x, max_x, min_y, max_y), r| {
            (
                min_x.min(r.position.x),
                max_x.max(r.position.x),
                min_y.min(r.position.y),
                max_y.max(r.position.y),
            )
        },
    );
    positions.len() == CELLS
        && max_x.checked_sub(min_x) == Some(SIDE - 1)
        && max_y.checked_sub(min_y) == Some(SIDE - 1)
}
