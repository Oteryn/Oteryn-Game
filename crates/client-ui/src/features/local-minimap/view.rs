//! Fixed native overlay geometry, shared by drawing and pointer projection.
use oteryn_renderer::{BatchError, TileCoord, TileView};

pub const COLUMNS: u32 = 31;
pub const ROWS: u32 = 25;
pub const TILE_PX: u32 = 4;

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
