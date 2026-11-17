//! The native console splitter (the Tibia sidebar has a fixed width); its height and the main window geometry are kept
//! in the local layout file across sessions.
//! Behaviour: Tibia manual §3.1 (docs/reference/tibia-manual/interface.md).
//! Structural reference: edubart/otclient gameinterface.otui:70-77,
//! revision dab86e610991092fe8b2471f7de55a7c986b3cd2; implemented with Bevy picking.
use super::modal::ModalGuard;
use crate::window_layout::WindowLayout;
use bevy::{
    camera::NormalizedRenderTarget,
    picking::{
        events::*,
        pointer::{PointerButton, PointerId},
    },
    prelude::*,
    window::{PrimaryWindow, WindowFocused, WindowMode, WindowPosition, WindowRef},
};
use std::path::PathBuf;

/// Layout entry of the main window: physical desktop position and logical size.
const WINDOW: &str = "window.logical";
// The layout format caps rectangle sizes at 4096 but supports desktop coordinates
// up to 32768. A point in window-local pixels records the full logical extent.
const WINDOW_EXTENT: &str = "window.logical.extent";
/// The local layout file and what it holds; without a path the layout stays transient.
#[derive(Resource, Default)]
pub(in crate::bevy_shell) struct LayoutStore {
    path: Option<PathBuf>,
    layout: WindowLayout,
    pending_save: bool,
    migrated: bool,
    #[cfg(test)]
    rejection_count: usize,
    retry_after: std::time::Duration,
    rejected: std::collections::HashMap<&'static str, [i32; 4]>,
}
impl LayoutStore {
    /// Reads the layout file; a missing or invalid file starts from the default layout.
    pub(in crate::bevy_shell) fn load(path: Option<PathBuf>) -> Self {
        let layout = path
            .as_deref()
            .and_then(|path| WindowLayout::load(path).ok())
            .unwrap_or_default();
        Self {
            path,
            layout,
            ..default()
        }
    }
    pub(in crate::bevy_shell) fn primary_window_plugin(&self) -> WindowPlugin {
        let mut window = Window {
            title: "Oteryn".into(),
            resolution: (1024, 720).into(),
            resize_constraints: bevy::window::WindowResizeConstraints {
                min_width: 1020.,
                min_height: 650.,
                ..default()
            },
            ..default()
        };
        if let Some([x, y, width, height]) = self.window()
            && let (Ok(width), Ok(height)) = (u32::try_from(width), u32::try_from(height))
        {
            window.position = bevy::window::WindowPosition::At(IVec2::new(x, y));
            window.resolution = (width, height).into();
        }
        if let Some([x, y, _, _]) = self.legacy_window() {
            window.position = bevy::window::WindowPosition::At(IVec2::new(x, y));
        }
        WindowPlugin {
            close_when_requested: false,
            primary_window: Some(window),
            ..default()
        }
    }
    /// The remembered main window physical desktop position and logical size.
    pub(in crate::bevy_shell) fn window(&self) -> Option<[i32; 4]> {
        self.bounds_with_extent(WINDOW, WINDOW_EXTENT)
    }
    /// Unversioned files stored physical pixels. Restore them once the backend knows the monitor.
    pub(in crate::bevy_shell) fn legacy_window(&self) -> Option<[i32; 4]> {
        if self.window().is_some() || self.migrated {
            return None;
        }
        self.bounds_with_extent("window", "window.extent")
    }
    fn bounds_with_extent(&self, id: &str, extent: &str) -> Option<[i32; 4]> {
        let [x, y, width, height] = self.layout.bounds(id)?;
        if let Some([w, h, 1, 1]) = self.layout.bounds(extent)
            && w.min(4096) == width
            && h.min(4096) == height
        {
            return Some([x, y, w, h]);
        }
        Some([x, y, width, height])
    }

    fn remember_bounds(&mut self, id: &'static str, bounds: [i32; 4]) {
        if self.rejected.get(id) == Some(&bounds) {
            return;
        }
        self.rejected.remove(id);
        let result = if id == WINDOW {
            if self.window() == Some(bounds) {
                return;
            }
            self.record_window(bounds)
        } else {
            if self.layout.bounds(id) == Some(bounds) {
                return;
            }
            self.layout.record(id, bounds)
        };
        match result {
            Ok(()) => self.pending_save = true,
            Err(error) => {
                #[cfg(test)]
                {
                    self.rejection_count += 1;
                }
                self.rejected.insert(id, bounds);
                leprintln!("Could not save window layout: {error}", error = error);
            }
        }
    }

    fn record_window(&mut self, bounds: [i32; 4]) -> std::io::Result<()> {
        let [x, y, width, height] = bounds;
        let base = [x, y, width.min(4096), height.min(4096)];
        if base != bounds || self.layout.bounds(WINDOW_EXTENT).is_some() {
            // Validate the origin before changing the extent of an existing window.
            WindowLayout::default().record(WINDOW, base)?;
            self.layout.record(WINDOW_EXTENT, [width, height, 1, 1])?;
        }
        self.layout.record(WINDOW, base)
    }
}

#[derive(Component, Clone, Copy)]
pub(super) struct Splitter {
    id: &'static str,
    target: Entity,
    gesture: Option<Gesture>,
}
#[derive(Clone, Copy)]
struct Gesture {
    pointer: PointerId,
    size: f32,
    scale: f32,
    window_size: Vec2,
    dpi: f32,
}

pub(super) fn install(app: &mut App) {
    app.add_message::<WindowFocused>()
        .init_resource::<LayoutStore>()
        .init_resource::<Time<Real>>()
        .add_observer(cancel)
        .add_systems(Startup, restore.after(super::setup))
        .add_systems(Update, (maintain, remember).chain());
}
/// Applies the remembered console height within the same bounds a drag allows.
fn restore(store: Res<LayoutStore>, splitters: Query<&Splitter>, mut nodes: Query<&mut Node>) {
    for splitter in &splitters {
        let Some([_, _, _, height]) = store.layout.bounds(splitter.id) else {
            continue;
        };
        let Ok(mut node) = nodes.get_mut(splitter.target) else {
            continue;
        };
        node.height = px((height as f32).clamp(90., 600.));
    }
}
/// Writes the region sizes and the windowed main window geometry when they change; a drag
/// in progress is written once it ends.
fn remember(
    mut store: ResMut<LayoutStore>,
    time: Res<Time<Real>>,
    splitters: Query<&Splitter>,
    nodes: Query<&Node>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if store.path.is_none() || splitters.iter().any(|splitter| splitter.gesture.is_some()) {
        return;
    }
    for splitter in &splitters {
        let Ok(node) = nodes.get(splitter.target) else {
            continue;
        };
        let Val::Px(size) = node.height else {
            continue;
        };
        let size = size.round() as i32;
        let bounds = [0, 0, 1, size];
        store.remember_bounds(splitter.id, bounds);
    }
    if let Some([_, _, width, height]) = store.legacy_window()
        && let Ok(mut window) = windows.single_mut()
        && window.mode == WindowMode::Windowed
    {
        window
            .resolution
            .set_physical_resolution(width as u32, height as u32);
        store.migrated = true;
    }
    if let Ok(window) = windows.single()
        && window.mode == WindowMode::Windowed
        && let WindowPosition::At(position) = window.position
    {
        let bounds = [
            position.x,
            position.y,
            window.width().round() as i32,
            window.height().round() as i32,
        ];
        store.remember_bounds(WINDOW, bounds);
    }
    if store.pending_save
        && time.elapsed() >= store.retry_after
        && let Some(path) = &store.path
    {
        match store.layout.save(path) {
            Ok(()) => store.pending_save = false,
            Err(error) => {
                store.retry_after = time.elapsed() + std::time::Duration::from_secs(1);
                leprintln!("Could not save window layout: {error}", error = error);
            }
        }
    }
}
fn maintain(
    mut focus: MessageReader<WindowFocused>,
    mut splitters: Query<(Entity, &mut Splitter)>,
    windows: Query<&Window, With<PrimaryWindow>>,
    scale: Res<UiScale>,
    guard: ModalGuard,
) {
    // Drain every message so a queued loss cannot cancel a fresh gesture next frame.
    let mut lost_focus = false;
    for event in focus.read() {
        lost_focus |= !event.focused;
    }
    for (entity, mut splitter) in &mut splitters {
        if splitter.gesture.is_some_and(|gesture| {
            lost_focus
                || !guard.allows(entity)
                || windows.single().map_or(true, |window| {
                    !window.focused
                        || window.size() != gesture.window_size
                        || scale.0 != gesture.scale
                        || window.scale_factor() != gesture.dpi
                })
        }) {
            splitter.gesture = None;
        }
    }
}
pub(super) fn spawn(commands: &mut Commands, target: Entity, id: &'static str) -> Entity {
    commands
        .spawn((
            Name::new("Resize console"),
            Splitter {
                id,
                target,
                gesture: None,
            },
            Node {
                width: percent(100.),
                height: px(6.),
                flex_shrink: 0.,
                ..default()
            },
            BackgroundColor(Color::srgb(0.32, 0.32, 0.32)),
        ))
        .observe(start)
        .observe(drag)
        .observe(end)
        .id()
}
fn start(
    mut event: On<PointerDragStart>,
    mut splitters: Query<&mut Splitter>,
    nodes: Query<&ComputedNode>,
    windows: Query<(Entity, &Window), With<PrimaryWindow>>,
    scale: Res<UiScale>,
    guard: ModalGuard,
) {
    let Ok(mut splitter) = splitters.get_mut(event.entity) else {
        return;
    };
    if splitter.gesture.is_some() {
        return;
    }
    let Ok((window_entity, window)) = windows.single() else {
        return;
    };
    if event.button != PointerButton::Primary
        || !window.focused
        || !guard.allows(event.entity)
        || WindowRef::Entity(window_entity)
            .normalize(None)
            .map(NormalizedRenderTarget::Window)
            .as_ref()
            != Some(&event.pointer.target)
    {
        return;
    }
    let Ok(node) = nodes.get(splitter.target) else {
        return;
    };
    let size = node.size().y * node.inverse_scale_factor();
    if !size.is_finite() || size <= 0. {
        return;
    }
    splitter.gesture = Some(Gesture {
        pointer: event.pointer.id,
        size,
        scale: scale.0,
        window_size: window.size(),
        dpi: window.scale_factor(),
    });
    event.propagate(false);
}
fn drag(
    mut event: On<PointerDrag>,
    mut splitters: Query<&mut Splitter>,
    windows: Query<(Entity, &Window), With<PrimaryWindow>>,
    scale: Res<UiScale>,
    mut ui: ParamSet<(ModalGuard, Query<&mut Node>)>,
) {
    let Ok(mut splitter) = splitters.get_mut(event.entity) else {
        return;
    };
    let Some(gesture) = splitter.gesture else {
        return;
    };
    let Ok((window_entity, window)) = windows.single() else {
        return;
    };
    if !window.focused
        || scale.0 != gesture.scale
        || window.size() != gesture.window_size
        || window.scale_factor() != gesture.dpi
        || WindowRef::Entity(window_entity)
            .normalize(None)
            .map(NormalizedRenderTarget::Window)
            .as_ref()
            != Some(&event.pointer.target)
        || !ui.p0().allows(event.entity)
    {
        splitter.gesture = None;
        return;
    }
    if event.button != PointerButton::Primary
        || event.pointer.id != gesture.pointer
        || !event.distance.is_finite()
        || !scale.0.is_finite()
        || scale.0 <= 0.
    {
        return;
    }
    let mut nodes = ui.p1();
    let Ok(mut node) = nodes.get_mut(splitter.target) else {
        return;
    };
    node.height = px((gesture.size - event.distance.y / scale.0).clamp(90., 600.));
    event.propagate(false);
}
fn end(event: On<PointerDragEnd>, mut splitters: Query<&mut Splitter>) {
    if let Ok(mut splitter) = splitters.get_mut(event.entity)
        && splitter
            .gesture
            .is_some_and(|gesture| gesture.pointer == event.pointer.id)
        && event.button == PointerButton::Primary
    {
        splitter.gesture = None;
    }
}
fn cancel(event: On<PointerCancel>, mut splitters: Query<&mut Splitter>) {
    for mut splitter in &mut splitters {
        if splitter
            .gesture
            .is_some_and(|gesture| gesture.pointer == event.pointer.id)
        {
            splitter.gesture = None;
        }
    }
}
#[cfg(test)]
#[path = "layout_tests.rs"]
mod tests;
