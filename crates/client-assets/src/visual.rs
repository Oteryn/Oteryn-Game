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
