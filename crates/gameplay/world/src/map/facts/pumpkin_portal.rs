//! One map-owned seasonal portal presentation; no ItemInstance or teleport authority.
use super::*;
const ENTRY: TilePos = TilePos {
    x: 33317,
    y: 32415,
    floor: -7,
};
impl BundleFacts {
    /// Runs while boot still owns the unique mutable facts, before any snapshot.
    pub(super) fn initialize_pumpkin_portal(
        &mut self,
        pos: TilePos,
        tile: &oteryn_world_bundle::sector::Tile,
        ordinal: usize,
        key: u64,
        palette: u32,
    ) {
        let top = tile
            .items
            .iter()
            .filter(|item| item.depth == 0)
            .collect::<Vec<_>>();
        if pos != ENTRY
            || ordinal != 1
            || top.len() != 2
            || tile.flags != 0
            || tile.house != 0
            || !tile.zones.is_empty()
            || top[1].attrs != Attrs::default()
            || self
                .palette
                .get(top[0].palette as usize)
                .is_none_or(|p| p.facts.appearance_id != 231)
            || self
                .palette
                .get(palette as usize)
                .is_none_or(|p| p.facts.appearance_id != 1949)
        {
            return;
        }
        if let Ok(states) = self.callbacks.get_mut() {
            let states = Arc::make_mut(states);
            states.pumpkin_hidden.insert(key);
            states.hidden.insert(key);
        }
    }
    pub fn project_pumpkin_portal(&self, base: &super::super::WorldBase, open: bool) -> bool {
        let Some(tile) = base.tile(ENTRY.x, ENTRY.y, ENTRY.floor) else {
            return false;
        };
        let top = tile
            .ids()
            .iter()
            .zip(tile.depths())
            .filter(|(_, depth)| **depth == 0)
            .collect::<Vec<_>>();
        if top.len() != 2 {
            return false;
        }
        let Some(ground) = self.base_entry(ENTRY, 0, *top[0].0) else {
            return false;
        };
        let Some(portal) = self.base_entry(ENTRY, 1, *top[1].0) else {
            return false;
        };
        if ground.appearance_id != 231 || portal.appearance_id != 1949 || portal.bound {
            return false;
        }
        let Some(key) = placement_key(ENTRY, 1) else {
            return false;
        };
        let Ok(mut states) = self.callbacks.lock() else {
            return false;
        };
        if !open && states.pumpkin_hidden.contains(&key) {
            return true;
        }
        if open && !states.pumpkin_hidden.contains(&key) {
            return true;
        }
        if !states.has_capacity(1) {
            return false;
        }
        let states = Arc::make_mut(&mut states);
        if open {
            states.pumpkin_hidden.remove(&key);
            if !states.cake_hidden.contains(&key) && !states.gathering_hidden.contains(&key) {
                states.hidden.remove(&key);
            }
        } else {
            states.pumpkin_hidden.insert(key);
            states.hidden.insert(key);
        }
        true
    }
}
