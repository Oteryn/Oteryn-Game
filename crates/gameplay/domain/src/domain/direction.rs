//! Shared cardinal movement offsets in North, East, South, West order.

/// Tile offsets indexed by the North, East, South, West cardinal direction order.
pub const CARDINAL_DIRECTION_DELTAS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];
