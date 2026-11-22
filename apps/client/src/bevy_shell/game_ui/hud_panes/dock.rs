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
        let _lock = crate::atomic_file::lock(path)?;
        let mut active = Self::load_for_write(path)?;
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
        for (index, edit) in edited.entries.iter().enumerate() {
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
            let previous_predecessor = before
                .entries
                .iter()
                .position(|entry| entry.widget == edit.widget)
                .and_then(|index| index.checked_sub(1))
                .map(|index| before.entries[index].widget);
            let edited_predecessor = index
                .checked_sub(1)
                .map(|index| edited.entries[index].widget);
            if previous_predecessor != edited_predecessor {
                let entry = self.entries.remove(active_index);
                let insertion = edited_predecessor
                    .and_then(|widget| self.entries.iter().position(|entry| entry.widget == widget))
                    .map_or(0, |index| index + 1);
                self.entries.insert(insertion, entry);
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
    #[cfg(test)]
    pub(in crate::bevy_shell) fn reset(&mut self, path: Option<&Path>) -> io::Result<()> {
        let mut reset = Self::default();
        if let Some(path) = path {
            reset.write_snapshot(path, true)?;
        }
        *self = reset;
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
    let revision = snapshot.saved_revision.clone();
    widgets.set_if_neq(snapshot);
    widgets.bypass_change_detection().saved_revision = revision;
}

/// How long the dock must stay unchanged before it is written, so a drag writes once.
const SAVE_DELAY: Duration = Duration::from_secs(1);

#[derive(Resource, Default)]
pub(super) struct Persistence {
    path: Option<PathBuf>,
    dirty: Option<Duration>,
    writes: std::collections::BTreeMap<PathBuf, DockWrite>,
}

struct DockWrite {
    snapshot: SidebarWidgets,
    reset: bool,
    due: Duration,
    task: Option<bevy_tasks::Task<io::Result<SidebarWidgets>>>,
    submitted: SidebarWidgets,
    submitted_reset: bool,
    changed: bool,
}

impl SidebarWidgets {
    fn write_snapshot(&mut self, path: &Path, reset: bool) -> io::Result<()> {
        if reset {
            let _lock = crate::atomic_file::lock(path)?;
            Self::load_for_write(path)?;
            self.publish(path)
        } else {
            self.save(path)
        }
    }
}

impl Drop for Persistence {
    fn drop(&mut self) {
        for (path, write) in &mut self.writes {
            if let Some(task) = write.task.take() {
                match bevy_tasks::block_on(task) {
                    Ok(mut saved) => {
                        if !write.changed {
                            continue;
                        }
                        if !write.reset {
                            saved.apply_edits(&write.submitted, &write.snapshot);
                            write.snapshot = saved;
                        }
                    }
                    Err(error) => {
                        eprintln!("Could not save dock: {error}");
                        write.reset |= write.submitted_reset;
                    }
                }
            }
            if let Err(error) = write.snapshot.write_snapshot(path, write.reset) {
                eprintln!("Could not flush dock: {error}");
            }
        }
    }
}

impl Persistence {
    #[cfg(test)]
    pub(super) fn writing(&self) -> bool {
        !self.writes.is_empty()
    }

    fn queue(&mut self, path: PathBuf, snapshot: SidebarWidgets, reset: bool, now: Duration) {
        let write = self.writes.entry(path).or_insert_with(|| DockWrite {
            submitted: snapshot.clone(),
            submitted_reset: false,
            snapshot: snapshot.clone(),
            reset: false,
            due: now,
            task: None,
            changed: false,
        });
        write.snapshot = snapshot;
        write.reset |= reset;
        write.changed = true;
        write.due = now;
    }
}

pub(super) fn persist(
    owner: NonSend<HostOwner>,
    time: Res<Time<Real>>,
    mut widgets: ResMut<SidebarWidgets>,
    mut state: ResMut<Persistence>,
) {
    let now = time.elapsed();
    let path = owner.dock_layout_path();
    let closing = owner.phase == crate::bevy_shell::owner::Phase::Closed;
    if widgets.is_changed() {
        state.dirty = Some(now);
    }
    if state.dirty.is_some_and(|since| {
        closing || path != state.path || now.saturating_sub(since) >= SAVE_DELAY
    }) {
        if let Some(previous) = state.path.clone() {
            state.queue(previous, widgets.clone(), false, now);
        }
        state.dirty = None;
    }
    if path != state.path {
        let next = path.as_ref().map_or_else(SidebarWidgets::default, |p| {
            if let Some(write) = state.writes.get(p) {
                return write.snapshot.clone();
            }
            SidebarWidgets::load(p).unwrap_or_else(|error| {
                eprintln!("Could not load window layouts: {error}");
                SidebarWidgets::default()
            })
        });
        install_snapshot(&mut widgets, next);
        state.path.clone_from(&path);
        state.dirty = None;
    }
    state.writes.retain(|target, write| {
        if let Some(result) = write
            .task
            .as_mut()
            .and_then(bevy_tasks::futures::check_ready)
        {
            write.task = None;
            match result {
                Ok(mut saved) => {
                    if !write.reset {
                        saved.apply_edits(&write.submitted, &write.snapshot);
                        if path.as_ref() == Some(target) {
                            saved.apply_edits(&write.snapshot, &widgets);
                            install_snapshot(&mut widgets, saved.clone());
                        }
                        write.snapshot = saved;
                    }
                    if !write.changed {
                        return false;
                    }
                }
                Err(error) => {
                    leprintln!("Could not save window layout: {error}", error = error);
                    write.reset |= write.submitted_reset;
                    write.due = now + SAVE_DELAY;
                }
            }
        }
        if write.task.is_none() && (closing || now >= write.due) {
            let mut edited = write.snapshot.clone();
            let target = target.clone();
            let reset = write.reset;
            write.submitted_reset = reset;
            write.submitted = edited.clone();
            write.changed = false;
            write.task = Some(bevy_tasks::IoTaskPool::get().spawn(async move {
                edited.write_snapshot(&target, reset)?;
                Ok(edited)
            }));
            write.reset = false;
        }
        true
    });
}

pub(super) fn reset(
    _: On<LayoutReset>,
    owner: NonSend<HostOwner>,
    mut widgets: ResMut<SidebarWidgets>,
    mut state: ResMut<Persistence>,
    time: Res<Time<Real>>,
) {
    *widgets = SidebarWidgets::default();
    if let Some(path) = owner.dock_layout_path() {
        state.queue(path, widgets.clone(), true, time.elapsed());
    }
}

#[cfg(test)]
mod staging_tests {
    use super::*;

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
