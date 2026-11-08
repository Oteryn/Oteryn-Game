use crate::play::PlayView;
impl PlayView {
    pub fn world_clock_text(&self) -> Option<String> {
        self.world_clock
            .map(oteryn_client_ui::world_light::clock_text)
    }
    pub(in crate::play) fn light_scene(&mut self) -> Result<(), oteryn_renderer::BatchError> {
        if !self.preferences().show_lights {
            self.scene.set_ambient([255; 3])?;
            self.scene.set_lights(Vec::new());
            return Ok(());
        }
        let ambient = self.world_clock.map_or([255; 3], oteryn_client_ui::world_light::ambient_rgb);
        let minimum = (u16::from(self.preferences().ambient_brightness) * 255 / 100) as u8;
        self.scene.set_ambient(ambient.map(|channel| channel.max(minimum)))?;
        let view = self.scene.view();
        let tile_px = view.tile_px() as f32;
        let sources = self
            .appearances
            .iter()
            .filter(|entry| entry.entity.position.floor == self.floor)
            .filter_map(|entry| {
                let (level, color) = entry.light?;
                let at = entry.entity.position;
                let [x, y] = view.tile_to_screen(oteryn_renderer::TileCoord::new(at.x, at.y));
                Some(oteryn_renderer::lighting::PointLight {
                    position: [x + tile_px / 2.0, y + tile_px / 2.0],
                    radius: f32::from(level) * tile_px,
                    color: oteryn_renderer::lighting::light_color(color),
                })
            })
            .collect();
        self.scene.set_lights(sources);
        Ok(())
    }
}
