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
#[derive(Debug)]
struct Editor {
    stopped: std::sync::Arc<std::sync::atomic::AtomicBool>,
    thread: std::thread::JoinHandle<()>,
}
impl Editor {
    fn close(self) {
        self.stopped
            .store(true, std::sync::atomic::Ordering::Release);
        self.thread.thread().unpark();
        let _ = self.thread.join();
    }
}
// Only Windows opens this editor; tests exercise its thread ownership headlessly.
#[allow(dead_code)]
pub(super) fn open_editor(
    shared: &SharedMinimap,
    run: impl FnOnce(std::sync::Arc<std::sync::atomic::AtomicBool>) + Send + 'static,
) {
    let previous = {
        let mut map = lock(shared);
        if map.editor_open {
            return;
        }
        map.editor_open = true;
        map.editor.take()
    };
    if let Some(previous) = previous {
        previous.close();
    }
    let stopped = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let signal = stopped.clone();
    let mut map = lock(shared);
    match std::thread::Builder::new()
        .name("minimap-marker-editor".into())
        .spawn(move || run(signal))
    {
        Ok(thread) => map.editor = Some(Editor { stopped, thread }),
        Err(_) => map.editor_open = false,
    }
}
fn close_editor(shared: &SharedMinimap) {
    let editor = lock(shared).editor.take();
    // The editor saves under the map lock on exit; never join while holding it.
    if let Some(editor) = editor {
        editor.close();
    }
}
/// Shared by view snapshots; only the last session view closes its native editor.
#[derive(Debug)]
pub(in crate::play) struct EditorOwner(pub SharedMinimap);
impl Drop for EditorOwner {
    fn drop(&mut self) {
        close_editor(&self.0);
    }
}
type CellPosition = (i16, i32, i32);

#[derive(Debug)]
enum CellChange {
    Discover,
    Set(u8),
    Clear,
}

type Binding = ([u8; 32], [u8; 32], u64);
const MAX_CELLS: usize = 1_048_576;
const MAX_MARKERS: usize = 512;
const MAX_BYTES: u64 = 64 << 20;

fn cache_position(floor: i32, x: i32, y: i32) -> bool {
    (-15..=0).contains(&floor) && u16::try_from(x).is_ok() && u16::try_from(y).is_ok()
}

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
    pub cells: BTreeMap<CellPosition, u8>,
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
    #[allow(dead_code)]
    pub editor_open: bool,
    pub(super) pending_edit: Option<(i32, i32, i16)>,
    editor: Option<Editor>,
    pub own: (i32, i32, i16),
    pub feedback: String,
    root: Option<PathBuf>,
    world: Option<String>,
    binding: Option<Binding>,
    cell_changes: BTreeMap<CellPosition, CellChange>,
    load_failed: bool,
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
            pending_edit: None,
            editor: None,
            own: (0, 0, -7),
            feedback: String::new(),
            root: std::env::var_os("LOCALAPPDATA")
                .map(|path| PathBuf::from(path).join("Oteryn/minimap"))
                .or_else(|| crate::play::PreferenceStore::configuration_path("minimap")),
            world: None,
            binding: None,
            cell_changes: BTreeMap::new(),
            load_failed: false,
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
        self.pending_edit = None;
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
            self.cell_changes.clear();
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
            self.cell_changes.clear();
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
    #[cfg(test)]
    fn observe(&mut self, map: &WorldMapViewSnapshot) {
        self.observe_colours(map, &BTreeMap::new());
    }

    pub(in crate::play) fn observe_colours(
        &mut self,
        map: &WorldMapViewSnapshot,
        colours: &BTreeMap<(i16, i32, i32), u8>,
    ) {
        let binding = (
            map.header.content_generation,
            map.header.bundle_digest,
            map.header.reset_epoch,
        );
        if self.binding != Some(binding) {
            self.pending_edit = None;
            if let Err(error) = self.save() {
                self.feedback = error.to_string();
            }
            self.cells.clear();
            self.markers.clear();
            self.cell_changes.clear();
            self.binding = Some(binding);
            self.load_failed = false;
            if let Err(error) = self.load() {
                self.load_failed = true;
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
                    && cache_position(
                        i32::from(tile.position.floor),
                        tile.position.x,
                        tile.position.y,
                    )
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
                            self.cell_changes.insert(key, CellChange::Clear);
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
            if let std::collections::btree_map::Entry::Vacant(entry) = self.cells.entry(key) {
                entry.insert(55);
                self.cell_changes.insert(key, CellChange::Discover);
            }
            if let Some(&colour) = colours.get(&key)
                && let Some(&current) = self.cells.get(&key)
                && (current != colour
                    || matches!(self.cell_changes.get(&key), Some(CellChange::Discover)))
            {
                self.set_cell_colour(key, colour);
            }
        }
        if self.saved_at.elapsed() >= Duration::from_secs(5)
            && let Err(error) = self.save()
        {
            self.feedback = lfmt!("Could not save minimap: {error}", error = error);
        }
    }
    fn set_cell_colour(&mut self, key: CellPosition, colour: u8) {
        self.cells.insert(key, colour);
        self.cell_changes.insert(key, CellChange::Set(colour));
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub fn edit_marker(
        &mut self,
        x: i32,
        y: i32,
        floor: i16,
        label: String,
        color: u8,
    ) -> io::Result<()> {
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
        self.change_marker(
            (x, y, floor),
            Some(Marker {
                x,
                y,
                floor,
                label,
                color,
            }),
            false,
        )?;
        Ok(())
    }
    /// `Misc.: Create Mark`: adds a default-labelled marker at `own`. Returns `false` and leaves
    /// the map unchanged when a marker already sits there, so the caller opens it for editing.
    pub fn create_mark_at(&mut self, own: (i32, i32, i16)) -> io::Result<bool> {
        let (x, y, floor) = own;
        if !cache_position(i32::from(floor), x, y) {
            return Err(io::Error::other(tr!(
                "Use map coordinates 0-65535, floor -15 to 0 and a label of 1-128 bytes"
            )));
        }
        self.change_marker(
            own,
            Some(Marker {
                x,
                y,
                floor,
                label: tr!("Mark").into(),
                color: 0,
            }),
            true,
        )
    }
    // Invoked by native Windows controls; the headless tests exercise the same state.
    #[allow(dead_code)]
    pub fn remove_marker(&mut self, x: i32, y: i32, floor: i16) -> io::Result<()> {
        self.change_marker((x, y, floor), None, false)?;
        Ok(())
    }

    fn change_marker(
        &mut self,
        position: (i32, i32, i16),
        replacement: Option<Marker>,
        create_only: bool,
    ) -> io::Result<bool> {
        if self
            .quest_markers
            .iter()
            .any(|m| (m.x, m.y, m.floor) == position)
        {
            return Err(io::Error::other("Quest markers are supplied by the server"));
        }
        self.update_cache(|markers| {
            let index = markers.iter().position(|m| (m.x, m.y, m.floor) == position);
            match (index, replacement) {
                (Some(_), _) if create_only => return Ok(false),
                (Some(index), Some(marker)) => markers[index] = marker,
                (Some(_), None) => {
                    markers.retain(|m| (m.x, m.y, m.floor) != position);
                }
                (None, Some(marker)) => {
                    if markers.len() >= MAX_MARKERS {
                        return Err(io::Error::other(tr!(
                            "The minimap supports up to 512 markers"
                        )));
                    }
                    markers.push(marker);
                }
                (None, None) => {}
            }
            Ok(true)
        })
    }

    pub fn save(&mut self) -> io::Result<()> {
        if !self.cell_changes.is_empty() {
            self.update_cache(|_| Ok(true))?;
        }
        Ok(())
    }

    fn update_cache(
        &mut self,
        change: impl FnOnce(&mut Vec<Marker>) -> io::Result<bool>,
    ) -> io::Result<bool> {
        let Some(path) = self.path() else {
            if self.world.is_some() {
                return Err(io::Error::other("Minimap storage is unavailable"));
            }
            return change(&mut self.markers);
        };
        if self.load_failed {
            return Err(io::Error::other(tr!("Invalid minimap file")));
        }
        let _lock = Saved::lock_cache(&path)?;
        // Apply only local changes to the latest cache. The view may have loaded
        // before another client explored, cleared a cell, or edited a marker.
        let (cells, mut markers) = Saved::read_from(&path)?
            .map(|saved| (saved.cells, saved.markers))
            .unwrap_or_default();
        if !change(&mut markers)? {
            self.markers = markers;
            return Ok(false);
        }
        let mut cells = saved_cells(cells);
        for (&key, change) in &self.cell_changes {
            match *change {
                CellChange::Discover => {
                    cells.entry(key).or_insert(55);
                }
                CellChange::Set(colour) => {
                    cells.insert(key, colour);
                }
                CellChange::Clear => {
                    cells.remove(&key);
                }
            }
        }
        let saved = Saved {
            version: 1,
            cells: cells
                .iter()
                .map(|(&(f, x, y), &c)| [i32::from(f), x, y, i32::from(c)])
                .collect(),
            markers,
        };
        saved.validate()?;
        saved.write_to(&path)?;
        self.cells = cells;
        self.markers = saved.markers;
        self.cell_changes.clear();
        self.saved_at = Instant::now();
        Ok(true)
    }

    fn load(&mut self) -> io::Result<()> {
        let Some(path) = self.path() else {
            return Ok(());
        };
        let Some(saved) = Saved::read_from(&path)? else {
            return Ok(());
        };
        // Version 1 uses the same value for old item-only cells and real gray terrain.
        // Only an authoritative observation can tell which remembered cells to clear.
        self.cells = saved_cells(saved.cells);
        self.markers = saved.markers;
        Ok(())
    }
}

fn saved_cells(cells: Vec<[i32; 4]>) -> BTreeMap<CellPosition, u8> {
    cells
        .into_iter()
        .map(|[f, x, y, c]| ((f as i16, x, y), c as u8))
        .collect()
}

impl Saved {
    fn read_from(path: &std::path::Path) -> io::Result<Option<Self>> {
        let file = match std::fs::File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
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
        Ok(Some(saved))
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

#[cfg(test)]
#[path = "native.rs"]
mod native_action_tests;
