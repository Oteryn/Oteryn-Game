//! GPU-free construction of the 2D tile and sprite batches (ADR-0020 section 4, child N2).
//!
//! Positions are in surface pixels with the origin at the top-left and Y growing downwards, the
//! same space as window cursor positions. Everything here is plain data and integer or float math
//! so batch shape, counts and tile/screen conversion are testable without a GPU.

use std::fmt::{self, Display, Formatter};

/// Upper bound of quads in one batch. Bounds both memory and the GPU instance buffer.
pub const MAX_BATCH_QUADS: usize = 16_384;
/// Vertices the GPU draws per quad instance (two triangles).
pub const VERTICES_PER_QUAD: u32 = 6;
/// Largest atlas edge in pixels.
pub const MAX_ATLAS_DIMENSION: u32 = 4096;
/// Largest on-screen extent of a view in pixels; keeps pixel coordinates exact in `f32`.
pub const MAX_VIEW_PIXELS: u32 = 1 << 16;
/// Size in bytes of one serialized [`QuadInstance`].
pub const QUAD_INSTANCE_BYTES: usize = 32;

const RGBA_BYTES_PER_PIXEL: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchError {
    CapacityExceeded { max: usize },
    CellOutOfRange { cell: u16 },
    CellCountMismatch { expected: usize, actual: usize },
    InvalidAtlas,
    InvalidView,
}

impl Display for BatchError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapacityExceeded { max } => {
                write!(formatter, "render batch exceeds its bound of {max} quads")
            }
            Self::CellOutOfRange { cell } => {
                write!(formatter, "atlas cell {cell} is outside the atlas")
            }
            Self::CellCountMismatch { expected, actual } => write!(
                formatter,
                "tile grid needs {expected} cells but {actual} were supplied"
            ),
            Self::InvalidAtlas => formatter.write_str("atlas dimensions or pixel data are invalid"),
            Self::InvalidView => formatter.write_str("tile view dimensions are invalid"),
        }
    }
}

impl std::error::Error for BatchError {}

/// Signed tile coordinate. The renderer treats it as opaque grid position, not as a world id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileCoord {
    pub x: i32,
    pub y: i32,
}

impl TileCoord {
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// One textured quad: screen rectangle plus the atlas rectangle `[u0, v0, u1, v1]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuadInstance {
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub uv: [f32; 4],
}

impl QuadInstance {
    /// Native-endian bytes in the exact field order the vertex layout reads.
    #[must_use]
    pub fn to_bytes(self) -> [u8; QUAD_INSTANCE_BYTES] {
        let mut bytes = [0_u8; QUAD_INSTANCE_BYTES];
        let floats = [
            self.position[0],
            self.position[1],
            self.size[0],
            self.size[1],
            self.uv[0],
            self.uv[1],
            self.uv[2],
            self.uv[3],
        ];
        for (chunk, value) in bytes.chunks_exact_mut(4).zip(floats) {
            chunk.copy_from_slice(&value.to_ne_bytes());
        }
        bytes
    }
}

/// Serializes instances for upload to the GPU instance buffer.
#[must_use]
pub fn instance_bytes(instances: &[QuadInstance]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(instances.len() * QUAD_INSTANCE_BYTES);
    for instance in instances {
        bytes.extend_from_slice(&instance.to_bytes());
    }
    bytes
}

/// RGBA8 atlas of square cells addressed by a row-major cell index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtlasImage {
    cell_px: u32,
    columns: u32,
    rows: u32,
    rgba: Vec<u8>,
}

impl AtlasImage {
    pub fn new(cell_px: u32, columns: u32, rows: u32, rgba: Vec<u8>) -> Result<Self, BatchError> {
        if cell_px == 0 || columns == 0 || rows == 0 {
            return Err(BatchError::InvalidAtlas);
        }
        let width = cell_px
            .checked_mul(columns)
            .ok_or(BatchError::InvalidAtlas)?;
        let height = cell_px.checked_mul(rows).ok_or(BatchError::InvalidAtlas)?;
        if width > MAX_ATLAS_DIMENSION || height > MAX_ATLAS_DIMENSION {
            return Err(BatchError::InvalidAtlas);
        }
        let expected = (width as usize) * (height as usize) * RGBA_BYTES_PER_PIXEL;
        if rgba.len() != expected {
            return Err(BatchError::InvalidAtlas);
        }
        Ok(Self {
            cell_px,
            columns,
            rows,
            rgba,
        })
    }

    #[must_use]
    pub const fn cell_px(&self) -> u32 {
        self.cell_px
    }

    #[must_use]
    pub const fn width(&self) -> u32 {
        self.cell_px * self.columns
    }

    #[must_use]
    pub const fn height(&self) -> u32 {
        self.cell_px * self.rows
    }

    #[must_use]
    pub const fn cell_count(&self) -> u32 {
        self.columns * self.rows
    }

    #[must_use]
    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }

    /// Atlas rectangle `[u0, v0, u1, v1]` of a cell, in normalized texture coordinates.
    pub fn uv_rect(&self, cell: u16) -> Result<[f32; 4], BatchError> {
        let index = u32::from(cell);
        if index >= self.cell_count() {
            return Err(BatchError::CellOutOfRange { cell });
        }
        let column = index % self.columns;
        let row = index / self.columns;
        let columns = self.columns as f32;
        let rows = self.rows as f32;
        Ok([
            column as f32 / columns,
            row as f32 / rows,
            (column + 1) as f32 / columns,
            (row + 1) as f32 / rows,
        ])
    }
}

/// Rectangle of `columns * rows` tiles whose top-left tile is `origin`, drawn from screen (0, 0)
/// at `tile_px` pixels per tile. It is the single source of tile/screen conversion, so a click
/// (N3) and a drawn sprite (N6) agree on the same tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileView {
    origin: TileCoord,
    tile_px: u32,
    columns: u32,
    rows: u32,
}

impl TileView {
    pub fn new(
        origin: TileCoord,
        tile_px: u32,
        columns: u32,
        rows: u32,
    ) -> Result<Self, BatchError> {
        if tile_px == 0 || columns == 0 || rows == 0 {
            return Err(BatchError::InvalidView);
        }
        let quads = (columns as usize).saturating_mul(rows as usize);
        if quads > MAX_BATCH_QUADS {
            return Err(BatchError::CapacityExceeded {
                max: MAX_BATCH_QUADS,
            });
        }
        let width = tile_px.checked_mul(columns);
        let height = tile_px.checked_mul(rows);
        let (Some(width), Some(height)) = (width, height) else {
            return Err(BatchError::InvalidView);
        };
        if width > MAX_VIEW_PIXELS || height > MAX_VIEW_PIXELS {
            return Err(BatchError::InvalidView);
        }
        // `columns` and `rows` are bounded by the quad limit, so they fit in `i32`.
        let last_x = i32::try_from(columns)
            .ok()
            .and_then(|c| origin.x.checked_add(c));
        let last_y = i32::try_from(rows)
            .ok()
            .and_then(|r| origin.y.checked_add(r));
        if last_x.is_none() || last_y.is_none() {
            return Err(BatchError::InvalidView);
        }
        Ok(Self {
            origin,
            tile_px,
            columns,
            rows,
        })
    }

    #[must_use]
    pub const fn origin(&self) -> TileCoord {
        self.origin
    }

    #[must_use]
    pub const fn tile_px(&self) -> u32 {
        self.tile_px
    }

    #[must_use]
    pub const fn columns(&self) -> u32 {
        self.columns
    }

    #[must_use]
    pub const fn rows(&self) -> u32 {
        self.rows
    }

    /// On-screen size of the whole view in pixels.
    #[must_use]
    pub const fn viewport_px(&self) -> (u32, u32) {
        (self.tile_px * self.columns, self.tile_px * self.rows)
    }

    /// True when the tile lies inside the view.
    #[must_use]
    pub fn contains(&self, tile: TileCoord) -> bool {
        let dx = i64::from(tile.x) - i64::from(self.origin.x);
        let dy = i64::from(tile.y) - i64::from(self.origin.y);
        (0..i64::from(self.columns)).contains(&dx) && (0..i64::from(self.rows)).contains(&dy)
    }

    /// Top-left screen pixel of a tile. Tiles outside the view give off-screen positions.
    #[must_use]
    pub fn tile_to_screen(&self, tile: TileCoord) -> [f32; 2] {
        let dx = i64::from(tile.x) - i64::from(self.origin.x);
        let dy = i64::from(tile.y) - i64::from(self.origin.y);
        let tile_px = i64::from(self.tile_px);
        [(dx * tile_px) as f32, (dy * tile_px) as f32]
    }

    /// Tile under a screen pixel, or `None` outside the view or for a non-finite position.
    #[must_use]
    pub fn screen_to_tile(&self, x: f32, y: f32) -> Option<TileCoord> {
        if !x.is_finite() || !y.is_finite() || x < 0.0 || y < 0.0 {
            return None;
        }
        let (width, height) = self.viewport_px();
        if x >= width as f32 || y >= height as f32 {
            return None;
        }
        // Both are below `MAX_VIEW_PIXELS`, so the truncating casts are exact floors.
        let column = (x as u32) / self.tile_px;
        let row = (y as u32) / self.tile_px;
        let column = i32::try_from(column).ok()?;
        let row = i32::try_from(row).ok()?;
        Some(TileCoord::new(
            self.origin.x.checked_add(column)?,
            self.origin.y.checked_add(row)?,
        ))
    }
}

/// Opaque-terrain batch: one quad for every tile of a view, row-major.
#[derive(Debug, Clone, PartialEq)]
pub struct TileBatch {
    instances: Vec<QuadInstance>,
}

impl TileBatch {
    /// Builds the batch from `view.columns() * view.rows()` atlas cell indices, row-major.
    pub fn from_cells(
        view: &TileView,
        atlas: &AtlasImage,
        cells: &[u16],
    ) -> Result<Self, BatchError> {
        let expected = (view.columns() as usize) * (view.rows() as usize);
        if cells.len() != expected {
            return Err(BatchError::CellCountMismatch {
                expected,
                actual: cells.len(),
            });
        }
        let tile_px = view.tile_px() as f32;
        let mut instances = Vec::with_capacity(expected);
        for (index, cell) in cells.iter().enumerate() {
            let column = (index % view.columns() as usize) as f32;
            let row = (index / view.columns() as usize) as f32;
            instances.push(QuadInstance {
                position: [column * tile_px, row * tile_px],
                size: [tile_px, tile_px],
                uv: atlas.uv_rect(*cell)?,
            });
        }
        Ok(Self { instances })
    }

    #[must_use]
    pub fn instances(&self) -> &[QuadInstance] {
        &self.instances
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.instances.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    /// Vertices the GPU draws for this batch.
    #[must_use]
    pub fn vertex_count(&self) -> u32 {
        // Bounded by `MAX_BATCH_QUADS`, so the product fits in `u32`.
        (self.instances.len() as u32) * VERTICES_PER_QUAD
    }
}

/// Alpha-blended sprite batch: one tile-sized quad per sprite, in draw order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpriteBatch {
    instances: Vec<QuadInstance>,
}

impl SpriteBatch {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            instances: Vec::new(),
        }
    }

    /// Adds a sprite on a tile. Returns `Ok(false)` when the tile is outside the view and the
    /// sprite was culled. An invalid cell or a full batch is an error.
    pub fn push(
        &mut self,
        view: &TileView,
        atlas: &AtlasImage,
        tile: TileCoord,
        cell: u16,
    ) -> Result<bool, BatchError> {
        let uv = atlas.uv_rect(cell)?;
        if !view.contains(tile) {
            return Ok(false);
        }
        if self.instances.len() >= MAX_BATCH_QUADS {
            return Err(BatchError::CapacityExceeded {
                max: MAX_BATCH_QUADS,
            });
        }
        let tile_px = view.tile_px() as f32;
        self.instances.push(QuadInstance {
            position: view.tile_to_screen(tile),
            size: [tile_px, tile_px],
            uv,
        });
        Ok(true)
    }

    pub fn clear(&mut self) {
        self.instances.clear();
    }

    #[must_use]
    pub fn instances(&self) -> &[QuadInstance] {
        &self.instances
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.instances.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    /// Vertices the GPU draws for this batch.
    #[must_use]
    pub fn vertex_count(&self) -> u32 {
        // Bounded by `MAX_BATCH_QUADS`, so the product fits in `u32`.
        (self.instances.len() as u32) * VERTICES_PER_QUAD
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atlas() -> Result<AtlasImage, BatchError> {
        // 4 x 2 cells of 2 px.
        AtlasImage::new(2, 4, 2, vec![0; 8 * 4 * 4])
    }

    fn view() -> Result<TileView, BatchError> {
        TileView::new(TileCoord::new(-3, 10), 32, 5, 4)
    }

    #[test]
    fn atlas_validates_dimensions_and_pixel_length() {
        assert_eq!(
            AtlasImage::new(0, 1, 1, vec![]),
            Err(BatchError::InvalidAtlas)
        );
        assert_eq!(
            AtlasImage::new(2, 4, 2, vec![0; 3]),
            Err(BatchError::InvalidAtlas)
        );
        assert_eq!(
            AtlasImage::new(MAX_ATLAS_DIMENSION, 2, 1, vec![]),
            Err(BatchError::InvalidAtlas)
        );
        assert_eq!(
            AtlasImage::new(u32::MAX, 2, 1, vec![]),
            Err(BatchError::InvalidAtlas)
        );
        assert!(atlas().is_ok());
    }

    #[test]
    fn uv_rects_are_row_major_normalized_and_bounded() -> Result<(), BatchError> {
        let atlas = atlas()?;
        assert_eq!(
            (atlas.width(), atlas.height(), atlas.cell_count()),
            (8, 4, 8)
        );
        assert_eq!(atlas.uv_rect(0)?, [0.0, 0.0, 0.25, 0.5]);
        assert_eq!(atlas.uv_rect(3)?, [0.75, 0.0, 1.0, 0.5]);
        assert_eq!(atlas.uv_rect(4)?, [0.0, 0.5, 0.25, 1.0]);
        assert_eq!(atlas.uv_rect(7)?, [0.75, 0.5, 1.0, 1.0]);
        assert_eq!(
            atlas.uv_rect(8),
            Err(BatchError::CellOutOfRange { cell: 8 })
        );
        Ok(())
    }

    #[test]
    fn view_rejects_degenerate_oversized_and_overflowing_shapes() {
        let origin = TileCoord::new(0, 0);
        assert_eq!(TileView::new(origin, 0, 1, 1), Err(BatchError::InvalidView));
        assert_eq!(TileView::new(origin, 1, 0, 1), Err(BatchError::InvalidView));
        assert_eq!(
            TileView::new(origin, 1, 129, 128),
            Err(BatchError::CapacityExceeded {
                max: MAX_BATCH_QUADS
            })
        );
        assert_eq!(
            TileView::new(origin, 4100, 16, 1),
            Err(BatchError::InvalidView)
        );
        assert_eq!(
            TileView::new(TileCoord::new(i32::MAX, 0), 8, 2, 2),
            Err(BatchError::InvalidView)
        );
        assert!(TileView::new(origin, 8, 128, 128).is_ok());
    }

    #[test]
    fn tile_to_screen_is_relative_to_the_view_origin() -> Result<(), BatchError> {
        let view = view()?;
        assert_eq!(view.viewport_px(), (160, 128));
        assert_eq!(view.tile_to_screen(TileCoord::new(-3, 10)), [0.0, 0.0]);
        assert_eq!(view.tile_to_screen(TileCoord::new(0, 12)), [96.0, 64.0]);
        assert_eq!(view.tile_to_screen(TileCoord::new(-4, 9)), [-32.0, -32.0]);
        Ok(())
    }

    #[test]
    fn screen_to_tile_inverts_tile_to_screen_for_every_tile() -> Result<(), BatchError> {
        let view = view()?;
        for row in 0..4 {
            for column in 0..5 {
                let tile = TileCoord::new(-3 + column, 10 + row);
                let [x, y] = view.tile_to_screen(tile);
                assert_eq!(view.screen_to_tile(x, y), Some(tile));
                assert_eq!(view.screen_to_tile(x + 31.9, y + 31.9), Some(tile));
            }
        }
        Ok(())
    }

    #[test]
    fn screen_to_tile_rejects_outside_and_non_finite_positions() -> Result<(), BatchError> {
        let view = view()?;
        for (x, y) in [
            (-0.1, 0.0),
            (0.0, -0.1),
            (160.0, 0.0),
            (0.0, 128.0),
            (f32::NAN, 0.0),
            (0.0, f32::INFINITY),
            (f32::NEG_INFINITY, 5.0),
        ] {
            assert_eq!(view.screen_to_tile(x, y), None, "({x}, {y})");
        }
        assert_eq!(
            view.screen_to_tile(159.9, 127.9),
            Some(TileCoord::new(1, 13))
        );
        assert!(view.contains(TileCoord::new(1, 13)));
        assert!(!view.contains(TileCoord::new(2, 13)));
        assert!(!view.contains(TileCoord::new(-4, 10)));
        Ok(())
    }

    #[test]
    fn tile_batch_has_one_quad_per_tile_in_row_major_order() -> Result<(), BatchError> {
        let (atlas, view) = (atlas()?, view()?);
        let cells: Vec<u16> = (0..20).map(|index| index % 8).collect();
        let batch = TileBatch::from_cells(&view, &atlas, &cells)?;
        assert_eq!(batch.len(), 20);
        assert!(!batch.is_empty());
        assert_eq!(batch.vertex_count(), 120);
        let second_row_first = batch.instances()[5];
        assert_eq!(second_row_first.position, [0.0, 32.0]);
        assert_eq!(second_row_first.size, [32.0, 32.0]);
        assert_eq!(second_row_first.uv, atlas.uv_rect(5)?);
        let last = batch.instances()[19];
        assert_eq!(last.position, [128.0, 96.0]);
        assert_eq!(last.uv, atlas.uv_rect(3)?);
        Ok(())
    }

    #[test]
    fn tile_batch_rejects_wrong_cell_count_and_bad_cells() -> Result<(), BatchError> {
        let (atlas, view) = (atlas()?, view()?);
        assert_eq!(
            TileBatch::from_cells(&view, &atlas, &[0; 19]),
            Err(BatchError::CellCountMismatch {
                expected: 20,
                actual: 19
            })
        );
        let mut cells = [0_u16; 20];
        cells[7] = 8;
        assert_eq!(
            TileBatch::from_cells(&view, &atlas, &cells),
            Err(BatchError::CellOutOfRange { cell: 8 })
        );
        Ok(())
    }

    #[test]
    fn sprite_batch_places_sprites_on_tiles_and_culls_outside_the_view() -> Result<(), BatchError> {
        let (atlas, view) = (atlas()?, view()?);
        let mut sprites = SpriteBatch::new();
        assert!(sprites.is_empty());
        assert_eq!(
            sprites.push(&view, &atlas, TileCoord::new(0, 12), 6),
            Ok(true)
        );
        assert_eq!(
            sprites.push(&view, &atlas, TileCoord::new(2, 12), 6),
            Ok(false)
        );
        assert_eq!(
            sprites.push(&view, &atlas, TileCoord::new(-3, 10), 4),
            Ok(true)
        );
        assert_eq!(sprites.len(), 2);
        assert_eq!(sprites.vertex_count(), 12);
        let first = sprites.instances()[0];
        assert_eq!(first.position, [96.0, 64.0]);
        assert_eq!(first.size, [32.0, 32.0]);
        assert_eq!(first.uv, atlas.uv_rect(6)?);
        assert_eq!(sprites.instances()[1].position, [0.0, 0.0]);
        sprites.clear();
        assert!(sprites.is_empty());
        Ok(())
    }

    #[test]
    fn sprite_batch_rejects_bad_cells_even_when_culled_and_enforces_capacity()
    -> Result<(), BatchError> {
        let (atlas, view) = (atlas()?, view()?);
        let mut sprites = SpriteBatch::new();
        assert_eq!(
            sprites.push(&view, &atlas, TileCoord::new(100, 100), 99),
            Err(BatchError::CellOutOfRange { cell: 99 })
        );
        for _ in 0..MAX_BATCH_QUADS {
            assert_eq!(
                sprites.push(&view, &atlas, TileCoord::new(0, 10), 0),
                Ok(true)
            );
        }
        assert_eq!(
            sprites.push(&view, &atlas, TileCoord::new(0, 10), 0),
            Err(BatchError::CapacityExceeded {
                max: MAX_BATCH_QUADS
            })
        );
        assert_eq!(sprites.len(), MAX_BATCH_QUADS);
        Ok(())
    }

    #[test]
    fn instance_bytes_follow_the_declared_field_order() {
        let instance = QuadInstance {
            position: [1.0, 2.0],
            size: [3.0, 4.0],
            uv: [5.0, 6.0, 7.0, 8.0],
        };
        let bytes = instance_bytes(&[instance, instance]);
        assert_eq!(bytes.len(), 2 * QUAD_INSTANCE_BYTES);
        let floats: Vec<f32> = bytes[..QUAD_INSTANCE_BYTES]
            .chunks_exact(4)
            .map(|chunk| f32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();
        assert_eq!(floats, [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
        assert_eq!(bytes[..QUAD_INSTANCE_BYTES], bytes[QUAD_INSTANCE_BYTES..]);
    }
}
