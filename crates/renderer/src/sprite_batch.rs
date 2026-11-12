//! Bounded sprite and projected overlay quads.
use super::{
    AtlasImage, BatchError, MAX_BATCH_QUADS, QuadInstance, SpriteBatch, TileCoord, TileView,
    VERTICES_PER_QUAD,
};

impl SpriteBatch {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            instances: Vec::new(),
            lighting: crate::lighting::SceneLighting {
                opacity: Vec::new(),
                ambient: [255; 3],
                sources: Vec::new(),
                palette: None,
                emissive_from: None,
                dimmed: Vec::new(),
            },
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

    /// Adds a sprite cell shifted from a tile's top-left by `offset` source pixels, where a
    /// tile is `source_px` source pixels wide; the shift scales with the view's tile size.
    /// Returns `Ok(false)` when the quad lies wholly outside the view and was culled.
    pub fn push_offset(
        &mut self,
        view: &TileView,
        atlas: &AtlasImage,
        tile: TileCoord,
        cell: u16,
        offset: [i32; 2],
        source_px: u32,
    ) -> Result<bool, BatchError> {
        let uv = atlas.uv_rect(cell)?;
        if source_px == 0 {
            return Err(BatchError::InvalidView);
        }
        let tile_px = view.tile_px() as f32;
        let scale = tile_px / source_px as f32;
        let [x, y] = view.tile_to_screen(tile);
        let position = [x + offset[0] as f32 * scale, y + offset[1] as f32 * scale];
        let (width, height) = view.viewport_px();
        if position[0] + tile_px <= 0.0
            || position[1] + tile_px <= 0.0
            || position[0] >= width as f32
            || position[1] >= height as f32
        {
            return Ok(false);
        }
        if self.instances.len() >= MAX_BATCH_QUADS {
            return Err(BatchError::CapacityExceeded {
                max: MAX_BATCH_QUADS,
            });
        }
        self.instances.push(QuadInstance {
            position,
            size: [tile_px, tile_px],
            uv,
        });
        Ok(true)
    }

    /// Appends an already projected quad, preserving the batch capacity bound.
    pub fn push_quad(&mut self, quad: QuadInstance) -> Result<(), BatchError> {
        if self.instances.len() >= MAX_BATCH_QUADS {
            return Err(BatchError::CapacityExceeded {
                max: MAX_BATCH_QUADS,
            });
        }
        self.instances.push(quad);
        Ok(())
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
