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
    #[cfg(feature = "bevy-shell")]
    pub(crate) fn minimap_mark_at(&mut self, x: f64, y: f64) {
        if !x.is_finite()
            || !y.is_finite()
            || !(0.0..124.0).contains(&x)
            || !(0.0..100.0).contains(&y)
        {
            return;
        }
        let mut map = super::state::lock(&self.minimap);
        if let Some(position) = map.position((x / 4.0) as i32, (y / 4.0) as i32)
            && let Err(error) = map.create_mark_at(position)
        {
            map.feedback = error.to_string();
        }
    }
    /// Route a native screen click through the overlay before selecting the world beneath it.
    pub fn click_screen(&mut self, x: f64, y: f64) -> Result<(), oteryn_renderer::BatchError> {
        if self.minimap_click(x, y) {
            return Ok(());
        }
        if let Some(tile) = self.click_tile(x, y) {
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
        let edit = oteryn_client_ui::minimap::editor_key(events);
        #[cfg(windows)]
        if edit && !self.shared_frontend {
            native::open(self.minimap.clone(), (self.own.x, self.own.y, self.floor));
        }
        edit
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;

impl PlayView {
    /// Current player position for a separately navigable overview.
    pub fn satellite_player(&self) -> (i32, i32, i16) {
        (self.own.x, self.own.y, self.floor)
    }
    /// Read local exploration and markers without changing sidebar navigation.
    pub fn satellite_map<R>(
        &self,
        read: impl FnOnce(
            &std::collections::BTreeMap<(i16, i32, i32), u8>,
            &[super::state::Marker],
        ) -> R,
    ) -> R {
        let map = super::state::lock(&self.minimap);
        read(&map.cells, &map.markers)
    }
}

impl PlayView {
    /// Local camera only; navigation never queues a movement command.
    pub fn navigate_minimap(
        &mut self,
        action: super::view::MinimapNavigation,
    ) -> Result<(), oteryn_renderer::BatchError> {
        use super::view::MinimapNavigation as Nav;
        use oteryn_client_ui::minimap;
        {
            let mut map = super::state::lock(&self.minimap);
            match action {
                Nav::Pan(x, y) => map.pan(x, y),
                Nav::Centre => {
                    map.tracking = true;
                    map.follow(self.own.x, self.own.y, self.floor);
                }
                Nav::ZoomIn => map.zoom = minimap::zoom_in(map.zoom),
                Nav::ZoomOut => map.zoom = minimap::zoom_out(map.zoom),
                Nav::FloorUp => {
                    map.tracking = false;
                    map.floor = minimap::floor_up(map.floor);
                }
                Nav::FloorDown => {
                    map.tracking = false;
                    map.floor = minimap::floor_down(map.floor);
                }
            }
        }
        self.rebuild()
    }
    /// Camera coordinates, floor, tiles per cell, and player-follow mode.
    pub fn minimap_camera(&self) -> (i32, i32, i16, i32, bool) {
        let map = super::state::lock(&self.minimap);
        (map.x, map.y, map.floor, map.zoom, map.tracking)
    }
}
