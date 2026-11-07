use crate::play::PlayView;

impl PlayView {
    /// Intentionally captures every click in the overlay rectangle before world selection.
    /// C24 owns negotiation and dispatch.
    pub fn minimap_click(&mut self, x: f64, y: f64) -> bool {
        if !self.live_map || !self.preferences().panel_visible("minimap") {
            return false;
        }
        // Even an unrepresentable goal at the world edge belongs to the overlay.
        if !(0.0..f64::from(
            oteryn_client_ui::minimap::COLUMNS * oteryn_client_ui::minimap::TILE_PX,
        ))
            .contains(&x)
            || !(0.0..f64::from(
                oteryn_client_ui::minimap::ROWS * oteryn_client_ui::minimap::TILE_PX,
            ))
                .contains(&y)
        {
            return false;
        }
        if let Some(tile) = oteryn_client_ui::minimap::click(self.own, x, y)
            && tile != self.own
        {
            self.walk_to(tile);
        }
        true
    }
}

impl PlayView {
    /// Route a native screen click through the overlay before selecting the world beneath it.
    pub fn click_screen(&mut self, x: f64, y: f64) -> Result<(), oteryn_renderer::BatchError> {
        if self.minimap_click(x, y) {
            return Ok(());
        }
        if let Some(tile) = crate::input::click_tile(self.scene().view(), x, y) {
            self.click(tile)?;
        }
        Ok(())
    }
}
