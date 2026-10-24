//! Oteryn default, not Reference-qualified: authored square around the bundle entry.
//! Only its reachable served tiles are members. No source mask is treated as native geometry.
use super::*;
use std::collections::{BTreeSet, VecDeque};
pub(in crate::gameplay_transport) struct AdmittedArea {
    pub digest: [u8; 32],
    pub pool: Vec<Point>,
    pub tiles: BTreeSet<Point>,
}
pub(super) fn admit(owner: &ComposedFreshAdmission<'_, '_, '_>) -> Option<AdmittedArea> {
    let area = BasicArea::embedded()?;
    let native = owner.active_generation?.native_gameplay()?;
    let index = native.item_index()?;
    for key in [
        "oteryn:item.tibia.i2854",
        "oteryn:item.tibia.i3031",
        "oteryn:item.tibia.i3035",
        "oteryn:item.tibia.i3043",
    ] {
        index.revision(key)?;
    }

    let map = owner.bundle_map()?;
    let entry = map.entry_start;
    let start = Point {
        x: i32::from(entry.x),
        y: i32::from(entry.y),
        floor: i16::from(entry.floor),
    };
    let walkable = |p: Point| {
        let (Ok(x), Ok(y), Ok(floor)) = (
            u16::try_from(p.x),
            u16::try_from(p.y),
            i8::try_from(p.floor),
        ) else {
            return false;
        };
        // Oteryn default, not Reference-qualified: exclude door tiles in both states,
        // so opening a door cannot reinterpret durable POI pools or area membership.
        if map
            .facts
            .doors()
            .at(crate::map::overlay::TilePos { x, y, floor })
        {
            return false;
        }

        let cell = LogicalCell {
            x: p.x,
            y: p.y,
            z: i32::from(-p.floor),
        };
        owner
            .movement_cells
            .index()
            .lookup(owner.movement_cells.scope(), cell)
            == Ok(CollisionClass::Walkable)
    };
    if !walkable(start) {
        return None;
    }
    let mut seen = BTreeSet::from([start]);
    let mut queue = VecDeque::from([start]);
    while let Some(point) = queue.pop_front() {
        for (dx, dy) in [(-1, 0), (0, -1), (0, 1), (1, 0)] {
            let next = Point {
                x: point.x.checked_add(dx)?,
                y: point.y.checked_add(dy)?,
                floor: point.floor,
            };
            if next.x.abs_diff(start.x) > area.radius as u32
                || next.y.abs_diff(start.y) > area.radius as u32
                || seen.contains(&next)
                || !walkable(next)
            {
                continue;
            }
            if seen.len() >= 4225 {
                return None;
            }
            seen.insert(next);
            queue.push_back(next);
        }
    }
    let pool: Vec<_> = seen
        .iter()
        .copied()
        .filter(|p| !p.in_window(start))
        .take(area.pool_limit)
        .collect();
    if pool.len() < area.generated_count {
        return None;
    }
    // Pin exactly the admitted bundle. A changed geometry fails closed when state exists.
    Some(AdmittedArea {
        digest: map.overlay.base().digest(),
        pool,
        tiles: seen,
    })
}
