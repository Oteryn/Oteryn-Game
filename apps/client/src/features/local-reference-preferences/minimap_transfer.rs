//! The settings thread queues requests; the shell applies them to the active world's minimap.
use super::SettingsState;
use crate::play::{PlayView, feature_local_minimap::state::lock};
use std::{io, path::PathBuf};
impl SettingsState {
    pub fn request_minimap_transfer(&mut self, import: bool, path: PathBuf) {
        self.minimap_transfer = Some((import, path));
    }
    pub fn apply_minimap_transfer(&mut self, view: &mut PlayView) -> io::Result<bool> {
        let Some((import, path)) = self.minimap_transfer.take() else {
            return Ok(false);
        };
        let mut map = lock(&view.minimap);
        if import {
            map.import_from(&path)?;
        } else {
            map.export_to(&path)?;
        }
        drop(map);
        if import {
            view.rebuild().map_err(io::Error::other)?;
        }
        Ok(true)
    }
}
