use crate::play::PlayView;
impl PlayView {
    pub fn world_clock_text(&self) -> Option<String> {
        self.world_clock.map(oteryn_client_ui::world_light::clock_text)
    }
    pub(in crate::play) fn light_scene(&mut self) -> Result<(), oteryn_renderer::BatchError> {
        if let Some(clock) = self.world_clock {
            self.scene.set_ambient(oteryn_client_ui::world_light::ambient_rgb(clock))?;
        }
        Ok(())
    }
}
