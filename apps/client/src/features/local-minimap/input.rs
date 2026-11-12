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
        let goal = {
            let map = super::state::lock(&self.minimap);
            map.position((x / 4.0) as i32, (y / 4.0) as i32)
        };
        if let Some((x, y, floor)) = goal
            && floor == self.floor
        {
            let tile = oteryn_renderer::TileCoord::new(x, y);
            if tile != self.own {
                self.walk_to(tile);
            }
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

impl PlayView {
    pub fn bind_minimap_world(&mut self, world: &str) {
        super::state::lock(&self.minimap).bind_world(world);
    }
    pub fn minimap_input(&mut self, events: &[oteryn_input_actions::NormalizedInputEvent]) -> bool {
        if oteryn_client_ui::minimap::create_mark_key(events) {
            let own = (self.own.x, self.own.y, self.floor);
            let created = super::state::lock(&self.minimap).create_mark_at(own);
            if let Err(error) = &created {
                super::state::lock(&self.minimap).feedback = error.to_string();
            }
            // A mark already sits at the player's position: edit it.
            #[cfg(windows)]
            if !self.shared_frontend && matches!(created, Ok(false)) {
                native::open(self.minimap.clone(), own);
            }
            return true;
        }
        if !oteryn_client_ui::minimap::editor_key(events) {
            return false;
        }
        #[cfg(windows)]
        if !self.shared_frontend {
            native::open(self.minimap.clone(), (self.own.x, self.own.y, self.floor));
        }
        true
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
