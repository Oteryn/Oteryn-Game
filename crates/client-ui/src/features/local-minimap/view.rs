//! Fixed native overlay geometry, shared by drawing and pointer projection.
use oteryn_renderer::{BatchError, TileCoord, TileView};

pub const COLUMNS: u32 = 31;
pub const ROWS: u32 = 25;
pub const TILE_PX: u32 = 4;

pub const MIN_ZOOM: i32 = 1;
pub const MAX_ZOOM: i32 = 8;
pub const LOWEST_FLOOR: i16 = -15;
pub const HIGHEST_FLOOR: i16 = 0;

/// Zoom in halves the tiles per cell, never below one.
pub fn zoom_in(zoom: i32) -> i32 {
    (zoom / 2).max(MIN_ZOOM)
}
/// Zoom out doubles the tiles per cell, never above eight.
pub fn zoom_out(zoom: i32) -> i32 {
    zoom.saturating_mul(2).min(MAX_ZOOM)
}
/// One floor towards the surface; the surface is floor zero.
pub fn floor_up(floor: i16) -> i16 {
    floor.saturating_add(1).min(HIGHEST_FLOOR)
}
/// One floor deeper, never below the lowest map floor.
pub fn floor_down(floor: i16) -> i16 {
    floor.saturating_sub(1).max(LOWEST_FLOOR)
}

pub fn view() -> Result<TileView, BatchError> {
    TileView::new(TileCoord::new(0, 0), TILE_PX, COLUMNS, ROWS)
}

/// Convert a panel cell to an absolute position without wrapping at world edges.
pub fn position(own: TileCoord, cell: TileCoord) -> Option<TileCoord> {
    if !(0..COLUMNS as i32).contains(&cell.x) || !(0..ROWS as i32).contains(&cell.y) {
        return None;
    }
    Some(TileCoord::new(
        own.x.checked_add(cell.x - (COLUMNS / 2) as i32)?,
        own.y.checked_add(cell.y - (ROWS / 2) as i32)?,
    ))
}

pub fn click(own: TileCoord, x: f64, y: f64) -> Option<TileCoord> {
    if !x.is_finite()
        || !y.is_finite()
        || x < 0.0
        || y < 0.0
        || x >= f64::from(COLUMNS * TILE_PX)
        || y >= f64::from(ROWS * TILE_PX)
    {
        return None;
    }
    let cell = TileCoord::new(
        (x / f64::from(TILE_PX)) as i32,
        (y / f64::from(TILE_PX)) as i32,
    );
    position(own, cell)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t076_zoom_and_floor_controls_clamp_at_their_limits() {
        assert_eq!(zoom_out(1), 2);
        assert_eq!(zoom_out(4), 8);
        assert_eq!(zoom_out(8), 8);
        assert_eq!(zoom_in(8), 4);
        assert_eq!(zoom_in(2), 1);
        assert_eq!(zoom_in(1), 1);
        assert_eq!(floor_up(-7), -6);
        assert_eq!(floor_up(0), 0);
        assert_eq!(floor_down(-7), -8);
        assert_eq!(floor_down(-15), -15);
        let mut zoom = 1;
        for _ in 0..6 {
            zoom = zoom_out(zoom);
        }
        assert_eq!(zoom, MAX_ZOOM);
        let mut floor = 0;
        for _ in 0..20 {
            floor = floor_down(floor);
        }
        assert_eq!(floor, LOWEST_FLOOR);
    }
    #[test]
    fn center_edges_and_invalid_coordinates() {
        let own = TileCoord::new(100, 200);
        assert_eq!(click(own, 62.0, 50.0), Some(own));
        assert_eq!(click(own, 0.0, 0.0), Some(TileCoord::new(85, 188)));
        for (x, y) in [
            (124.0, 0.0),
            (0.0, 100.0),
            (-0.01, 0.0),
            (f64::NAN, 0.0),
            (f64::INFINITY, 0.0),
        ] {
            assert_eq!(click(own, x, y), None);
        }
        assert_eq!(click(TileCoord::new(i32::MAX, 0), 66.0, 50.0), None);
        assert_eq!(click(TileCoord::new(i32::MIN, 0), 58.0, 50.0), None);
    }
}
