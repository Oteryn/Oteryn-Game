//! HUD anchoring follows the scene's current zoom and camera.
impl super::Scene {
    pub(crate) fn hud_center(&self) -> [i32; 2] {
        let [x, y] = self.view.tile_to_screen(self.own);
        let half = self.view.tile_px() as f32 / 2.0;
        [(x + half).round() as i32, (y + half).round() as i32]
    }
}
