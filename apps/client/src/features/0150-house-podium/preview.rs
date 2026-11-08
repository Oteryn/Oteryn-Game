//! Preview plans carry only active-catalogue metadata received from the server.
use oteryn_client_runtime::house_podium_display::State;
pub fn tile_view() -> Result<oteryn_renderer::TileView, oteryn_renderer::BatchError> {
    use oteryn_renderer::{TileCoord, TileView};
    TileView::new(TileCoord::new(0, 0), 64, 5, 5)
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layer {
    pub kind: u8,
    pub appearance: u32,
}
pub fn layers(state: &State) -> Result<Vec<Layer>, &'static str> {
    let mut layers = Vec::new();
    if state.visibility.platform {
        if state.platform_appearance == 0 {
            return Err("The active catalogue has no platform appearance");
        }
        layers.push(Layer {
            kind: 2,
            appearance: state.platform_appearance,
        });
    }
    if state.visibility.display && state.projection.is_some() {
        if state.appearance == 0 || state.appearance_kind == 0 {
            return Err("The active catalogue has no display appearance");
        }
        layers.push(Layer {
            kind: state.appearance_kind,
            appearance: state.appearance,
        });
    }
    Ok(layers)
}
#[cfg(windows)]
#[path = "preview_native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;

#[cfg(test)]
mod tests {
    #[test]
    fn podium_preview_grid_accepts_its_real_background_batch()
    -> Result<(), oteryn_renderer::BatchError> {
        use oteryn_renderer::{AtlasImage, TileBatch, world::BLACK_CELL};
        let view = super::tile_view()?;
        let atlas = AtlasImage::new(1, 1, 1, vec![0, 0, 0, 255])?;
        let tiles = TileBatch::from_cells(&view, &atlas, &[BLACK_CELL; 25])?;
        assert_eq!(tiles.len(), 25);
        assert_eq!(view.viewport_px(), (320, 320));
        Ok(())
    }
}
