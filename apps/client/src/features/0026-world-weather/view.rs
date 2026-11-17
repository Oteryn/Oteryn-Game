use crate::play::PlayView;
impl PlayView {
    pub(in crate::play) fn weather_scene(&mut self) {
        // Each call uses a rebuilt scene, so None leaves no stale weather tint.
        if let Some((color, strength)) =
            oteryn_client_ui::world_weather::overlay(self.world_weather, self.floor)
        {
            self.scene.set_weather(color, strength);
        }
    }
}
