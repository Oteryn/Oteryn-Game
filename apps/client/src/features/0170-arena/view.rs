use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::arena::ArenaPanel;
impl PlayLink {
    pub fn arena_panel(&self) -> ArenaPanel {
        lock(&self.mailbox).arena.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;

impl crate::play::PlayView {
    pub(in crate::play) fn project_arena_hazards(
        &mut self,
        link: &PlayLink,
        now: std::time::Instant,
    ) -> Result<(), oteryn_renderer::BatchError> {
        let panel = link.arena_panel();
        let mut warnings = Vec::new();
        if panel.selected
            && panel.drome.is_some_and(|reply| reply.phase == 2)
            && let Some(received) = panel.hazards_received_at
        {
            let elapsed = now.saturating_duration_since(received).as_micros();
            for hazard in &panel.hazards.entries {
                if elapsed
                    >= u128::from(
                        hazard
                            .expires_at_micros
                            .saturating_sub(panel.hazards.observed_at_micros),
                    )
                {
                    continue;
                }
                use oteryn_client_runtime::arena::DromeHazardKind::*;
                let (color, seed) = match hazard.kind {
                    Lava => ([255, 51, 0], false),
                    Beam => ([0, 153, 255], false),
                    Tanked => ([153, 51, 204], false),
                    Sorrow => ([51, 255, 51], true),
                    Roots => ([153, 102, 51], true),
                };
                warnings.extend(
                    hazard
                        .positions
                        .iter()
                        .filter(|position| position.floor == self.floor)
                        .map(|position| crate::scene::GroundWarning {
                            tile: oteryn_renderer::TileCoord::new(position.x, position.y),
                            color,
                            seed,
                        }),
                );
            }
        }
        self.scene.set_drome_hazards(warnings)
    }
}
