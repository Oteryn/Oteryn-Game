/// Quantize a missile path into the eight 45-degree artwork sectors.
#[must_use]
pub fn missile_direction([x, y]: [i32; 2]) -> [i32; 2] {
    if x == 0 && y == 0 { return [0, 0]; }
    let sector = ((f64::from(y).atan2(f64::from(x)) / std::f64::consts::FRAC_PI_4 + 0.5).floor() as i32).rem_euclid(8);
    [[1, 0], [1, 1], [0, 1], [-1, 1], [-1, 0], [-1, -1], [0, -1], [1, -1]][sector as usize]
}

impl Appearance {
    /// Effects play once. Missing timing uses the reference client's 100 ms default.
    #[must_use]
    pub fn phase_at(&self, mut elapsed_ms: u64) -> Option<u32> {
        for phase in 0..self.phases {
            let duration = u64::from(self.phase_ms.get(phase as usize).copied().unwrap_or(100));
            if elapsed_ms < duration {
                return Some(phase);
            }
            elapsed_ms = elapsed_ms.saturating_sub(duration);
        }
        None
    }
}
impl AppearanceIndex {
    #[must_use]
    pub fn visual(&self, id: u32, missile: bool) -> Option<&Appearance> {
        (if missile {
            &self.missiles
        } else {
            &self.effects
        })
        .get(&id)
    }
    /// Exact catalog frame; missiles use the 3 × 3 direction pattern, effects use phase timing.
    pub fn resolve_visual(
        &self,
        sheets: &SpriteSheets,
        id: u32,
        missile: bool,
        phase: u32,
        direction: [i32; 2],
    ) -> Result<ResolvedEntry, AssetError> {
        let appearance = self
            .visual(id, missile)
            .ok_or(AssetError::UnknownAppearance { id })?;
        if phase >= appearance.phases {
            return Err(AssetError::UnknownSprite { sprite_id: 0 });
        }
        let pattern = if missile {
            let direction = missile_direction(direction);
            (
                (direction[0].signum() + 1) as u32 % appearance.pattern_width,
                (direction[1].signum() + 1) as u32 % appearance.pattern_height,
                0,
            )
        } else {
            (0, 0, 0)
        };
        entry(appearance, sheets, pattern, 0..appearance.layers, phase)
    }
}
