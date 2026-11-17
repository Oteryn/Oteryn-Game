// Ordinary asset lights follow the same positions and lifetimes as their rendered cues.
impl Scene {
    pub(crate) fn presentation_lights<'a>(
        &self,
        cues: impl Iterator<Item = &'a VisualCue>,
        now: Instant,
        floor: i16,
    ) -> Vec<oteryn_renderer::lighting::PointLight> {
        let view = self.view();
        let tile_px = view.tile_px() as f32;
        cues.filter_map(|cue| {
            let age = now.saturating_duration_since(cue.received);
            let (light, at) = match cue.kind {
                PresentationEventKind::MagicEffect { position, effect_id, .. } if position.floor == floor => {
                    let light = self.cache.visual_light(effect_id, false, u64::try_from(age.as_millis()).unwrap_or(u64::MAX))?;
                    (light, view.tile_to_screen(oteryn_renderer::TileCoord::new(position.x, position.y)))
                }
                PresentationEventKind::Missile { from, to, missile_id, .. } if from.floor == floor && to.floor == floor => {
                    let a = view.tile_to_screen(oteryn_renderer::TileCoord::new(from.x, from.y));
                    let b = view.tile_to_screen(oteryn_renderer::TileCoord::new(to.x, to.y));
                    let duration = ((a[0]-b[0]).hypot(a[1]-b[1]) / tile_px).sqrt() * 0.15;
                    if age.as_secs_f32() >= duration { return None; }
                    let t = age.as_secs_f32() / duration;
                    (self.cache.visual_light(missile_id, true, 0)?, [a[0]+(b[0]-a[0])*t,a[1]+(b[1]-a[1])*t])
                }
                _ => return None,
            };
            let (level,color) = light;
            Some(oteryn_renderer::lighting::PointLight { position: [at[0]+tile_px/2.,at[1]+tile_px/2.], radius: f32::from(level)*tile_px, color: oteryn_renderer::lighting::light_color(color) })
        }).collect()
    }
}
