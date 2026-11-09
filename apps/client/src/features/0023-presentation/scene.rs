//! Effect preparation precedes all UV generation, including the UI palette.
use crate::play::feature_0023_presentation::state::VisualCue;
use crate::{map::Draw, scene::Scene};
use oteryn_renderer::BatchError;
use oteryn_session::PresentationEventKind;
use std::{sync::Arc, time::Instant};

type VisualFrames = Vec<Option<Vec<Draw>>>;

impl Scene {
    pub(crate) fn visual_frames(
        &mut self,
        cues: &[VisualCue],
        now: Instant,
        floor: i16,
    ) -> Result<(VisualFrames, bool), BatchError> {
        let visible = |kind| match kind {
            PresentationEventKind::MagicEffect { position, .. } => position.floor == floor,
            PresentationEventKind::Missile { from, to, .. } => {
                from.floor == floor && to.floor == floor
            }
            _ => false,
        };
        if !cues.iter().any(|cue| visible(cue.kind)) {
            return Ok((vec![None; cues.len()], false));
        }
        self.cache.begin(
            self.world.atlas(),
            std::env::var_os(crate::map::ASSET_DIR_ENV).map(std::path::PathBuf::from),
        );
        let frames = cues
            .iter()
            .map(|cue| {
                if !visible(cue.kind) {
                    return None;
                }
                let age = u64::try_from(now.saturating_duration_since(cue.received).as_millis())
                    .unwrap_or(u64::MAX);
                match cue.kind {
                    PresentationEventKind::MagicEffect { effect_id, .. } => {
                        self.cache.visual(effect_id, false, age, [0, 0])
                    }
                    PresentationEventKind::Missile {
                        from,
                        to,
                        missile_id,
                        ..
                    } => self.cache.visual(
                        missile_id,
                        true,
                        age,
                        [to.x.saturating_sub(from.x), to.y.saturating_sub(from.y)],
                    ),
                    _ => None,
                }
            })
            .collect();
        let image = self.cache.image()?;
        let changed = self
            .presentation_atlas
            .as_ref()
            .is_none_or(|old| !Arc::ptr_eq(old, &image));
        if changed {
            self.presentation_atlas = Some(image);
            self.lit_atlas = None;
            self.tiles = oteryn_renderer::TileBatch::from_cells(
                &self.view,
                self.atlas(),
                &vec![
                    crate::map::BLACK_CELL;
                    (crate::scene::SCENE_COLUMNS * crate::scene::SCENE_ROWS) as usize
                ],
            )?;
        }
        Ok((frames, changed))
    }
}

#[cfg(test)]
impl Scene {
    pub(crate) fn prime_visual_assets(&mut self, root: std::path::PathBuf) {
        self.cache.begin(self.world.atlas(), Some(root));
    }
}
