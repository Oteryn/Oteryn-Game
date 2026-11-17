//! One place for each resizable window's size, minimum and content scale.
//!
//! The shared module layout state registers and persists these entries; until
//! then the entries live here and every window derives its sizes from them.
use bevy::{
    input::mouse::AccumulatedMouseScroll,
    picking::{
        events::{PointerDrag, PointerDragStart},
        hover::HoverMap,
        pointer::PointerId,
    },
    prelude::*,
    window::PrimaryWindow,
};
use std::collections::BTreeMap;

pub(in crate::bevy_shell) const MIN_ZOOM: f32 = 0.5;
pub(in crate::bevy_shell) const MAX_ZOOM: f32 = 2.;

/// The Global initial size (height `None` follows content) and the minimum, at 100%.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::bevy_shell) struct Geometry {
    pub width: f32,
    pub height: Option<f32>,
    pub min: Vec2,
}

const fn geometry(width: f32, height: Option<f32>, min: [f32; 2]) -> Geometry {
    Geometry {
        width,
        height,
        min: Vec2::new(min[0], min[1]),
    }
}

/// Ordered by window id.
pub(in crate::bevy_shell) const WINDOWS: &[(&str, Geometry)] = &[
    ("cyclopedia", geometry(700., Some(538.), [560., 400.])),
    ("market", geometry(1000., Some(634.), [800., 480.])),
    ("npc", geometry(600., None, [420., 350.])),
    ("options", geometry(685., Some(534.), [560., 420.])),
    ("selection", geometry(745., None, [560., 300.])),
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::bevy_shell) struct Layout {
    pub geometry: Geometry,
    /// `None` until the user resizes a content-sized window.
    pub size: Option<Vec2>,
    pub zoom: f32,
}

impl Layout {
    pub(in crate::bevy_shell) fn new(geometry: Geometry) -> Self {
        Self {
            geometry,
            size: geometry
                .height
                .map(|height| Vec2::new(geometry.width, height)),
            zoom: 1.,
        }
    }
    pub(in crate::bevy_shell) fn scaled(&self, value: f32) -> f32 {
        value * self.zoom
    }
    pub(in crate::bevy_shell) fn min(&self) -> Vec2 {
        self.geometry.min * self.zoom
    }
    pub(in crate::bevy_shell) fn width(&self) -> f32 {
        self.size
            .map_or(self.scaled(self.geometry.width), |size| size.x)
    }
    /// A drag on an edge or corner: never below the minimum, never past the screen.
    pub(in crate::bevy_shell) fn resize(&mut self, requested: Vec2, viewport: Vec2) {
        self.size = Some(fit(requested, self.min(), viewport));
    }
    /// Ctrl+wheel or the window menu's Zoom; the size follows the content scale.
    pub(in crate::bevy_shell) fn set_zoom(&mut self, zoom: f32, viewport: Vec2) {
        let zoom = if zoom.is_finite() {
            zoom.clamp(MIN_ZOOM, MAX_ZOOM)
        } else {
            1.
        };
        let ratio = zoom / self.zoom;
        self.zoom = zoom;
        if let Some(size) = self.size {
            self.size = Some(fit(size * ratio, self.min(), viewport));
        }
    }
    pub(in crate::bevy_shell) fn reset_zoom(&mut self, viewport: Vec2) {
        self.set_zoom(1., viewport);
    }
    /// The screen resolution or the global UI scale changed.
    pub(in crate::bevy_shell) fn fit(&mut self, viewport: Vec2) {
        if let Some(size) = self.size {
            self.size = Some(fit(size, self.min(), viewport));
        }
    }
    /// The minimum never exceeds a known screen, so a window always fits on it.
    pub(in crate::bevy_shell) fn apply(&self, node: &mut Node, viewport: Option<Vec2>) {
        let min = viewport.map_or(self.min(), |viewport| self.min().min(viewport));
        node.width = px(self.width());
        node.height = self.size.map_or(Val::Auto, |size| px(size.y));
        node.min_width = px(min.x);
        node.min_height = px(min.y);
    }
}

fn fit(requested: Vec2, min: Vec2, viewport: Vec2) -> Vec2 {
    let requested = if requested.is_finite() {
        requested
    } else {
        min
    };
    requested.max(min).min(viewport.max(Vec2::ONE))
}

/// The registry the shared layout state persists: one entry per window id.
#[derive(Resource, Clone, Debug, PartialEq)]
pub(in crate::bevy_shell) struct WindowLayouts(pub BTreeMap<&'static str, Layout>);

impl Default for WindowLayouts {
    fn default() -> Self {
        Self(
            WINDOWS
                .iter()
                .map(|(id, geometry)| (*id, Layout::new(*geometry)))
                .collect(),
        )
    }
}

/// Marks a window root whose node follows its `WindowLayouts` entry.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::bevy_shell) struct WindowId(pub &'static str);

/// Base sizes at 100% for a node inside a window; they follow the window's zoom.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub(in crate::bevy_shell) struct Scaled {
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub min_height: Option<f32>,
}

impl Scaled {
    pub(in crate::bevy_shell) fn square(side: f32) -> Self {
        Self {
            width: Some(side),
            height: Some(side),
            min_height: None,
        }
    }
    fn apply(&self, zoom: f32, node: &mut Node) {
        if let Some(width) = self.width {
            node.width = px(width * zoom);
        }
        if let Some(height) = self.height {
            node.height = px(height * zoom);
        }
        if let Some(height) = self.min_height {
            node.min_height = px(height * zoom);
        }
    }
}

/// The window root bundle at its initial size; `project` adds the screen-bounded minimum.
pub(in crate::bevy_shell) fn window(id: &'static str, mut node: Node) -> (WindowId, Node) {
    if let Some((_, geometry)) = WINDOWS.iter().find(|(candidate, _)| *candidate == id) {
        let layout = Layout::new(*geometry);
        node.width = px(layout.width());
        node.height = layout.size.map_or(Val::Auto, |size| px(size.y));
    }
    (WindowId(id), node)
}

/// The bottom-right corner a user drags to resize the window, like a desktop window.
#[derive(Component, Default)]
pub(in crate::bevy_shell) struct ResizeGrip {
    start: Option<Vec2>,
}

const GRIP: f32 = 12.;
const ZOOM_STEP: f32 = 0.1;

/// The primary window's size in UI pixels: global UI scale and DPI already applied.
fn viewport(
    screens: &Query<&Window, With<PrimaryWindow>>,
    scale: Option<&UiScale>,
) -> Option<Vec2> {
    let factor = scale.map_or(1., |scale| scale.0);
    screens
        .single()
        .ok()
        .map(|screen| Vec2::new(screen.width(), screen.height()) / factor)
        .filter(|viewport| viewport.is_finite() && viewport.min_element() > 0.)
}

fn grip_start(
    mut event: On<PointerDragStart>,
    mut grips: Query<(&ChildOf, &mut ResizeGrip)>,
    nodes: Query<&ComputedNode>,
) {
    event.propagate(false);
    if let Ok((parent, mut grip)) = grips.get_mut(event.entity)
        && let Ok(node) = nodes.get(parent.parent())
    {
        grip.start = Some(node.size() * node.inverse_scale_factor());
    }
}

fn grip_drag(
    mut event: On<PointerDrag>,
    grips: Query<(&ChildOf, &ResizeGrip)>,
    ids: Query<&WindowId>,
    screens: Query<&Window, With<PrimaryWindow>>,
    scale: Option<Res<UiScale>>,
    mut layouts: ResMut<WindowLayouts>,
) {
    event.propagate(false);
    let factor = scale.as_deref().map_or(1., |scale| scale.0);
    if let Ok((parent, grip)) = grips.get(event.entity)
        && let Some(start) = grip.start
        && let Ok(id) = ids.get(parent.parent())
        && let Some(viewport) = viewport(&screens, scale.as_deref())
        && let Some(layout) = layouts.0.get_mut(id.0)
    {
        layout.resize(start + event.distance / factor, viewport);
    }
}

/// Ctrl+wheel over a window zooms its content; Ctrl+0 resets it to 100%.
#[allow(clippy::too_many_arguments)]
pub(in crate::bevy_shell) fn zoom(
    scroll: Option<Res<AccumulatedMouseScroll>>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    hover: Option<Res<HoverMap>>,
    parents: Query<&ChildOf>,
    ids: Query<&WindowId>,
    screens: Query<&Window, With<PrimaryWindow>>,
    scale: Option<Res<UiScale>>,
    mut layouts: ResMut<WindowLayouts>,
) {
    let Some(keys) = keys else {
        return;
    };
    if !keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
        return;
    }
    let reset = keys.just_pressed(KeyCode::Digit0);
    let wheel = scroll.map_or(0., |scroll| scroll.delta.y);
    if !reset && wheel == 0. {
        return;
    }
    let Some(id) = hover
        .as_deref()
        .and_then(|hover| hover.get(&PointerId::Mouse))
        .into_iter()
        .flat_map(|hits| hits.keys())
        .find_map(|hit| {
            std::iter::once(*hit)
                .chain(parents.iter_ancestors(*hit))
                .find_map(|entity| ids.get(entity).ok())
        })
    else {
        return;
    };
    let Some(viewport) = viewport(&screens, scale.as_deref()) else {
        return;
    };
    let Some(current) = layouts.0.get(id.0).copied() else {
        return;
    };
    let mut next = current;
    if reset {
        next.reset_zoom(viewport);
    } else {
        let steps = (current.zoom / ZOOM_STEP).round() + wheel.signum();
        next.set_zoom(steps * ZOOM_STEP, viewport);
    }
    if next != current {
        layouts.0.insert(id.0, next);
    }
}

/// Clamps every window to the logical screen and projects sizes and zoom.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(in crate::bevy_shell) fn project(
    mut commands: Commands,
    mut layouts: ResMut<WindowLayouts>,
    screens: Query<&Window, With<PrimaryWindow>>,
    scale: Option<Res<UiScale>>,
    mut seen: Local<Option<Vec2>>,
    mut roots: Query<(Entity, Ref<WindowId>, &mut Node), Without<Scaled>>,
    mut scaled: Query<(Entity, Ref<Scaled>, &mut Node), Without<WindowId>>,
    parents: Query<&ChildOf>,
    ids: Query<&WindowId>,
) {
    let viewport = viewport(&screens, scale.as_deref());
    if let Some(viewport) = viewport
        && *seen != Some(viewport)
    {
        *seen = Some(viewport);
        let mut next = layouts.0.clone();
        next.values_mut().for_each(|layout| layout.fit(viewport));
        if next != layouts.0 {
            layouts.0 = next;
        }
    }
    let changed = layouts.is_changed();
    for (entity, id, mut node) in &mut roots {
        if id.is_added() {
            commands
                .spawn((
                    ResizeGrip::default(),
                    Name::new("window.resize"),
                    Node {
                        position_type: PositionType::Absolute,
                        right: px(0.),
                        bottom: px(0.),
                        width: px(GRIP),
                        height: px(GRIP),
                        ..default()
                    },
                    ChildOf(entity),
                ))
                .observe(grip_start)
                .observe(grip_drag);
        }
        if (changed || id.is_added())
            && let Some(layout) = layouts.0.get(id.0)
        {
            layout.apply(&mut node, *seen);
        }
    }
    for (entity, base, mut node) in &mut scaled {
        if !(changed || base.is_added()) {
            continue;
        }
        let zoom = parents
            .iter_ancestors(entity)
            .find_map(|ancestor| ids.get(ancestor).ok())
            .and_then(|id| layouts.0.get(id.0))
            .map_or(1., |layout| layout.zoom);
        base.apply(zoom, &mut node);
    }
}

#[cfg(test)]
mod tests;
