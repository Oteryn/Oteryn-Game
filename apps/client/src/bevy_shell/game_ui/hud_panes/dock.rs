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
}
#[derive(Serialize, Deserialize)]
struct Saved {
    left: bool,
    extra: bool,
    modules: Vec<SavedEntry>,
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
        }
    }
    /// Writes beside the target and renames, so a crash never leaves half a file.
    pub(in crate::bevy_shell) fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
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
                })
                .collect(),
        };
        let bytes = serde_json::to_vec_pretty(&saved)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let staging = path.with_extension("json.tmp");
        std::fs::write(&staging, bytes)?;
        std::fs::rename(staging, path)
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
    if path != state.path {
        if let Some(previous) = state.path.as_deref()
            && (state.pending.is_some() || widgets.is_changed())
            && let Err(error) = widgets.save(previous)
        {
            leprintln!("Could not save window layout: {error}", error = error);
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
    let Persistence { path, pending } = &mut *state;
    let Some(path) = path.as_deref() else {
        return;
    };
    if widgets.is_changed() {
        *pending = Some(now);
    }
    if pending.is_some_and(|since| now.saturating_sub(since) >= SAVE_DELAY) {
        *pending = None;
        if let Err(error) = widgets.save(path) {
            leprintln!("Could not save window layout: {error}", error = error);
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
