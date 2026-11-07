//! Two generation-qualified seasonal Item looks, using the existing official asset owner.
use super::*;

impl PresentationCache {
    pub(super) fn item(&mut self, reference: u32) -> Vec<Draw> {
        let Some(appearance) =
            oteryn_client_assets::seasonal_item_projection::seasonal_item_projection()
                .and_then(|projection| projection.appearance_id(reference))
        else {
            return marker();
        };
        if let Some(draws) = self.items.get(&reference) {
            return draws.clone();
        }
        let resolved = self.assets.as_ref().and_then(|assets| {
            let mut assets = assets.lock().unwrap_or_else(PoisonError::into_inner);
            let Assets { index, sheets } = &mut *assets;
            // An Item outfit is one standing item, independent of actor-facing or
            // landscape patterns. The reference never becomes an inventory handle.
            let entry = index
                .resolve(
                    sheets,
                    appearance,
                    oteryn_client_assets::Placement::at(0, 0, 0),
                )
                .ok()?;
            entry
                .cells
                .iter()
                .map(|cell| {
                    sheets
                        .cell_rgba(cell.sprite_id, cell.cell_x, cell.cell_y)
                        .ok()
                        .map(|rgba| ([cell.offset_x, cell.offset_y], rgba))
                })
                .collect::<Option<Vec<_>>>()
        });
        let draws = resolved
            .and_then(|cells| {
                if self.count() + cells.len() > oteryn_renderer::world::MAX_CELLS {
                    return None;
                }
                cells
                    .into_iter()
                    .map(|(offset, rgba)| self.append(&rgba).map(|cell| Draw { cell, offset }))
                    .collect()
            })
            .unwrap_or_else(marker);
        self.items.insert(reference, draws.clone());
        draws
    }
}
