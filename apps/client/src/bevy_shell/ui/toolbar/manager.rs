//! Immediate, persisted button-strip editing. The manager itself cannot be hidden.
use super::*;
use crate::bevy_shell::{
    game_ui::{GameModal, modal::ModalGuard},
    ui::Skins,
};
use bevy::{
    input_focus::tab_navigation::TabGroup,
    ui_widgets::{Activate, ScrollArea},
};
use oteryn_client_ui_bevy as source;

#[derive(Resource, Default)]
pub(in crate::bevy_shell) struct Manager {
    pub open: bool,
    root: Option<Entity>,
    snapshot: Option<(u64, Option<Vec<String>>)>,
}
#[derive(Component, Clone, Copy)]
enum Edit {
    Toggle(&'static str),
    Move(&'static str, bool),
    Reset,
    Close,
}

fn order(owner: &HostOwner) -> Vec<String> {
    owner
        .settings
        .preferences
        .toolbar_order
        .clone()
        .unwrap_or_else(|| {
            TOOLBAR
                .iter()
                .map(|(_, control)| control.id().to_owned())
                .collect()
        })
}
fn edit(
    event: On<Activate>,
    actions: Query<&Edit>,
    guard: ModalGuard,
    mut owner: NonSendMut<HostOwner>,
    mut state: ResMut<Manager>,
) {
    if !state.open
        || !guard.allows(event.entity)
        || owner.play.is_none()
        || state
            .snapshot
            .as_ref()
            .is_none_or(|snapshot| snapshot.0 != owner.generation())
    {
        return;
    }
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    if matches!(action, Edit::Close) {
        state.open = false;
        return;
    }
    let mut ids = order(&owner);
    match *action {
        Edit::Toggle(id) => {
            if let Some(index) = ids.iter().position(|entry| entry == id) {
                ids.remove(index);
            } else {
                ids.push(id.into());
            }
        }
        Edit::Move(id, up) => {
            let Some(index) = ids.iter().position(|entry| entry == id) else {
                return;
            };
            let next = if up {
                index.checked_sub(1)
            } else {
                index.checked_add(1).filter(|next| *next < ids.len())
            };
            let Some(next) = next else { return };
            ids.swap(index, next);
        }
        Edit::Reset | Edit::Close => {}
    }
    let mut preferences = owner.settings.preferences.clone();
    preferences.toolbar_order = if matches!(action, Edit::Reset) {
        None
    } else {
        Some(ids)
    };
    owner.settings.apply(preferences);
}

pub(in crate::bevy_shell) fn project_manager(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    keys: Res<ButtonInput<KeyCode>>,
    guard: ModalGuard,
    mut state: ResMut<Manager>,
    mut focus: ResMut<bevy::input_focus::InputFocus>,
) {
    if (keys.just_pressed(KeyCode::Escape) && state.root.is_some_and(|root| guard.allows(root)))
        || owner.play.is_none()
        || state
            .snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.0 != owner.generation())
    {
        state.open = false;
    }
    let snapshot = (
        owner.generation(),
        owner.settings.preferences.toolbar_order.clone(),
    );
    if state.open && state.snapshot.as_ref() == Some(&snapshot) {
        return;
    }
    if let Some(root) = state.root.take() {
        focus.clear();
        commands.entity(root).despawn();
    }
    state.snapshot = None;
    if !state.open {
        return;
    }
    let root = commands
        .spawn((
            GameModal,
            GlobalZIndex(110),
            TabGroup {
                order: 2,
                modal: true,
            },
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.65)),
        ))
        .id();
    let panel = commands
        .spawn((
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(560.),
                max_width: percent(98.),
                height: percent(85.),
                padding: UiRect::all(px(16.)),
                flex_direction: FlexDirection::Column,
                row_gap: px(6.),
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(panel);
    let title = commands
        .spawn(source::text(
            "Manage Buttons — changes are saved immediately",
            source::font(skins.font.clone(), false),
        ))
        .id();
    commands.entity(panel).add_child(title);
    let list = commands
        .spawn((
            ScrollArea,
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.,
                min_height: px(0.),
                overflow: Overflow::scroll_y(),
                row_gap: px(4.),
                ..default()
            },
        ))
        .id();
    commands.entity(panel).add_child(list);
    let ids = order(&owner);
    let mut entries: Vec<_> = TOOLBAR.iter().collect();
    entries.sort_by_key(|(_, control)| {
        ids.iter()
            .position(|id| id == control.id())
            .unwrap_or(usize::MAX)
    });
    for (caption, control) in entries {
        let id = control.id();
        let visible = ids.iter().any(|entry| entry == id);
        let row = commands
            .spawn(Node {
                column_gap: px(5.),
                flex_shrink: 0.,
                ..default()
            })
            .id();
        commands.entity(list).add_child(row);
        button(
            &mut commands,
            &skins,
            row,
            Edit::Toggle(id),
            &format!("{} {caption}", if visible { "[x]" } else { "[ ]" }),
            290.,
        );
        if visible {
            button(&mut commands, &skins, row, Edit::Move(id, true), "Up", 65.);
            button(
                &mut commands,
                &skins,
                row,
                Edit::Move(id, false),
                "Down",
                65.,
            );
        }
    }
    button(
        &mut commands,
        &skins,
        panel,
        Edit::Reset,
        "Restore defaults",
        150.,
    );
    button(&mut commands, &skins, panel, Edit::Close, "Close", 90.);
    state.root = Some(root);
    state.snapshot = Some(snapshot);
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    action: Edit,
    caption: &str,
    width: f32,
) {
    let button = commands
        .spawn((source::button(skins.buttons.clone(), width), action))
        .observe(edit)
        .id();
    let text = commands
        .spawn(source::button_caption(caption, skins.font.clone()))
        .id();
    commands.entity(button).add_child(text);
    commands.entity(parent).add_child(button);
}

#[cfg(test)]
#[path = "manager_tests.rs"]
mod tests;
