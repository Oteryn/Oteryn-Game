//! TibiaDialog's 17px caption drag, over upstream picking events and transient UI offsets.
use super::{
    game_ui::{ItemDragSource, modal::ModalGuard},
    owner::HostOwner,
};
use bevy::{
    camera::NormalizedRenderTarget,
    ecs::system::SystemParam,
    picking::{
        events::*,
        pointer::{PointerButton, PointerId},
    },
    prelude::*,
    ui_widgets::{Button, Checkbox, Slider, TextInput},
    window::{PrimaryWindow, WindowFocused, WindowRef},
};

/// Tag only an ordinary dialog panel, never its scrim, context menu or body controls.
#[derive(Component, Clone, Copy, Default)]
#[require(Node, UiTransform, DragState)]
pub(in crate::bevy_shell) struct DraggableDialog;
#[derive(Component, Default)]
struct DragState {
    context: Option<Context>,
    gesture: Option<Gesture>,
}
#[derive(Clone, Copy, PartialEq)]
struct Context {
    window: Entity,
    size: UVec2,
    dpi: f32,
    scale: f32,
    generation: Option<u64>,
}
impl Context {
    fn factor(self) -> f32 {
        self.dpi * self.scale
    }
    fn viewport(self) -> Vec2 {
        self.size.as_vec2() / self.factor()
    }
    fn target(self) -> Option<NormalizedRenderTarget> {
        WindowRef::Entity(self.window)
            .normalize(None)
            .map(NormalizedRenderTarget::Window)
    }
}
#[derive(Clone, Copy)]
struct Gesture {
    origin: Entity,
    pointer: PointerId,
    button: PointerButton,
    context: Context,
    start: Vec2,
    base: Vec2,
    size: Vec2,
    started: bool,
}
#[derive(SystemParam)]
struct Environment<'w, 's> {
    windows: Query<'w, 's, (Entity, &'static Window), With<PrimaryWindow>>,
    scale: Res<'w, UiScale>,
    owner: Option<NonSend<'w, HostOwner>>,
    guard: ModalGuard<'w, 's>,
}
impl Environment<'_, '_> {
    fn current(&self) -> Option<(Context, bool)> {
        let (entity, window) = self.windows.single().ok()?;
        let context = Context {
            window: entity,
            size: UVec2::new(window.physical_width(), window.physical_height()),
            dpi: window.scale_factor(),
            scale: self.scale.0,
            generation: self.owner.as_ref().map(|owner| owner.generation()),
        };
        (context.factor().is_finite() && context.factor() > 0. && context.size.min_element() > 0)
            .then_some((context, window.focused))
    }
    fn permits(&self, panel: Entity, pointer: &Pointer, context: Context) -> bool {
        self.current()
            .is_some_and(|(current, focused)| focused && current == context)
            && context.target().as_ref() == Some(&pointer.target)
            && pointer.position.is_finite()
            && self.guard.allows(panel)
    }
}
// An origin control must not turn into a caption drag as its event bubbles to a dialog.
#[allow(clippy::type_complexity)]
type Tree<'w, 's> = Query<
    'w,
    's,
    (
        Option<&'static ChildOf>,
        Has<DraggableDialog>,
        Has<Button>,
        Has<TextInput>,
        Has<Checkbox>,
        Has<Slider>,
        Has<ItemDragSource>,
    ),
>;
// The observer updates a panel projection and its private transient gesture together.
#[allow(clippy::type_complexity)]
type Panels<'w, 's> = Query<
    'w,
    's,
    (
        &'static ComputedNode,
        &'static UiGlobalTransform,
        &'static mut UiTransform,
        &'static mut DragState,
    ),
    With<DraggableDialog>,
>;
fn panel(origin: Entity, tree: &Tree) -> Option<Entity> {
    let mut next = Some(origin);
    while let Some(entity) = next {
        let (parent, dialog, button, text, checkbox, slider, item) = tree.get(entity).ok()?;
        if button || text || checkbox || slider || item {
            return None;
        }
        if dialog {
            return Some(entity);
        }
        next = parent.map(ChildOf::parent);
    }
    None
}
fn offset(transform: &UiTransform) -> Option<Vec2> {
    let value = |value| match value {
        Val::Px(value) => Some(value),
        Val::Auto => Some(0.),
        _ => None,
    };
    let offset = Vec2::new(
        value(transform.translation.x)?,
        value(transform.translation.y)?,
    );
    offset.is_finite().then_some(offset)
}
fn geometry(
    node: &ComputedNode,
    global: &UiGlobalTransform,
    context: Context,
) -> Option<(Vec2, Vec2)> {
    // Global UI scale/DPI live in the computed geometry, not this local affine matrix.
    if (node.inverse_scale_factor() - context.factor().recip()).abs() > 0.00001
        || global.matrix2 != Mat2::IDENTITY
        || !node.size().is_finite()
        || node.size().min_element() <= 0.
    {
        return None;
    }
    let size = node.size() / context.factor();
    let position = (global.translation - node.size() * 0.5) / context.factor();
    (size.is_finite() && position.is_finite()).then_some((position, size))
}
fn bounded(base: Vec2, size: Vec2, viewport: Vec2, requested: Vec2) -> Vec2 {
    (base + requested).clamp(Vec2::ZERO, (viewport - size).max(Vec2::ZERO)) - base
}
pub(in crate::bevy_shell) fn install(app: &mut App) {
    app.init_resource::<UiScale>()
        .add_message::<WindowFocused>()
        .add_observer(press)
        .add_observer(start)
        .add_observer(drag)
        .add_observer(release)
        .add_observer(end)
        .add_observer(cancel)
        .add_systems(Update, maintain);
}
fn press(mut event: On<PointerPress>, tree: Tree, mut panels: Panels, environment: Environment) {
    if event.entity != event.original_event_target()
        || !matches!(
            event.button,
            PointerButton::Primary | PointerButton::Secondary
        )
    {
        return;
    }
    let Some(entity) = panel(event.original_event_target(), &tree) else {
        return;
    };
    let Some((context, _)) = environment.current() else {
        return;
    };
    if !environment.permits(entity, &event.pointer, context) {
        return;
    }
    let Ok((node, global, transform, mut state)) = panels.get_mut(entity) else {
        return;
    };
    if state.gesture.is_some() {
        return;
    }
    let Some((position, size)) = geometry(node, global, context) else {
        return;
    };
    let point = event.pointer.position / context.scale - position;
    if !(0.0..size.x).contains(&point.x) || !(0.0..17.0).contains(&point.y) {
        return;
    }
    let Some(start) = offset(&transform) else {
        return;
    };
    state.context = Some(context);
    state.gesture = Some(Gesture {
        origin: event.original_event_target(),
        pointer: event.pointer.id,
        button: event.button,
        context,
        start,
        base: position - start,
        size,
        started: false,
    });
    event.propagate(false);
}
fn start(
    mut event: On<PointerDragStart>,
    tree: Tree,
    mut panels: Panels,
    environment: Environment,
) {
    if event.entity != event.original_event_target() {
        return;
    }
    let Some(entity) = panel(event.original_event_target(), &tree) else {
        return;
    };
    let Ok((_, _, _, mut state)) = panels.get_mut(entity) else {
        return;
    };
    let Some(gesture) = &mut state.gesture else {
        return;
    };
    if gesture.origin != event.original_event_target()
        || gesture.pointer != event.pointer.id
        || gesture.button != event.button
    {
        return;
    }
    if !environment.permits(entity, &event.pointer, gesture.context) {
        state.gesture = None;
        return;
    }
    gesture.started = true;
    event.propagate(false);
}
fn drag(mut event: On<PointerDrag>, tree: Tree, mut panels: Panels, environment: Environment) {
    if event.entity != event.original_event_target() {
        return;
    }
    let Some(entity) = panel(event.original_event_target(), &tree) else {
        return;
    };
    let Ok((node, _, mut transform, mut state)) = panels.get_mut(entity) else {
        return;
    };
    let Some(gesture) = state.gesture else {
        return;
    };
    if gesture.origin != event.original_event_target()
        || gesture.pointer != event.pointer.id
        || gesture.button != event.button
        || !gesture.started
    {
        return;
    }
    if !environment.permits(entity, &event.pointer, gesture.context)
        || !event.distance.is_finite()
        || node.size() / gesture.context.factor() != gesture.size
    {
        state.gesture = None;
        return;
    }
    let next = bounded(
        gesture.base,
        gesture.size,
        gesture.context.viewport(),
        gesture.start + event.distance / gesture.context.scale,
    );
    transform.translation = Val2::px(next.x, next.y);
    event.propagate(false);
}
fn release(event: On<PointerRelease>, mut panels: Panels) {
    if event.entity != event.original_event_target() {
        return;
    }
    clear(&mut panels, event.pointer.id, Some(event.button));
}
fn end(event: On<PointerDragEnd>, mut panels: Panels) {
    if event.entity != event.original_event_target() {
        return;
    }
    clear(&mut panels, event.pointer.id, Some(event.button));
}
fn cancel(event: On<PointerCancel>, mut panels: Panels) {
    if event.entity != event.original_event_target() {
        return;
    }
    clear(&mut panels, event.pointer.id, None);
}
fn clear(panels: &mut Panels, pointer: PointerId, button: Option<PointerButton>) {
    for (_, _, _, mut state) in panels {
        if state.gesture.is_some_and(|gesture| {
            gesture.pointer == pointer && button.is_none_or(|button| button == gesture.button)
        }) {
            state.gesture = None;
        }
    }
}
// Independent panel geometry and transient offsets are injected by Bevy ECS.
#[allow(clippy::type_complexity)]
fn maintain(
    mut panels: Query<
        (
            Entity,
            &ComputedNode,
            &UiGlobalTransform,
            &mut UiTransform,
            &mut DragState,
        ),
        With<DraggableDialog>,
    >,
    environment: Environment,
    mut focus: MessageReader<WindowFocused>,
) {
    let context = environment.current();
    // Drain the whole frame so a queued loss cannot cancel a fresh press next frame.
    let mut lost_focus = false;
    for event in focus.read() {
        lost_focus |=
            !event.focused && context.is_some_and(|(context, _)| context.window == event.window);
    }
    for (entity, node, global, mut transform, mut state) in &mut panels {
        let Some((context, focused)) = context else {
            state.gesture = None;
            continue;
        };
        if state.context.is_some_and(|previous| previous != context) {
            transform.translation = Val2::ZERO;
            state.gesture = None;
            state.context = Some(context);
            continue;
        }
        state.context = Some(context);
        if lost_focus || !focused || !environment.guard.allows(entity) {
            state.gesture = None;
        }
        let (Some(current), Some((position, size))) =
            (offset(&transform), geometry(node, global, context))
        else {
            continue;
        };
        let next = bounded(position - current, size, context.viewport(), current);
        if current != next {
            transform.translation = Val2::px(next.x, next.y);
        }
    }
}
#[cfg(test)]
mod tests;
