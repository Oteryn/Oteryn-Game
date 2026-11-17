//! Ordinary windows stack by focus inside one z band, below dialogs that stay modal.
//! The frontmost window is the one the modal fence lets act, so Escape closes it first.
use crate::bevy_shell::game_ui::GameModal;
use bevy::{
    ecs::system::SystemParam,
    input_focus::InputFocus,
    picking::{Pickable, events::PointerPress},
    prelude::*,
};

/// GlobalZIndex of the back and front window: above the HUD and the options dialog (at most
/// 100), below the dialogs a window opens (150 and up).
pub(in crate::bevy_shell) const BAND: (i32, i32) = (110, 139);

/// The stacking node of an ordinary window: a full-screen overlay that lets the pointer
/// through to the windows behind it.
#[derive(Component, Clone, Copy, Debug, Default)]
#[require(Pickable = THROUGH, GlobalZIndex)]
pub(in crate::bevy_shell) struct WindowLayer;

const THROUGH: Pickable = Pickable {
    should_block_lower: false,
    is_hoverable: false,
};

/// Open windows from back to front.
#[derive(Resource, Default, Debug)]
pub(in crate::bevy_shell) struct WindowStack(Vec<Entity>);

impl WindowStack {
    pub(in crate::bevy_shell) fn front(&self) -> Option<Entity> {
        self.0.last().copied()
    }
    fn raise(&mut self, layer: Entity) {
        if self.front() != Some(layer) {
            self.0.retain(|entity| *entity != layer);
            self.0.push(layer);
        }
    }
}

type Tree<'w, 's> = Query<
    'w,
    's,
    (
        Option<&'static Node>,
        Option<&'static Visibility>,
        Option<&'static ChildOf>,
    ),
>;

fn shown(mut entity: Entity, tree: &Tree) -> bool {
    loop {
        let Ok((node, visibility, parent)) = tree.get(entity) else {
            return false;
        };
        if node.is_some_and(|node| node.display == Display::None)
            || visibility == Some(&Visibility::Hidden)
        {
            return false;
        }
        match parent {
            Some(parent) => entity = parent.parent(),
            None => return true,
        }
    }
}

fn layer_of(
    entity: Entity,
    parents: &Query<&ChildOf>,
    layers: &Query<(), With<WindowLayer>>,
) -> Option<Entity> {
    std::iter::once(entity)
        .chain(parents.iter_ancestors(entity))
        .find(|candidate| layers.contains(*candidate))
}

/// Whether every visible modal is an ordinary window, so the toolbar can open another.
#[derive(SystemParam)]
pub(in crate::bevy_shell) struct OpenWindows<'w, 's> {
    modals: Query<'w, 's, Entity, (With<GameModal>, Without<WindowLayer>)>,
    tree: Tree<'w, 's>,
}

impl OpenWindows<'_, '_> {
    pub(in crate::bevy_shell) fn only_windows(&self) -> bool {
        !self.modals.iter().any(|modal| shown(modal, &self.tree))
    }
}

/// A press anywhere in a window brings it to the front.
pub(in crate::bevy_shell) fn raise_on_press(
    event: On<PointerPress>,
    parents: Query<&ChildOf>,
    layers: Query<(), With<WindowLayer>>,
    mut stack: ResMut<WindowStack>,
) {
    if event.entity == event.original_event_target()
        && let Some(layer) = layer_of(event.entity, &parents, &layers)
    {
        stack.raise(layer);
    }
}

/// Opening or focusing a window raises it; the stack order becomes GlobalZIndex before
/// layout, so the modal fence of the same frame already sees the new front window.
pub(in crate::bevy_shell) fn project(
    mut stack: ResMut<WindowStack>,
    windows: Query<Entity, With<WindowLayer>>,
    layers: Query<(), With<WindowLayer>>,
    tree: Tree,
    parents: Query<&ChildOf>,
    focus: Option<Res<InputFocus>>,
    mut depths: Query<&mut GlobalZIndex, With<WindowLayer>>,
) {
    let open: Vec<Entity> = windows
        .iter()
        .filter(|window| shown(*window, &tree))
        .collect();
    if stack.0.iter().any(|window| !open.contains(window)) {
        stack.0.retain(|window| open.contains(window));
    }
    for window in open {
        if !stack.0.contains(&window) {
            stack.0.push(window);
        }
    }
    if let Some(focus) = focus
        && focus.is_changed()
        && let Some(layer) = focus
            .get()
            .and_then(|entity| layer_of(entity, &parents, &layers))
    {
        stack.raise(layer);
    }
    for (index, window) in stack.0.iter().enumerate() {
        let depth = GlobalZIndex((BAND.0 + index as i32).min(BAND.1));
        if let Ok(mut current) = depths.get_mut(*window)
            && *current != depth
        {
            *current = depth;
        }
    }
}
