//! Sidebar widget pane dragging and resizing.
use super::*;

pub(super) fn move_start(
    mut event: On<PointerDragStart>,
    mut bars: Query<&mut TitleBar>,
    state: Res<SidebarWidgets>,
    guard: ModalGuard,
) {
    if event.button != PointerButton::Primary || !guard.allows(event.entity) {
        return;
    }
    if let Ok(mut bar) = bars.get_mut(event.entity) {
        bar.pointer = Some(event.pointer.id);
        // A floating module's title also moves its window; a drop on a pane docks it.
        if !state.entry(bar.widget).is_some_and(|entry| entry.floating) {
            event.propagate(false);
        }
    }
}

/// Drops a dragged widget into the widget pane under the pointer, before the first widget
/// whose middle lies below it.
// Bevy injects the independent ECS queries and resources a drop needs.
#[allow(clippy::too_many_arguments)]
pub(super) fn move_end(
    event: On<PointerDragEnd>,
    mut bars: Query<&mut TitleBar>,
    guard: ModalGuard,
    sidebar: Res<super::super::Sidebar>,
    frames: Query<(&WidgetFrame, &ChildOf, &ComputedNode, &UiGlobalTransform)>,
    panes: Query<(Entity, &ComputedNode, &UiGlobalTransform), With<WidgetPane>>,
    scale: Res<UiScale>,
    mut state: ResMut<SidebarWidgets>,
) {
    let Ok(mut bar) = bars.get_mut(event.entity) else {
        return;
    };
    if bar.pointer.take() != Some(event.pointer.id)
        || event.button != PointerButton::Primary
        || !guard.allows(event.entity)
    {
        return;
    }
    let at = event.pointer.location().position;
    let logical = |node: &ComputedNode, transform: &UiGlobalTransform| {
        let factor = node.inverse_scale_factor * scale.0;
        Rect::from_center_size(transform.translation * factor, node.size() * factor)
    };
    let Some(pane) = panes
        .iter()
        .find(|(_, node, transform)| logical(node, transform).contains(at))
        .map(|(entity, _, _)| entity)
    else {
        return;
    };
    let dock = if pane == sidebar.left_widgets {
        Dock::Left
    } else if pane == sidebar.extra_widgets {
        Dock::Extra
    } else {
        Dock::Right
    };
    let before = state
        .entries
        .iter()
        .filter(|entry| entry.dock == dock && entry.widget != bar.widget)
        .find(|entry| {
            frames.iter().any(|(frame, parent, node, transform)| {
                frame.0 == entry.widget
                    && parent.parent() == pane
                    && logical(node, transform).center().y > at.y
            })
        })
        .map(|entry| entry.widget);
    state.place(bar.widget, dock, before);
}

pub(super) fn resize_start(
    mut event: On<PointerDragStart>,
    mut handles: Query<&mut Resize>,
    bodies: Query<(&Body, &ComputedNode)>,
    layouts: Option<Res<WindowLayouts>>,
    guard: ModalGuard,
) {
    if event.button != PointerButton::Primary || !guard.allows(event.entity) {
        return;
    }
    let Ok(mut handle) = handles.get_mut(event.entity) else {
        return;
    };
    let widget = handle.widget;
    let Some(height) = bodies
        .iter()
        .find(|(body, _)| body.0 == widget)
        .map(|(_, node)| node.size().y * node.inverse_scale_factor)
        .map(|height| height / dock::zoom(layouts.as_deref(), widget))
        .filter(|height| height.is_finite())
    else {
        return;
    };
    handle.gesture = Some((event.pointer.id, height));
    event.propagate(false);
}

pub(super) fn resize(
    mut event: On<PointerDrag>,
    handles: Query<&Resize>,
    scale: Res<UiScale>,
    layouts: Option<Res<WindowLayouts>>,
    mut state: ResMut<SidebarWidgets>,
) {
    let Ok(handle) = handles.get(event.entity) else {
        return;
    };
    let Some((pointer, start)) = handle.gesture else {
        return;
    };
    if pointer != event.pointer.id
        || !event.distance.is_finite()
        || !scale.0.is_finite()
        || scale.0 <= 0.
    {
        return;
    }
    if let Some(entry) = state.entry_mut(handle.widget) {
        let zoom = dock::zoom(layouts.as_deref(), handle.widget);
        entry.height =
            Some((start + event.distance.y / (scale.0 * zoom)).clamp(MIN_CONTENT, MAX_CONTENT));
    }
    event.propagate(false);
}

pub(super) fn resize_end(event: On<PointerDragEnd>, mut handles: Query<&mut Resize>) {
    if let Ok(mut handle) = handles.get_mut(event.entity)
        && handle
            .gesture
            .is_some_and(|(pointer, _)| pointer == event.pointer.id)
    {
        handle.gesture = None;
    }
}

pub(super) fn cancel(
    event: On<PointerCancel>,
    mut handles: Query<&mut Resize>,
    mut bars: Query<&mut TitleBar>,
) {
    for mut handle in &mut handles {
        if handle
            .gesture
            .is_some_and(|(pointer, _)| pointer == event.pointer.id)
        {
            handle.gesture = None;
        }
    }
    for mut bar in &mut bars {
        if bar.pointer == Some(event.pointer.id) {
            bar.pointer = None;
        }
    }
}
