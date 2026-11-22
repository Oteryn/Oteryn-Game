use super::*;
use bevy::ecs::message::MessageCursor;
use serde::{Deserialize, Serialize};
use std::io::Read;

const MAX_LAYOUT_BYTES: u64 = 256 * 1024;

/// What persists per window: size, zoom and position; the Global geometry is code.
#[derive(Serialize, Deserialize)]
struct Saved {
    size: Option<[f32; 2]>,
    zoom: f32,
    offset: [f32; 2],
}

impl Serialize for WindowLayouts {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let saved: BTreeMap<&str, Saved> = self
            .0
            .iter()
            .map(|(id, layout)| {
                (
                    *id,
                    Saved {
                        size: layout.size.map(|size| size.to_array()),
                        zoom: layout.zoom,
                        offset: layout.offset.to_array(),
                    },
                )
            })
            .collect();
        saved.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for WindowLayouts {
    /// Unknown windows are dropped and out-of-range values fall back, so a stale
    /// or hand-edited file can never place a window below its minimum.
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let saved = BTreeMap::<String, Saved>::deserialize(deserializer)?;
        let mut layouts = Self::default();
        for (id, layout) in &mut layouts.0 {
            let Some(saved) = saved.get(*id) else {
                continue;
            };
            if saved.zoom.is_finite() {
                layout.zoom = saved.zoom.clamp(MIN_ZOOM, MAX_ZOOM);
            }
            let offset = Vec2::from_array(saved.offset);
            if offset.is_finite() {
                layout.offset = offset;
            }
            if let Some(size) = saved.size.map(Vec2::from_array)
                && size.is_finite()
            {
                layout.size = Some(size.max(layout.min()));
            }
        }
        Ok(layouts)
    }
}

impl WindowLayouts {
    /// A missing file is the first start: every window at its Global size.
    pub(in crate::bevy_shell) fn load(path: &Path) -> io::Result<Self> {
        let file = match std::fs::File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => return Err(error),
        };
        let mut bytes = Vec::new();
        file.take(MAX_LAYOUT_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_LAYOUT_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "window layout is too large",
            ));
        }
        serde_json::from_slice(&bytes)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }
    /// The window menu's Reset window layout: Global position, size and zoom, saved at once.
    pub(in crate::bevy_shell) fn reset(&mut self, path: Option<&Path>) -> io::Result<()> {
        *self = Self::default();
        path.map_or(Ok(()), |path| self.save(path))
    }
    /// Writes beside the target and renames, so a crash never leaves half a file.
    pub(in crate::bevy_shell) fn save(&self, path: &Path) -> io::Result<()> {
        Self::locked(path, || self.publish(path))
    }

    fn locked(path: &Path, write: impl FnOnce() -> io::Result<()>) -> io::Result<()> {
        let _lock = crate::atomic_file::try_lock(path)?;
        write()
    }

    fn publish(&self, path: &Path) -> io::Result<()> {
        let bytes = serde_json::to_vec_pretty(self)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        crate::atomic_file::replace(path, &bytes, false)
    }
}

/// How long the layout must stay unchanged before it is written, so a drag writes once.
const SAVE_DELAY: std::time::Duration = std::time::Duration::from_secs(1);
const EXIT_SAVE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(1);

#[derive(Resource, Default)]
pub(super) struct SavedBaseline {
    pub(super) layouts: Option<WindowLayouts>,
    pub(super) reset_pending: bool,
}

pub(super) fn load_layouts(
    owner: Option<NonSend<crate::bevy_shell::owner::HostOwner>>,
    mut layouts: ResMut<WindowLayouts>,
    mut loaded: ResMut<SavedBaseline>,
) {
    if loaded.layouts.is_some() {
        return;
    }
    let Some(path) = owner.and_then(|owner| owner.window_layouts_path()) else {
        return;
    };
    match WindowLayouts::load(&path) {
        Ok(saved) => *layouts = saved,
        Err(error) => eprintln!("Could not load window layouts: {error}"),
    }
    loaded.layouts = Some(layouts.clone());
}

#[derive(Default)]
pub(super) struct ExitSave {
    reader: MessageCursor<AppExit>,
    deferred: Option<AppExit>,
    deadline: Option<std::time::Duration>,
}

impl ExitSave {
    fn retain(&mut self, exits: &mut Messages<AppExit>) {
        for exit in exits.drain() {
            if self.deferred.is_none()
                || (exit.is_error() && self.deferred.as_ref().is_some_and(AppExit::is_success))
            {
                self.deferred = Some(exit);
            }
        }
    }
}

/// Observe edits after the final resize correction, including on the exit frame.
pub(super) fn persist(
    owner: Option<NonSend<crate::bevy_shell::owner::HostOwner>>,
    time: Res<Time<Real>>,
    layouts: Res<WindowLayouts>,
    mut loaded: ResMut<SavedBaseline>,
    mut pending: Local<Option<std::time::Duration>>,
    mut exits: ResMut<Messages<AppExit>>,
    mut exit_save: Local<ExitSave>,
) {
    let new_exit = exit_save.reader.read(&exits).count() > 0;
    let exiting = new_exit || exit_save.deferred.is_some();
    if exit_save.deferred.is_some() {
        exit_save.retain(&mut exits);
    }
    let deadline_reached = exit_save
        .deadline
        .is_some_and(|deadline| time.elapsed() >= deadline);
    if let Some(baseline) = loaded.layouts.as_ref()
        && (loaded.reset_pending || baseline != &*layouts)
        && let Some(path) = owner.and_then(|owner| owner.window_layouts_path())
    {
        if layouts.is_changed() || pending.is_none() {
            *pending = Some(time.elapsed());
        }
        if !exiting
            && pending.is_some_and(|since| time.elapsed().saturating_sub(since) < SAVE_DELAY)
        {
            return;
        }
        let result = WindowLayouts::locked(&path, || {
            if loaded.reset_pending {
                return layouts.publish(&path);
            }
            let mut latest = match WindowLayouts::load(&path) {
                Err(error) if error.kind() == io::ErrorKind::InvalidData => baseline.clone(),
                result => result?,
            };
            for (&id, layout) in &layouts.0 {
                let (Some(before), Some(saved)) = (baseline.0.get(id), latest.0.get_mut(id)) else {
                    continue;
                };
                if layout.size != before.size {
                    saved.size = match (
                        before.size.or(layout.first_resize_origin),
                        layout.size,
                        saved.size,
                    ) {
                        (Some(before), Some(edited), Some(latest)) => {
                            Some(Vec2::select(edited.cmpne(before), edited, latest))
                        }
                        _ => layout.size,
                    };
                }
                if layout.zoom != before.zoom {
                    saved.zoom = layout.zoom;
                }
                saved.offset = Vec2::select(
                    layout.offset.cmpne(before.offset),
                    layout.offset,
                    saved.offset,
                );
            }
            latest.publish(&path)
        });
        match result {
            Ok(()) => {
                loaded.layouts = Some(layouts.clone());
                loaded.reset_pending = false;
                *pending = None;
            }
            Err(error) => {
                *pending = Some(time.elapsed());
                if error.kind() == io::ErrorKind::WouldBlock {
                    if deadline_reached {
                        bevy::log::warn!(
                            target: "oteryn_client::bevy_shell::ui::window_layout",
                            "Could not save window layout before exit: layout lock is still held"
                        );
                    } else if exiting {
                        exit_save
                            .deadline
                            .get_or_insert(time.elapsed() + EXIT_SAVE_TIMEOUT);
                        // The runner reads AppExit after this schedule.
                        exit_save.retain(&mut exits);
                        return;
                    }
                } else {
                    leprintln!("Could not save window layout: {error}", error = error);
                }
            }
        }
    } else {
        *pending = None;
    }
    exit_save.deadline = None;
    if let Some(exit) = exit_save.deferred.take() {
        exits.write(exit);
    }
}
