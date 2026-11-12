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
impl Minimap {
    pub fn bind_world(&mut self, world: &str) {
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
        } else if self.world.as_deref() != Some(world) {
            if let Err(error) = self.save() {
                self.feedback = error.to_string();
            }
            self.world = Some(world.into());
            self.tracking = true;
            self.zoom = 1;
            self.binding = None;
            self.cells.clear();
            self.markers.clear();
        }
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
