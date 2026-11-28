use super::Scene;
use crate::input::{TargetKind, Targetable, pick_target};
use oteryn_renderer::{BatchError, TileCoord};

impl Scene {
    /// Current target, if any.
    #[must_use]
    pub const fn target(&self) -> Option<Targetable> {
        self.target
    }

    pub(crate) fn item_target_at(&self, tile: TileCoord) -> Option<crate::play::GroundItem> {
        self.target
            .filter(|selected| selected.tile == tile)
            .and_then(|selected| selected.item)
            .or_else(|| {
                self.visible
                    .iter()
                    .rev()
                    .filter(|target| target.tile == tile)
                    .find_map(|target| target.item)
            })
    }

    pub(crate) fn target_at(&self, tile: TileCoord) -> Option<Targetable> {
        // The own-actor highlight has no command identity; it must not hide an item below it.
        let on_tile = || {
            self.visible.iter().rev().filter(|target| {
                target.tile == tile && (target.entity.is_some() || target.item.is_some())
            })
        };
        on_tile()
            .find(|target| target.kind == TargetKind::Entity)
            .or_else(|| on_tile().find(|target| target.kind == TargetKind::Object))
            .copied()
    }

    /// Selects what is drawn on `tile` (entity before object). Returns the new target; a tile
    /// with nothing on it leaves the target unchanged and returns `None`.
    pub fn select_tile(&mut self, tile: TileCoord) -> Result<Option<Targetable>, BatchError> {
        let Some(picked) = pick_target(tile, &self.visible) else {
            return Ok(None);
        };
        if picked.kind == TargetKind::Object && picked.item.is_some() {
            self.cycle_stack(tile)?;
            return Ok(self.target);
        }
        self.target = Some(picked);
        self.rebuild_sprites()?;
        Ok(Some(picked))
    }

    /// Select the next observed item even when an actor occupies the same cell.
    pub(crate) fn cycle_stack(&mut self, tile: TileCoord) -> Result<(), BatchError> {
        let mut stack = self
            .visible
            .iter()
            .rev()
            .filter(|entry| entry.tile == tile && entry.item.is_some());
        let Some(top) = stack.clone().next().copied() else {
            return Ok(());
        };
        let next = match self.target {
            Some(current) => stack
                .find(|entry| **entry == current)
                .and_then(|_| stack.next().copied()),
            None => None,
        };
        self.target = Some(next.unwrap_or(top));
        self.rebuild_sprites()
    }

    /// Keeps a selection only while that exact visible identity still exists.
    pub(crate) fn retain_target(&mut self, target: Option<Targetable>) -> Result<(), BatchError> {
        self.target = target.filter(|target| self.visible.contains(target));
        self.rebuild_sprites()
    }
}
