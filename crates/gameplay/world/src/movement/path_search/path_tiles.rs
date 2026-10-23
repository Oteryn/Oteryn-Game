//! Direct tile slots for the accepted 12/50-tile searches. Wider callers retain sparse
//! storage, so a large search distance never allocates its entire coordinate plane.
use super::*;
use std::collections::HashMap;

pub(super) struct TileIndex {
    left: i64,
    top: i64,
    width: usize,
    slots: Vec<usize>,
    sparse: HashMap<(i32, i32), usize>,
}

impl TileIndex {
    pub(super) fn new(start: MovementLocalPosition, distance: i32) -> Self {
        let width = if (0..=50).contains(&distance) {
            usize::try_from(distance * 2 + 1).unwrap_or(0)
        } else {
            0
        };
        Self {
            left: i64::from(start.x) - i64::from(distance),
            top: i64::from(start.y) - i64::from(distance),
            width,
            slots: vec![usize::MAX; width * width],
            sparse: HashMap::new(),
        }
    }

    #[inline]
    pub(super) fn index(&mut self, at: MovementLocalPosition, vacant: usize) -> usize {
        let x = usize::try_from(i64::from(at.x) - self.left).ok();
        let y = usize::try_from(i64::from(at.y) - self.top).ok();
        let dense = x.zip(y).filter(|(x, y)| *x < self.width && *y < self.width);
        let slot = match dense {
            Some((x, y)) => &mut self.slots[y * self.width + x],
            None => self.sparse.entry((at.x, at.y)).or_insert(usize::MAX),
        };
        if *slot == usize::MAX {
            *slot = vacant;
        }
        *slot
    }
}
