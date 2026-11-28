// UI cells share the scene's verified assets and packed atlas.
impl PresentationCache {
    pub(super) fn container_flag(&self, appearance: u16) -> Option<bool> {
        let assets = self.assets.as_ref()?.lock().unwrap_or_else(PoisonError::into_inner);
        assets.index.get(u32::from(appearance)).map(|item| item.container)
    }

    pub(super) fn stackable_flag(&self, appearance: u16) -> Option<bool> {
        let assets = self
            .assets
            .as_ref()?
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        assets
            .index
            .get(u32::from(appearance))
            .map(|item| item.stackable)
    }

    pub(super) fn prepare_item(&mut self, item: &oteryn_session::ItemEntry) {
        let Some(key) = oteryn_client_ui::item_appearance::atlas_key(item) else {
            return;
        };
        if self.items.contains_key(&key) {
            return;
        }
        let resolved = self.assets.as_ref().and_then(|assets| {
            let mut assets = assets.lock().unwrap_or_else(PoisonError::into_inner);
            let Assets { index, sheets } = &mut *assets;
            let mut placement = oteryn_client_assets::Placement::at(0, 0, 0);
            placement.count = item.count.get();
            placement.sub_type = item.sub_type;
            let entry = index
                .resolve(sheets, u32::from(item.appearance_id), placement)
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
        let Some(cells) = resolved else {
            return;
        };
        if cells.is_empty() || self.count() + cells.len() > oteryn_renderer::world::MAX_CELLS {
            return;
        }
        let draws = cells
            .into_iter()
            .filter_map(|(offset, rgba)| self.append(&rgba).map(|cell| Draw { cell, offset }))
            .collect();
        self.items.insert(key, draws);
    }

    pub(super) fn item(&self, item: &oteryn_session::ItemEntry) -> Option<&[Draw]> {
        self.items
            .get(&oteryn_client_ui::item_appearance::atlas_key(item)?)
            .filter(|draws| !draws.is_empty())
            .map(Vec::as_slice)
    }
}
