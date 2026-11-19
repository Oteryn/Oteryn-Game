//! Locally explored cells and user markers; geometry comes only from validated server tiles.
use oteryn_session::world_map::WorldMapViewSnapshot;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, btree_map::Entry},
    io,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

#[path = "state/markers.rs"]
mod markers;
#[path = "transfer.rs"]
mod transfer;

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
enum CacheAccess {
    Owner { _lock: Arc<std::fs::File> },
    ReadOnly(io::Error),
}

type Binding = ([u8; 32], [u8; 32], u64);
const MAX_CELLS: usize = 1_048_576;
const MAX_MARKERS: usize = 512;
const MAX_BYTES: u64 = 64 << 20;

fn cache_position(floor: i32, x: i32, y: i32) -> bool {
    (0..=15).contains(&floor) && u16::try_from(x).is_ok() && u16::try_from(y).is_ok()
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
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Saved {
    version: u8,
    cells: Vec<[i32; 4]>,
    // Accept the version 1 fields written by older clients.
    #[serde(default, rename = "observed_at", skip_serializing)]
    legacy_observed_at: Vec<u64>,
    #[serde(default, rename = "imported_at", skip_serializing)]
    _legacy_imported_at: u64,
    #[serde(default, rename = "applied_journals", skip_serializing)]
    _legacy_journal_receipts: Option<serde::de::IgnoredAny>,
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
    // Hold each cache lock for the session, including across binding changes.
    cache_access: BTreeMap<PathBuf, CacheAccess>,
    cells_dirty: bool,
    load_failed: bool,
    saved_at: Instant,
    retry_delay: Duration,
    #[cfg(bevy_shell)]
    background_save: bool,
    #[cfg(bevy_shell)]
    marker_save_queued: bool,
    #[cfg(bevy_shell)]
    cache_io: background_io::CacheIo,
    #[cfg(bevy_shell)]
    pub(super) binding_generation: u64,
    #[cfg(bevy_shell)]
    save_task: Option<bevy_tasks::Task<io::Result<()>>>,
    #[cfg(bevy_shell)]
    outgoing_saves: BTreeMap<PathBuf, background_save::OutgoingSave>,
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
            floor: 7,
            zoom: 1,
            tracking: true,
            editor_open: false,
            pending_edit: None,
            editor: None,
            own: (0, 0, 7),
            feedback: String::new(),
            root: std::env::var_os("LOCALAPPDATA")
                .map(|path| PathBuf::from(path).join("Oteryn/minimap"))
                .or_else(|| crate::play::PreferenceStore::configuration_path("minimap")),
            world: None,
            binding: None,
            cache_access: BTreeMap::new(),
            cells_dirty: false,
            load_failed: false,
            saved_at: Instant::now(),
            retry_delay: Duration::from_secs(5),
            #[cfg(bevy_shell)]
            background_save: false,
            #[cfg(bevy_shell)]
            marker_save_queued: false,
            #[cfg(bevy_shell)]
            cache_io: Default::default(),
            #[cfg(bevy_shell)]
            binding_generation: 0,
            #[cfg(bevy_shell)]
            save_task: None,
            #[cfg(bevy_shell)]
            outgoing_saves: BTreeMap::new(),
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
        #[cfg(bevy_shell)]
        if self.world.as_deref() != Some(world) {
            self.binding_generation = self.binding_generation.saturating_add(1);
        }
        self.pending_edit = None;
        // Native admission calls this even when another Character enters the same World.
        self.quest_marks = Default::default();
        self.quest_markers.clear();
        if !(1..=36).contains(&world.len())
            || !world.bytes().all(|c| c.is_ascii_hexdigit() || c == b'-')
        {
            if let Err(error) = self.save_before_rebind() {
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
            self.cells_dirty = false;
            self.tracking = true;
            self.zoom = 1;
            self.quest_marks = Default::default();
        } else if self.world.as_deref() != Some(world) {
            if let Err(error) = self.save_before_rebind() {
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
            self.cells_dirty = false;
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
            #[cfg(bevy_shell)]
            {
                self.binding_generation = self.binding_generation.saturating_add(1);
            }
            self.pending_edit = None;
            if let Err(error) = self.save_before_rebind() {
                self.feedback = error.to_string();
            }
            self.cells.clear();
            self.markers.clear();
            self.cells_dirty = false;
            self.binding = Some(binding);
            self.retry_delay = Duration::from_secs(5);
            self.load_failed = false;
            if let Err(error) = self.load() {
                self.load_failed = true;
                self.feedback = lfmt!("Could not load minimap: {error}", error = error);
            }
        }
        // The snapshot is authoritative only within its visible window. Retain distant
        // exploration, but remove cleared cells and item-only tiles from remembered ground.
        use std::collections::BTreeSet;
        use oteryn_session::world_map::{
            VIEW_HEIGHT, VIEW_LEFT, VIEW_TOP, VIEW_WIDTH, floors_in_view, in_window,
        };
        let origin = map.header.origin;
        let observed: BTreeSet<_> = map
            .tiles
            .iter()
            .filter(|tile| {
                cache_position(i32::from(tile.position.floor), tile.position.x, tile.position.y)
                    && tile.ground_speed > 0
                    && tile.position.floor >= map.header.first_visible_floor
                    && in_window(origin, tile.position)
            })
            .map(|tile| (tile.position.floor, tile.position.x, tile.position.y))
            .collect();
        if let Some(floors) = floors_in_view(origin.floor) {
            for floor in floors.filter(|f| *f >= map.header.first_visible_floor) {
                let shift = i64::from(origin.floor) - i64::from(floor);
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
                            self.cells_dirty = true;
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
            if let Some(&colour) = colours.get(&key) {
                self.set_cell_colour(key, colour);
            } else if let Entry::Vacant(entry) = self.cells.entry(key) {
                entry.insert(55);
                self.cells_dirty = true;
            }
        }
        self.save_if_due();
    }

    fn save_if_due(&mut self) {
        if !self.cells_dirty || self.saved_at.elapsed() < self.retry_delay {
            return;
        }
        #[cfg(bevy_shell)]
        if self.background_save {
            if self.background_io_owns_cache() || self.is_read_only() {
                return;
            }
            if self.save_task.is_none()
                && let Err(error) = self.start_save()
            {
                self.save_failed(error);
            }
            return;
        }
        if let Err(error) = self.save() {
            self.save_failed(error);
        }
    }

    fn save_failed(&mut self, error: io::Error) {
        self.cells_dirty = true;
        self.saved_at = Instant::now();
        self.retry_delay = (self.retry_delay * 2).min(Duration::from_secs(300));
        self.feedback = lfmt!("Could not save minimap: {error}", error = error);
    }

    fn set_cell_colour(&mut self, key: CellPosition, colour: u8) {
        if self.cells.insert(key, colour) != Some(colour) {
            self.cells_dirty = true;
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
    pub fn save(&mut self) -> io::Result<()> {
        if !self.cells_dirty {
            return Ok(());
        }
        match self.path() {
            Some(path)
                if matches!(self.cache_access.get(&path), Some(CacheAccess::ReadOnly(_))) => {}
            Some(path) => {
                self.require_cache_owner(&path)?;
                #[cfg(bevy_shell)]
                if self.save_task.is_some() {
                    return self.start_save();
                }
                return self.write_cache(&path, self.markers.clone());
            }
            None if self.world.is_some() => {
                return Err(io::Error::other("Minimap storage is unavailable"));
            }
            None => {}
        }
        self.cells_dirty = false;
        Ok(())
    }

    pub(in crate::play) fn is_read_only(&self) -> bool {
        self.path().is_some_and(|path| {
            matches!(self.cache_access.get(&path), Some(CacheAccess::ReadOnly(_)))
        })
    }

    fn require_cache_owner(&self, path: &Path) -> io::Result<()> {
        #[cfg(bevy_shell)]
        if self.background_io_owns_cache() {
            return Err(io::Error::other("Wait for minimap storage to finish"));
        }
        match self.cache_access.get(path) {
            Some(CacheAccess::Owner { .. }) => Ok(()),
            Some(CacheAccess::ReadOnly(error)) => {
                Err(io::Error::new(error.kind(), error.to_string()))
            }
            None => Err(io::Error::other("Minimap storage is unavailable")),
        }
    }

    fn snapshot(&self, markers: Vec<Marker>) -> Saved {
        Saved {
            cells: self
                .cells
                .iter()
                .map(|(&(f, x, y), &colour)| [i32::from(f), x, y, i32::from(colour)])
                .collect(),
            markers,
            ..Saved::default()
        }
    }

    fn write_cache(&mut self, path: &Path, markers: Vec<Marker>) -> io::Result<()> {
        if self.load_failed {
            return Err(io::Error::other(tr!("Invalid minimap file")));
        }
        let saved = self.snapshot(markers);
        saved.validate()?;
        saved.write_to(path)?;
        self.markers = saved.markers;
        #[cfg(bevy_shell)]
        {
            self.marker_save_queued = false;
        }
        self.cells_dirty = false;
        self.saved_at = Instant::now();
        self.retry_delay = Duration::from_secs(5);
        Ok(())
    }

    fn load(&mut self) -> io::Result<()> {
        let Some(path) = self.path() else {
            return Ok(());
        };
        #[cfg(bevy_shell)]
        if self.load_outgoing_save(&path) {
            return Ok(());
        }
        #[cfg(bevy_shell)]
        if self.background_save {
            self.load_in_background(path);
            return Ok(());
        }
        self.cache_access.entry(path.clone()).or_insert_with(|| CacheAccess::open(&path));
        if let Some(saved) = Saved::read_from(&path)? {
            self.cells = saved_cells(saved.cells);
            self.markers = saved.markers;
        }
        Ok(())
    }
}

fn saved_cells(cells: Vec<[i32; 4]>) -> BTreeMap<CellPosition, u8> {
    cells
        .into_iter()
        .map(|[f, x, y, c]| ((f as i16, x, y), c as u8))
        .collect()
}

impl Default for Saved {
    fn default() -> Self {
        Self {
            version: 1,
            cells: Vec::new(),
            legacy_observed_at: Vec::new(),
            _legacy_imported_at: 0,
            _legacy_journal_receipts: None,
            markers: Vec::new(),
        }
    }
}

impl Saved {
    fn read_from(path: &std::path::Path) -> io::Result<Option<Self>> {
        let file = match std::fs::File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        let mut saved: Saved = transfer::read_json(file)?;
        saved.normalize_floor_sign();
        saved.validate()?;
        Ok(Some(saved))
    }
}
impl Drop for Minimap {
    fn drop(&mut self) {
        let _ = self.save_before_rebind();
        #[cfg(bevy_shell)]
        {
            self.detach_outgoing_saves();
            self.detach_imports();
        }
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
#[path = "tests/quest_marks.rs"]
mod quest_mark_tests;

#[cfg(test)]
#[path = "native.rs"]
mod native_action_tests;

#[path = "state/background_save.rs"]
mod background_save;

#[cfg(bevy_shell)]
#[path = "state/background_io.rs"]
mod background_io;
