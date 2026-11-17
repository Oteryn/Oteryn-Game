// Bounded, lazy, hash-verified effect frames in the scene's existing atlas.
impl PresentationCache {
    pub(super) fn visual(
        &mut self,
        id: u32,
        missile: bool,
        elapsed: u64,
        direction: [i32; 2],
    ) -> Option<Vec<Draw>> {
        let assets = self.assets.clone()?;
        let mut assets = assets.lock().unwrap_or_else(PoisonError::into_inner);
        let Assets { index, sheets } = &mut *assets;
        let appearance = index.visual(id, missile)?;
        let phase = if missile {
            0
        } else {
            let Some(phase) = appearance.phase_at(elapsed) else {
                return Some(Vec::new());
            };
            phase
        };
        let direction = if missile {
            direction.map(i32::signum)
        } else {
            [0, 0]
        };
        let key = (id, missile, phase, direction);
        if let Some(draws) = self.visuals.get(&key) {
            return Some(draws.clone());
        }
        if self.visuals.len() >= 4096 {
            return None;
        }
        let entry = index
            .resolve_visual(sheets, id, missile, phase, direction)
            .ok()?;
        let cells: Vec<_> = entry
            .cells
            .iter()
            .map(|cell| {
                sheets
                    .cell_rgba(cell.sprite_id, cell.cell_x, cell.cell_y)
                    .ok()
                    .map(|rgba| ([cell.offset_x, cell.offset_y], rgba))
            })
            .collect::<Option<_>>()?;
        if self.count() + cells.len() > oteryn_renderer::world::MAX_CELLS {
            return None;
        }
        let draws: Vec<_> = cells
            .into_iter()
            .map(|(offset, rgba)| self.append(&rgba).map(|cell| Draw { cell, offset }))
            .collect::<Option<_>>()?;
        self.visuals.insert(key, draws.clone());
        Some(draws)
    }
}
