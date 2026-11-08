//! Locally explored cells and user markers; geometry comes only from validated server tiles.
use oteryn_session::world_map::WorldMapViewSnapshot;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{self, Read, Write},
    path::PathBuf,
    time::{Duration, Instant},
};
pub type SharedMinimap = std::sync::Arc<std::sync::Mutex<Minimap>>;
pub fn lock(shared: &SharedMinimap) -> std::sync::MutexGuard<'_, Minimap> {
    shared
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
type Binding = ([u8; 32], [u8; 32], u64);
const MAX_CELLS: usize = 1_048_576;
const MAX_MARKERS: usize = 512;
const MAX_BYTES: u64 = 64 << 20;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Marker {
    pub x: i32,
    pub y: i32,
    pub floor: i16,
    pub label: String,
    pub color: u8,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Saved {
    version: u8,
    cells: Vec<[i32; 4]>,
    markers: Vec<Marker>,
}
#[derive(Debug)]
pub struct Minimap {
    pub cells: BTreeMap<(i16, i32, i32), u8>,
    pub markers: Vec<Marker>,
    pub x: i32,
    pub y: i32,
    pub floor: i16,
    pub zoom: i32,
    pub tracking: bool,
    // Read by the native Windows marker editor.
    #[cfg_attr(not(windows), allow(dead_code))]
    pub editor_open: bool,
    pub own: (i32, i32, i16),
    pub feedback: String,
    root: Option<PathBuf>,
    world: Option<String>,
    binding: Option<Binding>,
    dirty: bool,
    saved_at: Instant,
}
impl Default for Minimap {
    fn default() -> Self {
        Self {
            cells: BTreeMap::new(),
            markers: Vec::new(),
            x: 0,
            y: 0,
            floor: -7,
            zoom: 1,
            tracking: true,
            editor_open: false,
            own: (0, 0, -7),
            feedback: String::new(),
            root: std::env::var_os("LOCALAPPDATA").map(|p| PathBuf::from(p).join("Oteryn/minimap")),
            world: None,
            binding: None,
            dirty: false,
            saved_at: Instant::now(),
        }
    }
}
include!("state/methods.rs");
impl Drop for Minimap {
    fn drop(&mut self) {
        let _ = self.save();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exploration_and_markers_survive_reopen_and_bindings_stay_separate()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = std::env::temp_dir().join(format!("minimap-{}", std::process::id()));
        let mut map = Minimap::default();
        map.root = Some(root.clone());
        map.bind_world("11111111-1111-4111-8111-111111111111");
        use oteryn_session::{
            ActorPosition,
            world_map::{MapTile, MapViewHeader, WorldMapViewSnapshot},
        };
        let position = ActorPosition {
            x: 100,
            y: 200,
            floor: -7,
        };
        let mut snapshot = WorldMapViewSnapshot {
            header: MapViewHeader {
                content_generation: [1; 32],
                bundle_digest: [2; 32],
                reset_epoch: 1,
                origin: position,
                first_visible_floor: -7,
            },
            tiles: vec![MapTile {
                position,
                ground_speed: 100,
                more: false,
                items: vec![],
            }],
        };
        map.observe(&snapshot);
        map.edit_marker(100, 200, -7, "Home".into(), 1)?;
        // Legacy item-only cells must not return as terrain when an older cache is reopened.
        map.cells.insert((-7, 999, 999), 86);
        map.dirty = true;
        map.save()?;
        let binding = map.binding;
        let mut restored = Minimap::default();
        restored.root = Some(root.clone());
        restored.bind_world("11111111-1111-4111-8111-111111111111");
        restored.observe(&snapshot);
        assert_eq!(restored.markers[0].label, "Home");
        assert!(restored.cells.contains_key(&(-7, 100, 200)));
        assert!(!restored.cells.contains_key(&(-7, 999, 999)));
        snapshot.header.reset_epoch += 1;
        restored.observe(&snapshot);
        assert!(restored.markers.is_empty());
        assert_ne!(restored.binding, binding);
        drop(restored);
        drop(map);
        std::fs::remove_dir_all(root)?;
        Ok(())
    }
    #[test]
    fn camera_and_marker_validation_do_not_change_gameplay_positions()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut map = Minimap::default();
        map.follow(100, 200, -7);
        assert_eq!(map.position(15, 12), Some((100, 200, -7)));
        map.pan(5, -5);
        map.zoom = 4;
        assert_eq!(map.position(16, 12), Some((109, 195, -7)));
        map.floor = -6;
        assert_eq!(map.position(15, 12), Some((105, 195, -6)));
        assert!(map.edit_marker(0, 0, 2, "x".into(), 0).is_err());
        assert!(map.edit_marker(0, 0, -7, "\0".into(), 0).is_err());
        map.edit_marker(100, 200, -7, "Gate".into(), 2)?;
        map.edit_marker(100, 200, -7, "Exit".into(), 3)?;
        assert_eq!(map.markers.len(), 1);
        assert_eq!(map.markers[0].label, "Exit");
        map.remove_marker(100, 200, -7)?;
        assert!(map.markers.is_empty());
        Ok(())
    }
    #[test]
    fn authoritative_window_clears_ground_but_preserves_distant_exploration() {
        use oteryn_session::{
            ActorPosition,
            world_map::{MapTile, MapViewHeader, WorldMapViewSnapshot},
        };
        let snapshot = |x, ground_speed| {
            let position = ActorPosition {
                x,
                y: 200,
                floor: -7,
            };
            WorldMapViewSnapshot {
                header: MapViewHeader {
                    content_generation: [1; 32],
                    bundle_digest: [2; 32],
                    reset_epoch: 1,
                    origin: position,
                    first_visible_floor: -7,
                },
                tiles: vec![MapTile {
                    position,
                    ground_speed,
                    more: false,
                    items: vec![],
                }],
            }
        };
        let mut map = Minimap::default();
        map.observe(&snapshot(100, 100));
        map.observe(&snapshot(1000, 100));
        assert!(map.cells.contains_key(&(-7, 100, 200)));
        assert!(map.cells.contains_key(&(-7, 1000, 200)));
        let mut cleared = snapshot(1000, 100);
        cleared.tiles.clear();
        map.observe(&cleared);
        assert!(map.cells.contains_key(&(-7, 100, 200)));
        assert!(!map.cells.contains_key(&(-7, 1000, 200)));
        map.observe(&snapshot(100, 0));
        assert!(!map.cells.contains_key(&(-7, 100, 200)));
    }
}
