//! Floating windows: a sidebar module, the minimap or the chat leaves its dock for a window
//! of its own on the shared window layer, which moves it, resizes it from every edge and
//! corner, zooms it, stacks it and lets Escape close only the front one.
use super::{BORDER, SidebarWidgets, Widget, WidgetFrame, close, shown};
use crate::bevy_shell::{
    game_ui::{GameRoot, modal::ModalGuard},
    owner::HostOwner,
    ui::{
        Skins,
        window_layout::{WindowLayer, WindowLayouts, WindowStack, window},
    },
    window_drag::DraggableDialog,
};
use bevy::{
    input::keyboard::KeyboardInput, input_focus::InputFocus, prelude::*, ui_widgets::Activate,
};
use oteryn_client_ui_bevy as source;

pub(in crate::bevy_shell) const CHAT: &str = "chat";
pub(in crate::bevy_shell) const MINIMAP: &str = "minimap";
/// The regions that can float, by window id.
pub(super) const REGIONS: [&str; 2] = [CHAT, MINIMAP];

/// A docked region that can float: where it docks back, and a node hidden while it floats.
#[derive(Component, Clone, Copy, Debug)]
pub(in crate::bevy_shell) struct Floatable {
    pub id: &'static str,
    pub home: Entity,
    pub index: usize,
    pub companion: Option<Entity>,
}
/// The window that holds one floating module or region.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::bevy_shell) struct FloatRoot(pub &'static str);
/// A button that floats or docks a region.
#[derive(Component, Clone, Copy, Debug)]
pub(in crate::bevy_shell) struct FloatToggle(pub &'static str);
/// The docked sizing a floating node gives up to fill its window.
#[derive(Component, Clone, Copy)]
pub(super) struct Docked {
    flex_grow: f32,
    flex_shrink: f32,
    max_height: Val,
}

impl SidebarWidgets {
    pub(in crate::bevy_shell) fn region_floats(&self, id: &str) -> bool {
        self.floating.contains(&id)
    }
    /// Floats a docked region or docks a floating one.
    pub(in crate::bevy_shell) fn toggle_region(&mut self, id: &str) {
        if let Some(index) = self.floating.iter().position(|floating| *floating == id) {
            self.floating.remove(index);
        } else if let Some(id) = REGIONS.into_iter().find(|region| *region == id) {
            self.floating.push(id);
        }
    }
}

pub(super) fn toggle(
    event: On<Activate>,
    toggles: Query<&FloatToggle>,
    guard: ModalGuard,
    mut state: ResMut<SidebarWidgets>,
) {
    if let Ok(toggle) = toggles.get(event.entity)
        && guard.allows(event.entity)
    {
        state.toggle_region(toggle.0);
    }
}

fn spawn_root(commands: &mut Commands, skins: &Skins, parent: Entity, id: &'static str) -> Entity {
    commands
        .spawn((
            FloatRoot(id),
            Name::new(format!("window.{id}")),
            WindowLayer(id),
            DraggableDialog,
            source::frame(skins.frame.clone(), [BORDER; 4]),
            window(
                id,
                Node {
                    position_type: PositionType::Absolute,
                    left: px(200.),
                    top: px(80.),
                    padding: UiRect::all(px(BORDER)),
                    flex_direction: FlexDirection::Column,
                    display: Display::None,
                    ..default()
                },
            ),
            ChildOf(parent),
        ))
        .id()
}

/// A window id, its content node, whether it should float, and for a region its home.
type Content = (
    &'static str,
    Entity,
    bool,
    Option<(Floatable, Option<Docked>)>,
);

/// Moves each floating module or region into its window and each docked one back home.
#[allow(clippy::too_many_arguments)]
pub(super) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    state: Res<SidebarWidgets>,
    skins: Option<Res<Skins>>,
    game: Query<Entity, With<GameRoot>>,
    mut roots: Local<Vec<(&'static str, Entity)>>,
    regions: Query<(Entity, &Floatable, Option<&Docked>)>,
    frames: Query<(Entity, &WidgetFrame)>,
    parents: Query<&ChildOf>,
    mut nodes: Query<&mut Node>,
    layouts: Option<ResMut<WindowLayouts>>,
) {
    roots.retain(|(_, root)| nodes.contains(*root));
    let mut moved = false;
    let mut contents: Vec<Content> = Vec::new();
    for (entity, region, docked) in &regions {
        let wanted = state.region_floats(region.id);
        contents.push((region.id, entity, wanted, Some((*region, docked.copied()))));
    }
    for (entity, frame) in &frames {
        let wanted = state.entry(frame.0).is_some_and(|entry| entry.floating)
            && shown(&owner, &state, frame.0);
        contents.push((frame.0.window_id(), entity, wanted, None));
    }
    for (id, content, wanted, region) in contents {
        let root = roots
            .iter()
            .find(|(root_id, _)| *root_id == id)
            .map(|(_, root)| *root);
        let root = match (root, wanted, skins.as_deref(), game.iter().next()) {
            (Some(root), _, _, _) => root,
            (None, true, Some(skins), Some(parent)) => {
                let root = spawn_root(&mut commands, skins, parent, id);
                roots.push((id, root));
                continue;
            }
            _ => continue,
        };
        let parent = parents.get(content).ok().map(ChildOf::parent);
        let visible = wanted
            && nodes
                .get(content)
                .is_ok_and(|node| node.display != Display::None);
        if let Ok(mut node) = nodes.get_mut(root) {
            let display = if visible {
                Display::Flex
            } else {
                Display::None
            };
            if node.display != display {
                node.display = display;
            }
        }
        if wanted && parent != Some(root) {
            moved = true;
            commands.entity(content).insert(ChildOf(root));
            if let Some((region, None)) = region
                && let Ok(mut node) = nodes.get_mut(content)
            {
                commands.entity(content).insert(Docked {
                    flex_grow: node.flex_grow,
                    flex_shrink: node.flex_shrink,
                    max_height: node.max_height,
                });
                node.flex_grow = 1.;
                node.flex_shrink = 1.;
                node.max_height = Val::Auto;
                if let Some(companion) = region.companion
                    && let Ok(mut node) = nodes.get_mut(companion)
                {
                    node.display = Display::None;
                }
            }
        } else if !wanted && parent == Some(root) {
            moved = true;
            // A module docks back through its pane; a region returns to its place.
            let Some((region, docked)) = region else {
                continue;
            };
            commands
                .entity(region.home)
                .insert_children(region.index, &[content]);
            if let Some(docked) = docked
                && let Ok(mut node) = nodes.get_mut(content)
            {
                node.flex_grow = docked.flex_grow;
                node.flex_shrink = docked.flex_shrink;
                node.max_height = docked.max_height;
                commands.entity(content).remove::<Docked>();
            }
            if let Some(companion) = region.companion
                && let Ok(mut node) = nodes.get_mut(companion)
            {
                node.display = Display::Flex;
            }
        }
    }
    // Zoomed nodes and text re-read their window's zoom after a move.
    if moved && let Some(mut layouts) = layouts {
        layouts.set_changed();
    }
}

/// Escape closes only the front window, and only when it is a floating one: a module closes,
/// a region docks back.
#[allow(clippy::too_many_arguments)]
pub(super) fn escape(
    mut events: MessageReader<KeyboardInput>,
    stack: Option<Res<WindowStack>>,
    roots: Query<&FloatRoot>,
    guard: ModalGuard,
    focus: Option<Res<InputFocus>>,
    editors: Query<(), With<bevy::text::EditableText>>,
    mut owner: NonSendMut<HostOwner>,
    mut state: ResMut<SidebarWidgets>,
) {
    for event in events.read() {
        if event.key_code != KeyCode::Escape
            || event.state != bevy::input::ButtonState::Pressed
            || event.repeat
            || focus
                .as_deref()
                .and_then(InputFocus::get)
                .is_some_and(|entity| editors.contains(entity))
        {
            continue;
        }
        let Some(front) = stack.as_deref().and_then(WindowStack::front) else {
            continue;
        };
        let Ok(root) = roots.get(front) else {
            continue;
        };
        if !guard.allows(front) {
            continue;
        }
        match Widget::ALL
            .into_iter()
            .find(|widget| widget.window_id() == root.0)
        {
            Some(widget) => close(&mut owner, &mut state, widget),
            None => state.toggle_region(root.0),
        }
    }
}
