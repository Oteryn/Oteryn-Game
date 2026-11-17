//! Locally explored cells and user markers; geometry comes only from validated server tiles.
use oteryn_session::world_map::WorldMapViewSnapshot;
use serde::{Deserialize, Serialize};
#[path = "transfer.rs"]
mod transfer;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{self, Read},
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
    /// Current Character facts; never serialized into the World-only local cache.
    pub quest_marks: oteryn_client_ui::personal_map_marks::PersonalMapMarks,
    pub quest_markers: Vec<Marker>,
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
            quest_marks: Default::default(),
            quest_markers: Vec::new(),
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
impl Minimap {
    /// Server display facts are connection-local and never saved in the shared local map.
    pub fn replace_quest_markers(
        &mut self,
        marks: &[oteryn_session::quest_log::QuestMapMark],
    ) -> bool {
        let next = marks
            .iter()
            .map(|m| Marker {
                x: i32::from(m.x),
                y: i32::from(m.y),
                floor: i16::from(m.floor),
                label: m.text.clone(),
                color: 0,
            })
            .collect::<Vec<_>>();
        if self.quest_markers == next {
            return false;
        }
        self.quest_markers = next;
        true
    }

    #[cfg(windows)]
    pub fn displayed_markers(&self) -> Vec<Marker> {
        self.markers
            .iter()
            .chain(&self.quest_markers)
            .cloned()
            .collect()
    }

    pub fn bind_world(&mut self, world: &str) {
        // Native admission calls this even when another Character enters the same World.
        self.quest_marks = Default::default();
        self.quest_markers.clear();
        if !(1..=36).contains(&world.len())
            || !world.bytes().all(|c| c.is_ascii_hexdigit() || c == b'-')
        {
            if let Err(error) = self.save() {
                self.feedback =
                    format!("Could not save minimap before refusing world binding: {error}");
            } else {
                self.feedback =
                    tr!("Invalid minimap world binding; local saving is disabled").into();
            }
            self.world = None;
            self.binding = None;
            self.cells.clear();
            self.markers.clear();
            self.dirty = false;
            self.tracking = true;
            self.zoom = 1;
            self.quest_marks = Default::default();
        } else if self.world.as_deref() != Some(world) {
            if let Err(error) = self.save() {
                self.feedback = error.to_string();
            }
            self.quest_markers.clear();
            self.world = Some(world.into());
            self.tracking = true;
            self.zoom = 1;
            self.quest_marks = Default::default();
            self.binding = None;
            self.cells.clear();
            self.markers.clear();
        }
    }
    pub fn replace_quest_marks(
        &mut self,
        marks: oteryn_client_ui::personal_map_marks::PersonalMapMarks,
    ) -> bool {
        if marks.encode().is_none() {
            return false;
        }
        self.quest_marks = marks;
        true
    }
    fn path(&self) -> Option<PathBuf> {
        let (generation, bundle, epoch) = self.binding?;
        let hex = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        Some(self.root.as_ref()?.join(self.world.as_ref()?).join(format!(
            "{}-{}-{epoch}.json",
            hex(&generation),
            hex(&bundle)
        )))
    }
    pub fn observe(&mut self, map: &WorldMapViewSnapshot) {
        let binding = (
            map.header.content_generation,
            map.header.bundle_digest,
            map.header.reset_epoch,
        );
        if self.binding != Some(binding) {
            if let Err(error) = self.save() {
                self.feedback = error.to_string();
            }
            self.cells.clear();
            self.markers.clear();
            self.dirty = false;
            self.binding = Some(binding);
            if let Err(error) = self.load() {
                self.feedback = lfmt!("Could not load minimap: {error}", error = error);
            }
        }
        // The snapshot is authoritative only within its visible window. Retain distant
        // exploration, but remove cleared cells and item-only tiles from remembered ground.
        use oteryn_session::world_map::{
            VIEW_HEIGHT, VIEW_LEFT, VIEW_TOP, VIEW_WIDTH, floors_in_view, in_window,
        };
        let origin = map.header.origin;
        let observed: BTreeSet<_> = map
            .tiles
            .iter()
            .filter(|tile| {
                tile.ground_speed > 0
                    && tile.position.floor <= map.header.first_visible_floor
                    && in_window(origin, tile.position)
            })
            .map(|tile| (tile.position.floor, tile.position.x, tile.position.y))
            .collect();
        if let Some(floors) = floors_in_view(origin.floor) {
            for floor in floors.filter(|f| *f <= map.header.first_visible_floor) {
                let shift = i64::from(floor) - i64::from(origin.floor);
                for y in 0..VIEW_HEIGHT {
                    for x in 0..VIEW_WIDTH {
                        let (Ok(x), Ok(y)) = (
                            i32::try_from(
                                i64::from(origin.x) - i64::from(VIEW_LEFT) + shift + i64::from(x),
                            ),
                            i32::try_from(
                                i64::from(origin.y) - i64::from(VIEW_TOP) + shift + i64::from(y),
                            ),
                        ) else {
                            continue;
                        };
                        let key = (floor, x, y);
                        if !observed.contains(&key) && self.cells.remove(&key).is_some() {
                            self.dirty = true;
                        }
                    }
                }
            }
        }
        for key in observed {
            if self.cells.len() >= MAX_CELLS && !self.cells.contains_key(&key) {
                self.feedback =
                    tr!("Local exploration limit reached; existing exploration is retained").into();
                continue;
            }
            if self.cells.insert(key, 55) != Some(55) {
                self.dirty = true;
            }
        }
        if self.saved_at.elapsed() >= Duration::from_secs(5)
            && let Err(error) = self.save()
        {
            self.feedback = lfmt!("Could not save minimap: {error}", error = error);
        }
    }
    pub fn follow(&mut self, x: i32, y: i32, floor: i16) {
        self.own = (x, y, floor);
        if self.tracking {
            self.x = x;
            self.y = y;
            self.floor = floor;
        }
    }
    // Invoked by native Windows controls; the headless tests exercise the same state.
    #[cfg_attr(not(windows), allow(dead_code))]
    pub fn pan(&mut self, dx: i32, dy: i32) {
        self.tracking = false;
        self.x = self
            .x
            .saturating_add(dx.saturating_mul(self.zoom))
            .clamp(0, 65_535);
        self.y = self
            .y
            .saturating_add(dy.saturating_mul(self.zoom))
            .clamp(0, 65_535);
    }
    pub fn position(&self, x: i32, y: i32) -> Option<(i32, i32, i16)> {
        if !(0..31).contains(&x) || !(0..25).contains(&y) {
            return None;
        }
        let position = (
            self.x.checked_add((x - 15).checked_mul(self.zoom)?)?,
            self.y.checked_add((y - 12).checked_mul(self.zoom)?)?,
            self.floor,
        );
        ((0..=65_535).contains(&position.0) && (0..=65_535).contains(&position.1))
            .then_some(position)
    }
    // Invoked by native Windows controls; the headless tests exercise the same state.
    #[cfg_attr(not(windows), allow(dead_code))]
    pub fn edit_marker(
        &mut self,
        x: i32,
        y: i32,
        floor: i16,
        label: String,
        color: u8,
    ) -> io::Result<()> {
        if self
            .quest_markers
            .iter()
            .any(|m| (m.x, m.y, m.floor) == (x, y, floor))
        {
            return Err(io::Error::other("Quest markers are supplied by the server"));
        }

        if !(0..=65_535).contains(&x)
            || !(0..=65_535).contains(&y)
            || !(-15..=0).contains(&floor)
            || color > 3
            || label.trim().is_empty()
            || label.len() > 128
            || label.chars().any(char::is_control)
        {
            return Err(io::Error::other(tr!(
                "Use map coordinates 0-65535, floor -15 to 0 and a label of 1-128 bytes"
            )));
        }
        if let Some(marker) = self
            .markers
            .iter_mut()
            .find(|m| (m.x, m.y, m.floor) == (x, y, floor))
        {
            marker.label = label;
            marker.color = color;
        } else {
            if self.markers.len() >= MAX_MARKERS {
                return Err(io::Error::other(tr!(
                    "The minimap supports up to 512 markers"
                )));
            }
            self.markers.push(Marker {
                x,
                y,
                floor,
                label,
                color,
            });
        }
        self.dirty = true;
        self.save()
    }
    /// `Misc.: Create Mark`: adds a default-labelled marker at `own`. Returns `false` and leaves
    /// the map unchanged when a marker already sits there, so the caller opens it for editing.
    pub fn create_mark_at(&mut self, own: (i32, i32, i16)) -> io::Result<bool> {
        let (x, y, floor) = own;
        if self
            .markers
            .iter()
            .any(|m| (m.x, m.y, m.floor) == (x, y, floor))
        {
            return Ok(false);
        }
        self.edit_marker(x, y, floor, tr!("Mark").into(), 0)?;
        Ok(true)
    }
    // Invoked by native Windows controls; the headless tests exercise the same state.
    #[cfg_attr(not(windows), allow(dead_code))]
    pub fn remove_marker(&mut self, x: i32, y: i32, floor: i16) -> io::Result<()> {
        if self
            .quest_markers
            .iter()
            .any(|m| (m.x, m.y, m.floor) == (x, y, floor))
        {
            return Err(io::Error::other("Quest markers are supplied by the server"));
        }
        self.markers
            .retain(|m| (m.x, m.y, m.floor) != (x, y, floor));
        self.dirty = true;
        self.save()
    }
    pub fn save(&mut self) -> io::Result<()> {
        if !self.dirty {
            return Ok(());
        }
        let Some(path) = self.path() else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let saved = Saved {
            version: 1,
            cells: self
                .cells
                .iter()
                .map(|(&(f, x, y), &c)| [i32::from(f), x, y, i32::from(c)])
                .collect(),
            markers: self.markers.clone(),
        };
        saved.write_to(&path)?;
        self.dirty = false;
        self.saved_at = Instant::now();
        Ok(())
    }
    fn load(&mut self) -> io::Result<()> {
        let Some(path) = self.path() else {
            return Ok(());
        };
        let file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e),
        };
        if file.metadata()?.len() > MAX_BYTES {
            return Err(io::Error::other(tr!("Minimap file is too large")));
        }
        let mut bytes = Vec::new();
        file.take(MAX_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err(io::Error::other(tr!("Minimap file is too large")));
        }
        let saved: Saved = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
        saved.validate()?;
        // Older caches recorded gray item-only cells. They are not persistent terrain.
        self.dirty |= saved.cells.iter().any(|cell| cell[3] == 86);
        self.cells = saved
            .cells
            .into_iter()
            .filter(|cell| cell[3] == 55)
            .map(|[f, x, y, c]| ((f as i16, x, y), c as u8))
            .collect();
        self.markers = saved.markers;
        Ok(())
    }
}
impl Drop for Minimap {
    fn drop(&mut self) {
        let _ = self.save();
    }
}
#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;
#[cfg(test)]
#[path = "tests/transfer.rs"]
mod transfer_tests;

#[cfg(test)]
#[path = "tests/play_persistence.rs"]
mod play_persistence;

#[cfg(test)]
#[path = "tests/e2e_n07d.rs"]
mod e2e_n07d;

#[cfg(test)]
mod quest_mark_tests {
    use super::*;
    use oteryn_client_ui::personal_map_marks::{PersonalMapMark, PersonalMapMarks};
    fn treasure() -> PersonalMapMarks {
        PersonalMapMarks(vec![PersonalMapMark {
            key: "sterling".into(),
            x: 31948,
            y: 31924,
            floor: -7,
            text: "Treasure".into(),
        }])
    }
    #[test]
    fn personal_quest_marks_are_not_editable_or_saved_in_world_only_cache()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = std::env::temp_dir().join(format!(
            "questpc5-personal-map-privacy-{}",
            std::process::id()
        ));
        let mut map = Minimap::default();
        map.root = Some(directory.clone());
        map.bind_world("11111111-1111-4111-8111-111111111111");
        map.binding = Some(([1; 32], [2; 32], 1));
        assert!(map.replace_quest_marks(treasure()));
        assert!(
            map.replace_quest_markers(&[oteryn_session::quest_log::QuestMapMark {
                x: 31948,
                y: 31924,
                floor: -7,
                text: "Treasure".into(),
            }])
        );
        map.bind_world("11111111-1111-4111-8111-111111111111");
        assert!(
            map.quest_markers.is_empty(),
            "admission also clears legacy Character facts"
        );
        assert!(
            map.quest_marks.0.is_empty(),
            "a new admission to the same World clears Character facts"
        );
        assert!(map.replace_quest_marks(treasure()));
        map.edit_marker(31948, 31924, -7, "Local label".into(), 0)?;
        assert_eq!(map.quest_marks, treasure());
        map.remove_marker(31948, 31924, -7)?;
        assert_eq!(map.quest_marks, treasure());
        map.edit_marker(10, 20, -7, "Home".into(), 1)?;
        let path = map.path().ok_or("path")?;
        let encoded = std::fs::read_to_string(path)?;
        assert!(!encoded.contains("Treasure"));
        assert!(!encoded.contains("quest_marks"));
        assert!(encoded.contains("Home"));
        let mut other = Minimap::default();
        other.root = Some(directory.clone());
        other.bind_world("11111111-1111-4111-8111-111111111111");
        other.binding = Some(([1; 32], [2; 32], 1));
        other.load()?;
        assert_eq!(other.markers.len(), 1);
        assert!(
            other.quest_marks.0.is_empty(),
            "another Character never inherits server facts from the World cache"
        );
        map.bind_world("22222222-2222-4222-8222-222222222222");
        assert!(map.quest_marks.0.is_empty());
        std::fs::remove_dir_all(directory)?;
        Ok(())
    }
    #[test]
    fn malformed_personal_quest_marks_cannot_replace_confirmed_facts() {
        let mut map = Minimap::default();
        assert!(map.replace_quest_marks(treasure()));
        let mut invalid = treasure();
        invalid.0[0].text = "bad\0label".into();
        assert!(!map.replace_quest_marks(invalid));
        assert_eq!(map.quest_marks, treasure());
    }
}
