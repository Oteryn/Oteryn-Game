//! Native splitters for the main regions; geometry remains transient for this batch.
//! Behaviour: Tibia manual §3.1 (docs/reference/tibia-manual/interface.md).
//! Structural reference: edubart/otclient gameinterface.otui:70-77,
//! revision dab86e610991092fe8b2471f7de55a7c986b3cd2; implemented with Bevy picking.
use super::modal::ModalGuard;
use bevy::{
    camera::NormalizedRenderTarget,
    picking::{
        events::*,
        pointer::{PointerButton, PointerId},
    },
    prelude::*,
    window::{PrimaryWindow, WindowFocused, WindowRef},
};

#[derive(Component, Clone, Copy)]
pub(super) struct Splitter {
    target: Entity,
    vertical: bool,
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
        .add_observer(cancel)
        .add_systems(Update, maintain);
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
pub(super) fn spawn(commands: &mut Commands, target: Entity, vertical: bool) -> Entity {
    commands
        .spawn((
            Name::new(if vertical {
                "Resize console"
            } else {
                "Resize sidebar"
            }),
            Splitter {
                target,
                vertical,
                gesture: None,
            },
            Node {
                width: if vertical { percent(100.) } else { px(6.) },
                height: if vertical { px(6.) } else { percent(100.) },
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
    let size = (if splitter.vertical {
        node.size().y
    } else {
        node.size().x
    }) * node.inverse_scale_factor();
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
    if splitter.vertical {
        node.height = px((gesture.size - event.distance.y / scale.0).clamp(90., 600.));
    } else {
        node.width = px((gesture.size - event.distance.x / scale.0).clamp(176., 480.));
    }
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
