//! Hover timing over confirmed visible entities; no hover request is sent to the server.
use super::*;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TooltipTarget { pub entity: oteryn_session::EntityRef, pub label: String }

#[derive(Debug, Default)]
pub struct HoverTooltip { target: Option<TooltipTarget>, since: Option<Instant> }
impl HoverTooltip {
    pub fn update(&mut self, target: Option<TooltipTarget>, now: Instant, delay: Duration) -> Option<&str> {
        if self.target != target { self.target = target; self.since = Some(now); }
        let target = self.target.as_ref()?;
        let since = self.since.get_or_insert(now);
        (now.saturating_duration_since(*since) >= delay).then_some(target.label.as_str())
    }
}

impl PlayView {
    pub fn tooltip_target(&self, point: [f32; 2]) -> Option<TooltipTarget> {
        if !self.preferences().tooltips_enabled { return None; }
        let tile = self.scene().view().screen_to_tile(point[0], point[1])?;
        let entity = self.entities.iter().rev().find(|entity| entity.position.floor == self.floor && entity.position.x == tile.x && entity.position.y == tile.y)?;
        let label = entity.display_name.as_ref()?.as_str().ok()?.to_owned();
        Some(TooltipTarget { entity: entity.entity, label })
    }
}
