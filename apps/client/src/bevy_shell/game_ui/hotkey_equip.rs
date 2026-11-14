//! Standalone presentation over the item-hotkey owner and its saved preferences.
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use crate::play::feature_0179_hotkey_equip::view::HotkeyEquipControl as Kind;
use crate::play::feature_0179_hotkey_equip::view::HotkeyEquipSnapshot as Panel;
use bevy::{
    prelude::*,
    text::{EditableText, TextCursorStyle},
    ui_widgets::{Activate, TextInput},
};
use oteryn_client_ui_bevy as source;

#[derive(Resource, Default)]
struct State {
    snapshot: Option<(u64, Panel)>,
    root: Option<Entity>,
    slot: usize,
    confirm: Option<usize>,
    redraw: bool,
}
#[derive(Component, Clone)]
struct Control {
    generation: u64,
    expected: Panel,
    action: Action,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Action {
    Select(usize),
    Owner(Kind),
    Definition,
}
#[derive(Component)]
struct Definition;
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain());
}
pub(in crate::bevy_shell) fn open(owner: &mut HostOwner) {
    if !owner.modal_active()
        && let Some(play) = &owner.play
    {
        play.link.hotkey_equip_open_bound();
    }
}
fn label(commands: &mut Commands, skins: &Skins, root: Entity, text: &str) {
    let child = commands
        .spawn(source::text(text, source::font(skins.font.clone(), false)))
        .id();
    commands.entity(root).add_child(child);
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    root: Entity,
    generation: u64,
    panel: &Panel,
    action: Action,
    caption: &str,
) {
    let child = commands
        .spawn((
            Control {
                generation,
                expected: panel.clone(),
                action,
            },
            source::button(skins.buttons.clone(), 300.),
        ))
        .observe(activate)
        .id();
    label(commands, skins, child, caption);
    commands.entity(root).add_child(child);
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
) {
    let current = owner
        .play
        .as_ref()
        .map(|p| (owner.generation(), p.link.hotkey_equip_snapshot()))
        .filter(|(_, p)| p.open);
    if state.snapshot == current && !state.redraw {
        return;
    }
    if state.snapshot != current {
        state.confirm = None;
    }
    state.redraw = false;
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
    }
    state.snapshot = current.clone();
    let Some((generation, panel)) = current else {
        return;
    };
    let root = commands
        .spawn((
            GameModal,
            bevy::ui_widgets::ScrollArea,
            bevy::input_focus::tab_navigation::TabGroup::new(15),
            Name::new("hotkey_equip.window"),
            GlobalZIndex(120),
            source::frame(skins.frame.clone(), [2., 17., 2., 2.]),
            Node {
                position_type: PositionType::Absolute,
                left: px(200.),
                top: px(40.),
                width: px(520.),
                max_height: percent(90.),
                overflow: Overflow::scroll_y(),
                padding: px(20.).all(),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.),
                ..default()
            },
        ))
        .id();
    state.root = Some(root);
    source::dialog_caption(
        &mut commands,
        root,
        "Item hotkeys — Ctrl+Shift+E",
        skins.font.clone(),
    );
    label(
        &mut commands,
        &skins,
        root,
        &oteryn_client_ui::hotkey_equip::panel_text(&panel),
    );
    label(
        &mut commands,
        &skins,
        root,
        if panel.smart_active {
            "Session source selection: smart"
        } else {
            "Session source selection: direct inventory"
        },
    );
    label(
        &mut commands,
        &skins,
        root,
        "Smart equip searches carried containers and prefers a started copy. Mode changes apply on your next login.",
    );
    for (slot, binding) in panel.bindings.iter().enumerate() {
        let caption = format!(
            "{}F{}: {}",
            if state.slot == slot { "> " } else { "" },
            slot + 1,
            binding.map_or_else(|| "Unassigned".to_string(), |d| format!("Item {d}"))
        );
        button(
            &mut commands,
            &skins,
            root,
            generation,
            &panel,
            Action::Select(slot),
            &caption,
        );
    }
    label(
        &mut commands,
        &skins,
        root,
        "Choose a carried or equipped item to assign:",
    );
    for &definition in &panel.items {
        button(
            &mut commands,
            &skins,
            root,
            generation,
            &panel,
            Action::Owner(Kind::Assign(state.slot, definition)),
            &format!("Assign item {definition} to F{}", state.slot + 1),
        );
    }
    if let Some(slot) = state.confirm {
        label(
            &mut commands,
            &skins,
            root,
            &format!(
                "Clear F{} binding {}? Press Clear key again to confirm.",
                slot + 1,
                panel.bindings[slot]
                    .map_or_else(|| "Unassigned".to_string(), |d| format!("Item {d}"))
            ),
        );
    }
    for (kind, caption) in [
        (Kind::Clear(state.slot, false), "Clear key"),
        (Kind::Use(state.slot), "Use key"),
        (
            Kind::Smart(!panel.smart),
            if panel.smart {
                "Smart equip: on (next login)"
            } else {
                "Smart equip: off (next login)"
            },
        ),
        (Kind::Close, "Close"),
    ] {
        button(
            &mut commands,
            &skins,
            root,
            generation,
            &panel,
            Action::Owner(kind),
            caption,
        );
    }
    label(&mut commands, &skins, root, "Item definition:");
    let mut editable = EditableText::new(panel.item_definition.to_string());
    editable.max_characters = Some(10);
    editable.allow_newlines = false;
    let field = commands
        .spawn((
            Definition,
            TextInput,
            editable,
            source::font(skins.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            TextLayout::default(),
            TextCursorStyle::default(),
            bevy::input_focus::tab_navigation::TabIndex(0),
            Node {
                min_height: px(24.),
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(field);
    button(
        &mut commands,
        &skins,
        root,
        generation,
        &panel,
        Action::Definition,
        "Equip definition",
    );
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    fields: Query<&EditableText, With<Definition>>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if owner.generation() != control.generation {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if control.action == Action::Owner(Kind::Close) {
        play.link.hotkey_equip_bound(&control.expected, Kind::Close);
        return;
    }
    if !guard.allows(event.entity)
        || owner.modal_active()
        || play.view.gameplay_input_blocked(&play.link)
        || play.link.hotkey_equip_snapshot() != control.expected
    {
        return;
    }
    match control.action {
        Action::Select(slot) => {
            state.slot = slot;
            state.confirm = None;
            state.redraw = true;
        }
        Action::Owner(Kind::Clear(slot, _)) => {
            if state.confirm != Some(slot) {
                state.confirm = Some(slot);
                state.redraw = true;
                return;
            }
            play.link
                .hotkey_equip_bound(&control.expected, Kind::Clear(slot, true));
            state.confirm = None;
            state.redraw = true;
        }
        Action::Owner(kind) => {
            state.confirm = None;
            play.link.hotkey_equip_bound(&control.expected, kind);
        }
        Action::Definition => {
            state.confirm = None;
            if let Some(definition) = fields
                .iter()
                .next()
                .and_then(|v| v.editor.text().to_string().trim().parse::<u32>().ok())
            {
                play.link
                    .hotkey_equip_bound(&control.expected, Kind::Equip(definition));
            }
        }
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    owner: NonSend<HostOwner>,
    state: Res<State>,
) {
    if events.read().any(|e| {
        e.key_code == KeyCode::Escape && e.state == bevy::input::ButtonState::Pressed && !e.repeat
    }) && let Some((generation, expected)) = &state.snapshot
        && *generation == owner.generation()
        && let Some(play) = &owner.play
    {
        play.link.hotkey_equip_bound(expected, Kind::Close);
    }
}
#[cfg(test)]
mod tests;
