//! Ordinary windows stack by focus inside one z band, below dialogs that stay modal.
//! The frontmost window is the one the modal fence lets act, so Escape closes it first.
use super::WindowId;
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

/// The stacking node of an ordinary window, named by its `WindowLayouts` id: either a
/// full-screen overlay holding the window as its first child, or the window itself.
#[derive(Component, Clone, Copy, Debug)]
#[require(GlobalZIndex)]
pub(in crate::bevy_shell) struct WindowLayer(pub &'static str);

/// A window overlay lets the pointer through to the windows and the world behind it.
const THROUGH: Pickable = Pickable {
    should_block_lower: false,
    is_hoverable: false,
};

/// Open windows from back to front, by id, so a window rebuilt on refresh keeps its place.
#[derive(Resource, Default, Debug)]
pub(in crate::bevy_shell) struct WindowStack(Vec<(&'static str, Entity)>);

impl WindowStack {
    pub(in crate::bevy_shell) fn front(&self) -> Option<Entity> {
        self.0.last().map(|(_, entity)| *entity)
    }
    fn raise(&mut self, layer: Entity) {
        if self.front() != Some(layer)
            && let Some(index) = self.0.iter().position(|(_, entity)| *entity == layer)
        {
            let window = self.0.remove(index);
            self.0.push(window);
        }
    }
}

fn overlay(node: &Node) -> bool {
    node.width == percent(100.) && node.height == percent(100.)
}

type Layers<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static WindowLayer,
        &'static Node,
        Option<&'static Children>,
        Option<&'static Pickable>,
    ),
>;

/// Gives each window layer its layout id and grips: an overlay's first child is the window
/// and the overlay itself lets the pointer through; otherwise the layer is the window.
pub(in crate::bevy_shell) fn adopt(
    mut commands: Commands,
    layers: Layers,
    panels: Query<Has<WindowId>, With<Node>>,
) {
    for (entity, layer, node, children, pickable) in &layers {
        let window = if overlay(node) {
            if pickable != Some(&THROUGH) {
                commands.entity(entity).insert(THROUGH);
            }
            children.and_then(|children| {
                if children.iter().any(|child| panels.get(child) == Ok(true)) {
                    return None;
                }
                children.iter().find(|child| panels.contains(*child))
            })
        } else {
            Some(entity)
        };
        if let Some(window) = window
            && panels.get(window) == Ok(false)
        {
            commands.entity(window).insert(WindowId(layer.0));
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
    windows: Query<(Entity, &WindowLayer)>,
    layers: Query<(), With<WindowLayer>>,
    tree: Tree,
    parents: Query<&ChildOf>,
    focus: Option<Res<InputFocus>>,
    mut depths: Query<&mut GlobalZIndex, With<WindowLayer>>,
) {
    let open: Vec<(&'static str, Entity)> = windows
        .iter()
        .filter(|(window, _)| shown(*window, &tree))
        .map(|(window, layer)| (layer.0, window))
        .collect();
    for (id, entity) in &mut stack.0 {
        if let Some((_, rebuilt)) = open.iter().find(|(open, _)| open == id) {
            *entity = *rebuilt;
        }
    }
    stack.0.retain(|window| open.contains(window));
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
    for (index, (_, window)) in stack.0.iter().enumerate() {
        let depth = GlobalZIndex((BAND.0 + index as i32).min(BAND.1));
        if let Ok(mut current) = depths.get_mut(*window)
            && *current != depth
        {
            *current = depth;
        }
    }
}
