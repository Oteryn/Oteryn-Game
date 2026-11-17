//! Bounded local map memory. Imported observations never become world geometry or handles.
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::{self, Read, Write},
    path::Path,
};
pub const MAX_REMEMBERED_TILES: usize = 8192;
pub const MAX_MAP_MARKS: usize = 128;
const MAX_TRANSFER_BYTES: u64 = 1024 * 1024;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MinimapBinding {
    pub world: Option<[u8; 16]>,
    pub channel: Option<[u8; 16]>,
    pub generation: [u8; 32],
    pub bundle: [u8; 32],
    pub epoch: u64,
}
/// Compact `[x,y,floor,ground_speed]` records; no assets, object handles or server identities.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MinimapMemory {
    pub binding: Option<MinimapBinding>,
    pub tiles: Vec<(i32, i32, i16, u16)>,
    pub marks: Vec<MinimapMark>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MinimapMark {
    pub x: i32,
    pub y: i32,
    pub floor: i16,
    pub label: String,
}
impl MinimapMemory {
    pub fn valid(&self) -> bool {
        self.tiles.len() <= MAX_REMEMBERED_TILES
            && self.marks.len() <= MAX_MAP_MARKS
            && (self.binding.is_some() || (self.tiles.is_empty() && self.marks.is_empty()))
            && self
                .binding
                .as_ref()
                .is_none_or(|b| b.world.is_some() == b.channel.is_some())
            && self
                .tiles
                .iter()
                .all(|(_, _, floor, speed)| (-128..=127).contains(floor) && *speed <= 1000)
            && self
                .tiles
                .iter()
                .map(|&(x, y, z, _)| ((x, y, z), ()))
                .collect::<BTreeMap<_, _>>()
                .len()
                == self.tiles.len()
            && self.marks.iter().all(|m| {
                (-128..=127).contains(&m.floor)
                    && !m.label.is_empty()
                    && m.label.len() <= 64
                    && !m.label.chars().any(char::is_control)
            })
    }
    pub fn add_mark(&mut self, mark: MinimapMark) -> io::Result<()> {
        if self.binding.is_none() || self.marks.len() == MAX_MAP_MARKS {
            return Err(io::Error::other("No map scope or marker limit reached"));
        }
        let mut next = self.clone();
        next.marks.push(mark);
        if !next.valid() {
            return Err(io::Error::other(
                "Invalid marker: use a label of 1–64 bytes and a valid floor",
            ));
        }
        *self = next;
        Ok(())
    }
    pub fn remove_mark(&mut self, index: usize) -> bool {
        if index >= self.marks.len() {
            return false;
        }
        self.marks.remove(index);
        true
    }
    fn observe(
        &mut self,
        map: &oteryn_session::world_map::WorldMapViewSnapshot,
        binding: MinimapBinding,
    ) {
        if self.binding.as_ref() != Some(&binding) {
            *self = Self {
                binding: Some(binding),
                ..Self::default()
            };
        }
        let updates: BTreeMap<_, _> = map
            .tiles
            .iter()
            .map(|t| {
                (
                    (t.position.x, t.position.y, t.position.floor),
                    t.ground_speed,
                )
            })
            .collect();
        self.tiles
            .retain(|&(x, y, z, _)| !updates.contains_key(&(x, y, z)));
        self.tiles.extend(
            updates
                .into_iter()
                .map(|((x, y, z), speed)| (x, y, z, speed)),
        );
        let excess = self.tiles.len().saturating_sub(MAX_REMEMBERED_TILES);
        self.tiles.drain(..excess);
    }
    pub fn export_new(&self, path: &Path) -> io::Result<()> {
        if !self.valid() {
            return Err(io::Error::other("Invalid minimap"));
        }
        let bytes = serde_json::to_vec(self).map_err(io::Error::other)?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        if let Err(error) = file.write_all(&bytes).and_then(|()| file.sync_all()) {
            drop(file);
            let _ = std::fs::remove_file(path);
            return Err(error);
        }
        Ok(())
    }
    pub fn import(path: &Path) -> io::Result<Self> {
        let mut bytes = Vec::new();
        std::fs::File::open(path)?
            .take(MAX_TRANSFER_BYTES + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_TRANSFER_BYTES {
            return Err(io::Error::other("Minimap file exceeds one MiB"));
        }
        let value: Self = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
        if !value.valid() {
            return Err(io::Error::other("Invalid minimap data"));
        }
        Ok(value)
    }
}
impl crate::play::PlayView {
    pub(in crate::play) fn remember_minimap(
        &mut self,
        map: &oteryn_session::world_map::WorldMapViewSnapshot,
    ) {
        let binding = MinimapBinding {
            world: self.served_channel.map(|c| c.world_id),
            channel: self.served_channel.map(|c| c.channel_id),
            generation: map.header.content_generation,
            bundle: map.header.bundle_digest,
            epoch: map.header.reset_epoch,
        };
        self.local.minimap_binding = Some(binding.clone());
        self.local.minimap_origin = Some(map.header.origin);
        let old = self.local.preferences.minimap.clone();
        self.local.preferences.minimap.observe(map, binding);
        if self.local.preferences.minimap != old {
            self.local.minimap_edit = crate::play::feature_0017_attack::view::next_edit();
        }
    }
    pub(in crate::play) fn project_minimap_memory(&mut self) {
        self.scene.minimap_memory =
            if self.local.preferences.minimap.binding == self.local.minimap_binding {
                self.local.preferences.minimap.clone()
            } else {
                MinimapMemory::default()
            };
        self.scene.minimap_floor = self.floor;
        self.scene.minimap_origin = self.local.minimap_origin;
    }
}
