//! The window menu: right-click a window's frame for Zoom and Reset window layout.
use super::{WindowId, WindowLayouts, stack::BAND, viewport};
use crate::bevy_shell::{owner::HostOwner, ui::Skins};
use bevy::{
    picking::{
        events::{PointerClick, PointerPress},
        pointer::PointerButton,
    },
    prelude::*,
    ui_widgets::{Activate, Button, Checkbox, TextInput},
    window::PrimaryWindow,
};
use oteryn_client_ui_bevy as source;

pub(super) const ZOOMS: [(&str, f32); 5] = [
    ("Zoom 75%", 0.75),
    ("Zoom 100%", 1.),
    ("Zoom 125%", 1.25),
    ("Zoom 150%", 1.5),
    ("Zoom 200%", 2.),
];
pub(super) const RESET: &str = "Reset window layout";

#[derive(Component)]
pub(in crate::bevy_shell) struct WindowMenu;

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub(super) enum Entry {
    Zoom(&'static str, f32),
    Reset(&'static str),
}

/// A press on one of these acts on the control, not on the window behind it.
type Controls<'w, 's> = Query<'w, 's, (), Or<(With<Button>, With<TextInput>, With<Checkbox>)>>;

/// Right-click on a window outside its controls opens the menu at the pointer.
#[allow(clippy::too_many_arguments)]
pub(in crate::bevy_shell) fn open(
    event: On<PointerClick>,
    mut commands: Commands,
    parents: Query<&ChildOf>,
    ids: Query<&WindowId>,
    controls: Controls,
    menus: Query<Entity, With<WindowMenu>>,
    skins: Option<Res<Skins>>,
    scale: Option<Res<UiScale>>,
) {
    if event.entity != event.original_event_target() || event.button != PointerButton::Secondary {
        return;
    }
    let Some(skins) = skins else {
        return;
    };
    let mut window = None;
    for entity in std::iter::once(event.entity).chain(parents.iter_ancestors(event.entity)) {
        if controls.contains(entity) {
            return;
        }
        if let Ok(id) = ids.get(entity) {
            window = Some(id.0);
            break;
        }
    }
    let Some(window) = window else {
        return;
    };
    for menu in &menus {
        commands.entity(menu).despawn();
    }
    let at = event.pointer.position / scale.map_or(1., |scale| scale.0);
    let menu = commands
        .spawn((
            WindowMenu,
            Name::new("window.menu"),
            GlobalZIndex(BAND.1 + 1),
            source::frame(skins.frame.clone(), [4.; 4]),
            Node {
                position_type: PositionType::Absolute,
                left: px(at.x),
                top: px(at.y),
                padding: px(4.).all(),
                flex_direction: FlexDirection::Column,
                row_gap: px(2.),
                ..default()
            },
        ))
        .id();
    let entries = ZOOMS
        .map(|(label, zoom)| Entry::Zoom(label, zoom))
        .into_iter()
        .chain([Entry::Reset(RESET)]);
    for entry in entries {
        let label = match entry {
            Entry::Zoom(label, _) | Entry::Reset(label) => label,
        };
        let button = commands
            .spawn((
                source::button(skins.buttons.clone(), 120.),
                entry,
                WindowId(window),
                ChildOf(menu),
            ))
            .observe(choose)
            .id();
        commands.spawn((
            source::button_caption(label, skins.font.clone()),
            ChildOf(button),
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn choose(
    event: On<Activate>,
    mut commands: Commands,
    entries: Query<(&Entry, &WindowId)>,
    menus: Query<Entity, With<WindowMenu>>,
    screens: Query<&Window, With<PrimaryWindow>>,
    scale: Option<Res<UiScale>>,
    owner: Option<NonSend<HostOwner>>,
    mut layouts: ResMut<WindowLayouts>,
) {
    let Ok((entry, id)) = entries.get(event.entity) else {
        return;
    };
    match *entry {
        Entry::Zoom(_, zoom) => {
            let viewport = viewport(&screens, scale.as_deref()).unwrap_or(Vec2::splat(f32::MAX));
            if let Some(layout) = layouts.0.get_mut(id.0) {
                layout.set_zoom(zoom, viewport);
            }
        }
        Entry::Reset(_) => {
            let path = owner.and_then(|owner| owner.window_layouts_path());
            if let Err(error) = layouts.reset(path.as_deref()) {
                leprintln!("Could not save window layout: {error}", error = error);
            }
            commands.trigger(super::LayoutReset);
        }
    }
    for menu in &menus {
        commands.entity(menu).despawn();
    }
}

/// A press outside the menu closes it.
pub(in crate::bevy_shell) fn dismiss(
    event: On<PointerPress>,
    mut commands: Commands,
    parents: Query<&ChildOf>,
    menus: Query<Entity, With<WindowMenu>>,
) {
    if event.entity != event.original_event_target() || menus.is_empty() {
        return;
    }
    let inside = std::iter::once(event.entity)
        .chain(parents.iter_ancestors(event.entity))
        .any(|entity| menus.contains(entity));
    if !inside {
        for menu in &menus {
            commands.entity(menu).despawn();
        }
    }
}
