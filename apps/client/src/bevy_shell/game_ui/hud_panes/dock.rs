//! The modular dock: the sidebar modules are saved per character, follow Reset window layout
//! and take their zoom from the shared window layer (`module.*` window ids).
use super::{Dock, Entry, MAX_CONTENT, MIN_CONTENT, SidebarWidgets, Widget};
use crate::bevy_shell::{
    owner::HostOwner,
    ui::window_layout::{LayoutReset, WindowLayouts},
};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    io,
    path::{Path, PathBuf},
    time::Duration,
};

impl Widget {
    /// The window layer id that carries the module's zoom.
    pub(in crate::bevy_shell) fn window_id(self) -> &'static str {
        match self {
            Self::BattleList => "module.battle_list",
            Self::Containers => "module.containers",
            Self::Skills => "module.skills",
            Self::Vip => "module.vip",
            Self::AnalyticsSelector => "module.analytics_selector",
        }
    }
    fn from_window_id(id: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|widget| widget.window_id() == id)
    }
}

/// The zoom of a module from the shared window layer; 100% without one.
pub(super) fn zoom(layouts: Option<&WindowLayouts>, widget: Widget) -> f32 {
    layouts
        .and_then(|layouts| layouts.0.get(widget.window_id()))
        .map_or(1., |layout| layout.zoom)
}

#[derive(Serialize, Deserialize)]
struct SavedEntry {
    module: String,
    dock: Dock,
    open: bool,
    minimised: bool,
    height: Option<f32>,
    #[serde(default)]
    floating: bool,
}
#[derive(Serialize, Deserialize)]
struct Saved {
    left: bool,
    extra: bool,
    modules: Vec<SavedEntry>,
    /// The floating regions (minimap, chat).
    #[serde(default)]
    floating: Vec<String>,
}

impl Default for SidebarWidgets {
    fn default() -> Self {
        Self {
            entries: Widget::ALL
                .into_iter()
                .map(|widget| Entry {
                    widget,
                    dock: Dock::Right,
                    open: widget.open_by_default(),
                    minimised: false,
                    height: widget.content_height(),
                    floating: false,
                })
                .collect(),
            left: false,
            extra: false,
            floating: Vec::new(),
            placements: Vec::new(),
            reset_pending: false,
            saved_revision: None,
        }
    }
}

impl PartialEq for SidebarWidgets {
    fn eq(&self, other: &Self) -> bool {
        self.entries == other.entries
            && self.left == other.left
            && self.extra == other.extra
            && self.floating == other.floating
    }
}

impl SidebarWidgets {
    /// A missing file is the character's first start: the Global default dock.
    pub(in crate::bevy_shell) fn load(path: &Path) -> io::Result<Self> {
        let Some(revision) = read_revision(path)? else {
            return Ok(Self::default());
        };
        let mut widgets = serde_json::from_slice(&revision)
            .map(Self::sanitize)
            .unwrap_or_default();
        widgets.saved_revision = Some(revision);
        Ok(widgets)
    }

    /// Unknown or repeated modules are dropped and missing ones keep their default place,
    /// so a stale or hand-edited file always yields every module exactly once.
    fn sanitize(saved: Saved) -> Self {
        let mut entries: Vec<Entry> = Vec::with_capacity(Widget::ALL.len());
        for saved_entry in saved.modules {
            let Some(widget) = Widget::from_window_id(&saved_entry.module) else {
                continue;
            };
            if entries.iter().any(|entry| entry.widget == widget) {
                continue;
            }
            let dock = match saved_entry.dock {
                Dock::Left if !saved.left => Dock::Right,
                Dock::Extra if !saved.extra => Dock::Right,
                dock => dock,
            };
            entries.push(Entry {
                widget,
                dock,
                open: saved_entry.open,
                minimised: saved_entry.open && saved_entry.minimised,
                height: saved_entry
                    .height
                    .filter(|height| height.is_finite())
                    .map(|height| height.clamp(MIN_CONTENT, MAX_CONTENT))
                    .or(widget.content_height()),
                floating: saved_entry.floating,
            });
        }
        let defaults = Self::default();
        for entry in defaults.entries {
            if !entries.iter().any(|known| known.widget == entry.widget) {
                entries.push(entry);
            }
        }
        Self {
            entries,
            placements: Vec::new(),
            reset_pending: false,
            saved_revision: None,
            left: saved.left,
            extra: saved.extra,
            floating: super::float::REGIONS
                .into_iter()
                .filter(|region| saved.floating.iter().any(|saved| saved == region))
                .collect(),
        }
    }
    /// Writes beside the target and renames, so a crash never leaves half a file.
    pub(in crate::bevy_shell) fn save(&mut self, path: &Path) -> io::Result<()> {
        let _lock = crate::atomic_file::try_lock(path)?;
        let mut active = Self::load_for_write(path)?;
        if self.reset_pending || self.saved_revision == active.saved_revision {
            self.publish(path)?;
            self.reset_pending = false;
            self.placements.clear();
            return Ok(());
        }
        let before = self
            .saved_revision
            .as_deref()
            .and_then(|bytes| serde_json::from_slice(bytes).ok())
            .map(Self::sanitize)
            .unwrap_or_default();
        active.apply_edits(&before, self);
        active.publish(path)?;
        *self = active;
        Ok(())
    }

    fn load_for_write(path: &Path) -> io::Result<Self> {
        let active = Self::load(path)?;
        if active
            .saved_revision
            .as_deref()
            .is_some_and(|bytes| serde_json::from_slice::<Saved>(bytes).is_err())
        {
            crate::atomic_file::preserve_malformed(path)?;
        }
        Ok(active)
    }

    fn apply_edits(&mut self, before: &Self, edited: &Self) {
        if edited.left != before.left {
            self.left = edited.left;
        }
        if edited.extra != before.extra {
            self.extra = edited.extra;
        }
        for region in super::float::REGIONS {
            if edited.floating.contains(&region) != before.floating.contains(&region) {
                self.floating.retain(|current| *current != region);
                if edited.floating.contains(&region) {
                    self.floating.push(region);
                }
            }
        }
        for edit in &edited.entries {
            let Some(previous) = before.entry(edit.widget) else {
                continue;
            };
            let Some(active_index) = self
                .entries
                .iter()
                .position(|entry| entry.widget == edit.widget)
            else {
                continue;
            };
            let active = &mut self.entries[active_index];
            if edit.dock != previous.dock {
                active.dock = edit.dock;
            }
            if edit.open != previous.open {
                active.open = edit.open;
            }
            if edit.minimised != previous.minimised {
                active.minimised = edit.minimised;
            }
            if edit.height != previous.height {
                active.height = edit.height;
            }
            if edit.floating != previous.floating {
                active.floating = edit.floating;
            }
        }
        for &(widget, dock, before) in &edited.placements {
            if let Some(index) = self.entries.iter().position(|entry| entry.widget == widget) {
                self.reorder(index, dock, before);
            }
        }
    }

    fn publish(&mut self, path: &Path) -> io::Result<()> {
        let saved = Saved {
            left: self.left,
            extra: self.extra,
            modules: self
                .entries
                .iter()
                .map(|entry| SavedEntry {
                    module: entry.widget.window_id().into(),
                    dock: entry.dock,
                    open: entry.open,
                    minimised: entry.minimised,
                    height: entry.height,
                    floating: entry.floating,
                })
                .collect(),
            floating: self.floating.iter().map(|id| (*id).into()).collect(),
        };
        let bytes = serde_json::to_vec_pretty(&saved)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        crate::atomic_file::replace(path, &bytes, false)?;
        self.saved_revision = Some(bytes.into());
        Ok(())
    }
    /// Reset window layout: the Global default dock, saved at once.
    pub(in crate::bevy_shell) fn reset(&mut self, path: Option<&Path>) -> io::Result<()> {
        *self = Self::default();
        if let Some(path) = path {
            self.reset_pending = true;
            self.save(path)?;
        }
        Ok(())
    }
}

fn read_revision(path: &Path) -> io::Result<Option<std::sync::Arc<[u8]>>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes.into())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn install_snapshot(widgets: &mut ResMut<SidebarWidgets>, snapshot: SidebarWidgets) {
    if **widgets != snapshot {
        widgets.set_changed();
    }
    *widgets.bypass_change_detection() = snapshot;
}

/// How long the dock must stay unchanged before it is written, so a drag writes once.
const SAVE_DELAY: Duration = Duration::from_secs(1);

#[derive(Default)]
pub(super) struct Persistence {
    path: Option<PathBuf>,
    pending: Option<Duration>,
    unsaved: std::collections::BTreeMap<PathBuf, (SidebarWidgets, Duration)>,
}

/// Loads the character's dock when the character changes (saving the previous one first),
/// then writes it after each settled change.
pub(super) fn persist(
    owner: NonSend<HostOwner>,
    time: Res<Time<Real>>,
    mut widgets: ResMut<SidebarWidgets>,
    mut state: Local<Persistence>,
) {
    let path = owner.dock_layout_path();
    let now = time.elapsed();
    // Keep the previous character's edits separate while a different dock is active.
    let closing = owner.phase == crate::bevy_shell::owner::Phase::Closed;
    state.unsaved.retain(|previous, (snapshot, attempted)| {
        if Some(previous) == path.as_ref()
            || (!closing && now.saturating_sub(*attempted) < SAVE_DELAY)
        {
            return true;
        }
        *attempted = now;
        match snapshot.save(previous) {
            Ok(()) => false,
            Err(error) => {
                if closing {
                    leprintln!("Could not save window layout: {error}", error = error);
                }
                true
            }
        }
    });
    if path != state.path {
        if let Some(previous) = state.path.as_deref()
            && (state.pending.is_some() || widgets.is_changed())
            && let Err(error) = widgets.bypass_change_detection().save(previous)
        {
            leprintln!("Could not save window layout: {error}", error = error);
            let previous = previous.to_owned();
            state.unsaved.insert(previous, (widgets.clone(), now));
        }
        if let Some((snapshot, _)) = path.as_ref().and_then(|path| state.unsaved.remove(path)) {
            install_snapshot(&mut widgets, snapshot);
            state.path = path;
            state.pending = Some(now);
            return;
        }
        let next = match path.as_deref().map(SidebarWidgets::load) {
            Some(Ok(saved)) => saved,
            Some(Err(error)) => {
                eprintln!("Could not load window layouts: {error}");
                SidebarWidgets::default()
            }
            None => SidebarWidgets::default(),
        };
        install_snapshot(&mut widgets, next);
        state.path = path;
        state.pending = None;
        return;
    }
    let Persistence { path, pending, .. } = &mut *state;
    let Some(path) = path.as_deref() else {
        return;
    };
    if widgets.is_changed() {
        *pending = Some(now);
    }
    if pending.is_some_and(|since| closing || now.saturating_sub(since) >= SAVE_DELAY) {
        let mut snapshot = widgets.clone();
        if let Err(error) = snapshot.save(path) {
            *pending = Some(now);
            leprintln!("Could not save window layout: {error}", error = error);
        } else {
            install_snapshot(&mut widgets, snapshot);
            *pending = None;
        }
    }
}

/// The window menu's Reset window layout also restores the character's default dock.
pub(super) fn reset(
    _: On<LayoutReset>,
    owner: NonSend<HostOwner>,
    mut widgets: ResMut<SidebarWidgets>,
) {
    let path = owner.dock_layout_path();
    if let Err(error) = widgets.reset(path.as_deref()) {
        leprintln!("Could not save window layout: {error}", error = error);
    }
}

#[cfg(test)]
mod staging_tests {
    use super::*;

    #[test]
    fn c2c_stale_dock_move_preserves_disjoint_move() -> io::Result<()> {
        use Widget::*;
        let root = tempfile::tempdir()?;
        let path = root.path().join("dock.json");
        let mut first = SidebarWidgets::load(&path)?;
        let mut second = first.clone();
        first.place(BattleList, Dock::Right, None);
        first.save(&path)?;
        second.place(Vip, Dock::Right, Some(Containers));
        second.save(&path)?;
        let order: Vec<_> = SidebarWidgets::load(&path)?
            .entries
            .into_iter()
            .map(|e| e.widget)
            .collect();
        assert_eq!(
            order,
            [Vip, Containers, Skills, AnalyticsSelector, BattleList]
        );
        Ok(())
    }

    #[test]
    fn c2c_contended_reset_retries_on_settle() -> Result<(), Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let mut owner = HostOwner::new(Some(root.path().join("preferences.json")))?;
        owner.enter_character(Some([3; 16]));
        let path = owner.dock_layout_path().ok_or("missing dock path")?;
        let mut edited = SidebarWidgets {
            extra: true,
            ..Default::default()
        };
        edited.save(&path)?;
        let mut app = App::new();
        app.insert_non_send(owner)
            .init_resource::<SidebarWidgets>()
            .init_resource::<Time<Real>>()
            .add_observer(reset)
            .add_systems(Last, persist);
        app.update();
        let lock = crate::atomic_file::try_lock(&path)?;
        app.world_mut().trigger(LayoutReset);
        app.update();
        drop(lock);
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(SAVE_DELAY);
        app.update();
        assert_eq!(SidebarWidgets::load(&path)?, SidebarWidgets::default());
        Ok(())
    }

    #[test]
    fn review_3376_stale_dock_saves_and_resets_rebase() -> io::Result<()> {
        let root = tempfile::tempdir()?;
        let path = root.path().join("dock.json");
        let mut first = SidebarWidgets::load(&path)?;
        let mut second = SidebarWidgets::load(&path)?;
        first.left = true;
        first.save(&path)?;
        second.extra = true;
        assert!(second.save(&path).is_ok(), "stale save should rebase");
        let combined = SidebarWidgets::load(&path)?;
        assert!(combined.left && combined.extra);
        first.reset(Some(&path))?;
        assert_eq!(SidebarWidgets::load(&path)?, SidebarWidgets::default());
        Ok(())
    }

    #[test]
    fn review_3376_malformed_dock_is_recovered_on_save() -> io::Result<()> {
        let root = tempfile::tempdir()?;
        let path = root.path().join("dock.json");
        std::fs::write(&path, b"{")?;
        let mut edited = SidebarWidgets::load(&path).unwrap_or_default();
        edited.extra = true;
        assert!(edited.save(&path).is_ok());
        assert!(SidebarWidgets::load(&path)?.extra);
        Ok(())
    }

    #[test]
    fn dock_save_uses_its_own_staging_path() -> io::Result<()> {
        let root = tempfile::tempdir()?;
        let path = root.path().join("dock.json");
        let in_progress = path.with_extension("json.tmp");
        std::fs::write(&in_progress, b"another writer")?;
        SidebarWidgets::default().save(&path)?;
        assert_eq!(std::fs::read(&in_progress)?, b"another writer");
        assert!(path.is_file());
        Ok(())
    }
}
