//! Eight resize handles, like a desktop window: four edges and four corners.
//! The opposite edge stays where it was; the layout registry clamps the size.
use super::{Layout, WindowId, WindowLayouts, viewport};
use bevy::{
    picking::{
        events::{PointerDrag, PointerDragEnd, PointerDragStart},
        hover::Hovered,
    },
    prelude::*,
    window::{PrimaryWindow, SystemCursorIcon},
};

/// Edges are this thick; corners are this square.
pub(super) const EDGE: f32 = 4.;
pub(super) const CORNER: f32 = 12.;

/// The cursor a handle shows while hovered; the hint projection applies it.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::bevy_shell) struct ResizeCursor(pub SystemCursorIcon);

/// `side` is -1 (left/top), 0 (unchanged) or 1 (right/bottom) per axis.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub(super) struct ResizeGrip {
    pub side: IVec2,
    start: Option<(Vec2, Vec2)>,
}

/// The top-left the window must reach after the next layout pass.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub(in crate::bevy_shell) struct ResizeAnchor(pub Vec2);

pub(super) const SIDES: [(&str, IVec2, SystemCursorIcon); 8] = [
    ("n", IVec2::new(0, -1), SystemCursorIcon::NsResize),
    ("ne", IVec2::new(1, -1), SystemCursorIcon::NeswResize),
    ("e", IVec2::new(1, 0), SystemCursorIcon::EwResize),
    ("se", IVec2::new(1, 1), SystemCursorIcon::NwseResize),
    ("s", IVec2::new(0, 1), SystemCursorIcon::NsResize),
    ("sw", IVec2::new(-1, 1), SystemCursorIcon::NeswResize),
    ("w", IVec2::new(-1, 0), SystemCursorIcon::EwResize),
    ("nw", IVec2::new(-1, -1), SystemCursorIcon::NwseResize),
];

fn handle(side: IVec2) -> Node {
    let mut node = Node {
        position_type: PositionType::Absolute,
        ..default()
    };
    let edge = |side: i32, start: &mut Val, end: &mut Val, length: &mut Val| match side {
        -1 => {
            *start = px(0.);
            *length = px(EDGE);
        }
        1 => {
            *end = px(0.);
            *length = px(EDGE);
        }
        _ => {
            *start = px(CORNER);
            *end = px(CORNER);
        }
    };
    edge(side.x, &mut node.left, &mut node.right, &mut node.width);
    edge(side.y, &mut node.top, &mut node.bottom, &mut node.height);
    if side.x != 0 && side.y != 0 {
        node.width = px(CORNER);
        node.height = px(CORNER);
    }
    node
}

pub(super) fn spawn(commands: &mut Commands, window: Entity) {
    for (name, side, cursor) in SIDES {
        commands
            .spawn((
                ResizeGrip { side, start: None },
                ResizeCursor(cursor),
                Hovered::default(),
                Name::new(format!("window.resize.{name}")),
                handle(side),
                ChildOf(window),
            ))
            .observe(start)
            .observe(drag)
            .observe(end);
    }
}

/// The new top-left after a handle moved by `distance` UI pixels; the size lands in `layout`.
pub(super) fn reshape(
    layout: &mut Layout,
    side: IVec2,
    (position, size): (Vec2, Vec2),
    distance: Vec2,
    viewport: Vec2,
) -> Vec2 {
    let side = side.as_vec2();
    // A dragged edge stops at the screen edge in its own direction.
    let room = Vec2::select(side.cmplt(Vec2::ZERO), position + size, viewport - position);
    let requested = Vec2::select(
        side.cmpeq(Vec2::ZERO),
        size,
        (size + side * distance).min(room),
    );
    let preferred = layout.size.unwrap_or(Vec2::new(layout.width(), size.y));
    layout.resize(requested, viewport);
    let next = layout.size.unwrap_or(size);
    layout.size = Some(Vec2::select(side.cmpeq(Vec2::ZERO), preferred, next));
    Vec2::select(side.cmplt(Vec2::ZERO), position + size - next, position)
}

/// The window's laid-out top-left and size in UI pixels.
fn rect(node: &ComputedNode, global: &UiGlobalTransform) -> (Vec2, Vec2) {
    let size = node.size() * node.inverse_scale_factor();
    (
        global.translation * node.inverse_scale_factor() - size / 2.,
        size,
    )
}

fn start(
    mut event: On<PointerDragStart>,
    mut grips: Query<(&ChildOf, &mut ResizeGrip)>,
    windows: Query<(&ComputedNode, &UiGlobalTransform)>,
) {
    event.propagate(false);
    if let Ok((parent, mut grip)) = grips.get_mut(event.entity)
        && let Ok((node, global)) = windows.get(parent.parent())
    {
        grip.start = Some(rect(node, global));
    }
}

fn drag(
    mut event: On<PointerDrag>,
    mut commands: Commands,
    grips: Query<(&ChildOf, &ResizeGrip)>,
    ids: Query<&WindowId>,
    screens: Query<&Window, With<PrimaryWindow>>,
    scale: Option<Res<UiScale>>,
    mut layouts: ResMut<WindowLayouts>,
) {
    event.propagate(false);
    let factor = scale.as_deref().map_or(1., |scale| scale.0);
    if let Ok((parent, grip)) = grips.get(event.entity)
        && let Some(origin) = grip.start
        && let Ok(id) = ids.get(parent.parent())
        && let Some(viewport) = viewport(&screens, scale.as_deref())
        && let Some(layout) = layouts.0.get_mut(id.0)
        && event.distance.is_finite()
    {
        let anchor = reshape(layout, grip.side, origin, event.distance / factor, viewport);
        commands
            .entity(parent.parent())
            .insert(ResizeAnchor(anchor));
    }
}

fn end(mut event: On<PointerDragEnd>, mut grips: Query<&mut ResizeGrip>) {
    event.propagate(false);
    if let Ok(mut grip) = grips.get_mut(event.entity) {
        grip.start = None;
    }
}

/// The move that puts a laid-out window's top-left on its anchor.
pub(super) fn correction(anchor: Vec2, laid_out: Vec2) -> Option<Vec2> {
    let delta = anchor - laid_out;
    (delta.is_finite() && delta.length_squared() > 0.01).then_some(delta)
}

/// After layout, keeps the edge opposite the dragged handle in place.
pub(in crate::bevy_shell) fn anchor(
    mut commands: Commands,
    mut layouts: ResMut<WindowLayouts>,
    mut windows: Query<(
        &WindowId,
        Entity,
        &ResizeAnchor,
        &ComputedNode,
        &UiGlobalTransform,
        &mut UiTransform,
    )>,
) {
    for (id, entity, anchor, node, global, mut transform) in &mut windows {
        commands.entity(entity).remove::<ResizeAnchor>();
        let value = |value| match value {
            Val::Px(value) => Some(value),
            Val::Auto => Some(0.),
            _ => None,
        };
        let (Some(x), Some(y)) = (
            value(transform.translation.x),
            value(transform.translation.y),
        ) else {
            continue;
        };
        if let Some(delta) = correction(anchor.0, rect(node, global).0) {
            let offset = Vec2::new(x, y) + delta;
            transform.translation = Val2::px(offset.x, offset.y);
            if let Some(layout) = layouts.0.get_mut(id.0) {
                layout.offset = offset;
            }
        }
    }
}
