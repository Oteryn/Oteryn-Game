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

impl SidebarWidgets {
    /// A missing file is the character's first start: the Global default dock.
    pub(in crate::bevy_shell) fn load(path: &Path) -> io::Result<Self> {
        let saved: Saved = match std::fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => return Err(error),
        };
        Ok(Self::sanitize(saved))
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
            left: saved.left,
            extra: saved.extra,
            floating: super::float::REGIONS
                .into_iter()
                .filter(|region| saved.floating.iter().any(|saved| saved == region))
                .collect(),
        }
    }
    /// Writes beside the target and renames, so a crash never leaves half a file.
    pub(in crate::bevy_shell) fn save(&self, path: &Path) -> io::Result<()> {
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
        crate::atomic_file::replace(path, &bytes, false)
    }
    /// Reset window layout: the Global default dock, saved at once.
    pub(in crate::bevy_shell) fn reset(&mut self, path: Option<&Path>) -> io::Result<()> {
        *self = Self::default();
        path.map_or(Ok(()), |path| self.save(path))
    }
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
        snapshot.save(previous).is_err()
    });
    if path != state.path {
        if let Some(previous) = state.path.as_deref()
            && (state.pending.is_some() || widgets.is_changed())
            && let Err(error) = widgets.save(previous)
        {
            leprintln!("Could not save window layout: {error}", error = error);
            let previous = previous.to_owned();
            state.unsaved.insert(previous, (widgets.clone(), now));
        }
        if let Some((snapshot, _)) = path.as_ref().and_then(|path| state.unsaved.remove(path)) {
            widgets.set_if_neq(snapshot);
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
        widgets.set_if_neq(next);
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
        if let Err(error) = widgets.save(path) {
            *pending = Some(now);
            leprintln!("Could not save window layout: {error}", error = error);
        } else {
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
