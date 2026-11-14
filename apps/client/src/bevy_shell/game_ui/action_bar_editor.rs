//! Advanced editing over the retained local action-bar owner and preference persistence.
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use crate::play::feature_local_spell_action_bar::view::ActionBarEditorSnapshot;
use crate::play::{BarAction, BarDock, ItemUseMode, SpellAim};
use bevy::{prelude::*, ui_widgets::Activate};
use oteryn_client_ui_bevy as source;
#[cfg(test)]
mod tests;

#[derive(Resource, Default)]
pub(super) struct State {
    snapshot: Option<(u64, ActionBarEditorSnapshot)>,
    root: Option<Entity>,
    slot: usize,
    assigned: [Option<BarAction>; 3],
    selected: usize,
    known_only: bool,
    confirm: Option<BarAction>,
    revision: u64,
    rendered: u64,
    message: String,
}
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Close,
    Slot(i8),
    Subslot(usize),
    Pick(usize),
    Clear(usize),
    Known,
    Aim(SpellAim),
    Use(ItemUseMode),
    Save,
    Confirm,
    Cancel,
    Visible(usize),
    Dock(usize),
    Lock(usize),
    Count(usize, i8),
    Scroll(usize, i8),
}
#[derive(Component)]
struct Control {
    generation: u64,
    expected: ActionBarEditorSnapshot,
    revision: u64,
    slot: usize,
    kind: Kind,
}
#[derive(Component)]
struct Choice;
#[derive(Component)]
struct RowCaption;

pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_systems(Update, (keyboard, project).chain());
}
pub(in crate::bevy_shell) fn open(owner: &mut HostOwner) {
    if !owner.modal_active()
        && let Some(play) = &owner.play
    {
        play.link.action_bar_editor_open();
    }
}
pub(super) fn edit_slot(owner: &HostOwner, state: &mut State, slot: usize) -> bool {
    if slot >= 150 || owner.modal_active() {
        return false;
    }
    let Some(play) = &owner.play else {
        return false;
    };
    if !play.link.action_bar_editor_open() {
        return false;
    }
    state.slot = slot;
    state.snapshot = None;
    state.confirm = None;
    state.revision = state.revision.wrapping_add(1);
    true
}
fn load(state: &mut State, snapshot: &ActionBarEditorSnapshot) {
    state.assigned = std::array::from_fn(|_| None);
    if let Some(BarAction::Multi { actions }) = snapshot.actions.0.get(&state.slot) {
        for (target, action) in state.assigned.iter_mut().zip(actions) {
            *target = Some(action.clone());
        }
    }
    state.confirm = None;
}
fn label(commands: &mut Commands, skins: &Skins, parent: Entity, value: &str) -> Entity {
    let child = commands
        .spawn(source::text(value, source::font(skins.font.clone(), false)))
        .id();
    commands.entity(parent).add_child(child);
    child
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    state: &State,
    kind: Kind,
    caption: &str,
) -> Entity {
    let Some((generation, expected)) = &state.snapshot else {
        return parent;
    };
    let entity = commands
        .spawn((
            source::button(skins.buttons.clone(), 155.),
            Control {
                generation: *generation,
                expected: expected.clone(),
                revision: state.revision,
                slot: state.slot,
                kind,
            },
        ))
        .observe(activate)
        .id();
    let text = commands
        .spawn(source::button_caption(caption, skins.font.clone()))
        .id();
    commands.entity(entity).add_child(text);
    commands.entity(parent).add_child(entity);
    entity
}
fn row(commands: &mut Commands, parent: Entity) -> Entity {
    let entity = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(4.),
            row_gap: px(4.),
            ..default()
        })
        .id();
    commands.entity(parent).add_child(entity);
    entity
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
        .map(|play| (owner.generation(), play.link.action_bar_editor_snapshot()))
        .filter(|(_, p)| p.open);
    if state.snapshot != current {
        if let Some((generation, snapshot)) = &current
            && state
                .snapshot
                .as_ref()
                .is_none_or(|(previous_generation, previous)| {
                    previous_generation != generation
                        || !previous.same_session(snapshot)
                        || previous.actions.0.get(&state.slot)
                            != snapshot.actions.0.get(&state.slot)
                        || previous.bindings.get(state.slot) != snapshot.bindings.get(state.slot)
                })
        {
            load(&mut state, snapshot);
        }
        state.snapshot = current;
        state.confirm = None;
        state.revision = state.revision.wrapping_add(1);
    }
    if state.rendered == state.revision {
        return;
    }
    state.rendered = state.revision;
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
    }
    let Some((_, snapshot)) = &state.snapshot else {
        return;
    };
    let Some(play) = &owner.play else {
        return;
    };
    let root = commands
        .spawn((
            GameModal,
            Name::new("action_bar_editor.window"),
            GlobalZIndex(120),
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            bevy::ui_widgets::ScrollArea,
            bevy::input_focus::tab_navigation::TabGroup::new(15),
            Node {
                position_type: PositionType::Absolute,
                left: px(100.),
                top: px(50.),
                width: px(700.),
                max_width: percent(95.),
                max_height: percent(90.),
                overflow: Overflow::scroll_y(),
                padding: px(22.).all(),
                flex_direction: FlexDirection::Column,
                row_gap: px(5.),
                ..default()
            },
        ))
        .id();
    source::dialog_caption(&mut commands, root, "Action bar editor", skins.font.clone());
    for (index, layout) in snapshot.layout.rows.iter().enumerate() {
        let text = label(
            &mut commands,
            &skins,
            root,
            &format!(
                "Bar {}: {:?}, {}, {}, {} slots, offset {}",
                index + 1,
                layout.dock,
                if layout.enabled { "On" } else { "Off" },
                if layout.locked { "Locked" } else { "Edit/drag" },
                layout.count,
                layout.offset
            ),
        );
        commands.entity(text).insert(RowCaption);
        let tools = row(&mut commands, root);
        for (kind, caption) in [
            (Kind::Visible(index), "On/off"),
            (Kind::Dock(index), "Dock"),
            (Kind::Lock(index), "Lock/edit"),
            (Kind::Count(index, -1), "Fewer slots"),
            (Kind::Count(index, 1), "More slots"),
            (Kind::Scroll(index, -1), "<"),
            (Kind::Scroll(index, 1), ">"),
        ] {
            button(&mut commands, &skins, tools, &state, kind, caption);
        }
    }
    label(
        &mut commands,
        &skins,
        root,
        &format!(
            "Assign Multiaction — slot {} (bar {}, position {})",
            state.slot + 1,
            state.slot / 50 + 1,
            state.slot % 50 + 1
        ),
    );
    let tools = row(&mut commands, root);
    for (kind, caption) in [
        (Kind::Slot(-1), "Previous slot"),
        (Kind::Slot(1), "Next slot"),
        (
            Kind::Known,
            oteryn_client_ui::spell_action_bar::SHOW_ONLY_KNOWN_LABEL,
        ),
    ] {
        button(&mut commands, &skins, tools, &state, kind, caption);
    }
    label(
        &mut commands,
        &skins,
        root,
        if state.known_only {
            "Known spells only: On"
        } else {
            "Known spells only: Off"
        },
    );
    let captions = play.link.action_bar_editor_labels(&state.assigned);
    for (index, caption) in captions.iter().enumerate() {
        let tools = row(&mut commands, root);
        button(
            &mut commands,
            &skins,
            tools,
            &state,
            Kind::Subslot(index),
            &format!(
                "{}{}: {}",
                if state.selected == index { "> " } else { "" },
                ["I", "II", "III"][index],
                caption
            ),
        );
        button(
            &mut commands,
            &skins,
            tools,
            &state,
            Kind::Clear(index),
            "Clear",
        );
    }
    let tools = row(&mut commands, root);
    match &state.assigned[state.selected] {
        Some(BarAction::Spell { aim, .. }) => {
            label(&mut commands, &skins, root, aim.option().label());
            for aim in SpellAim::ALL {
                button(
                    &mut commands,
                    &skins,
                    tools,
                    &state,
                    Kind::Aim(aim),
                    aim.option().label(),
                );
            }
        }
        Some(BarAction::Use { .. }) => {
            for (mode, caption) in [
                (ItemUseMode::CursorPosition, "Use at cursor position"),
                (ItemUseMode::SelfActor, "Use on self"),
                (ItemUseMode::Target, "Use on current target"),
                (ItemUseMode::Cursor, "Use with crosshair"),
            ] {
                button(
                    &mut commands,
                    &skins,
                    tools,
                    &state,
                    Kind::Use(mode),
                    caption,
                );
            }
        }
        _ => {
            label(
                &mut commands,
                &skins,
                root,
                "Select a spell or object and a slot (I, II or III).",
            );
        }
    }
    let choices = row(&mut commands, root);
    for (index, choice) in snapshot
        .choices
        .iter()
        .enumerate()
        .filter(|(_, c)| !state.known_only || c.known)
    {
        let entity = button(
            &mut commands,
            &skins,
            choices,
            &state,
            Kind::Pick(index),
            &choice.label,
        );
        commands.entity(entity).insert(Choice);
    }
    label(
        &mut commands,
        &skins,
        root,
        "First action off cooldown will execute.",
    );
    label(&mut commands, &skins, root, &state.message);
    if let Some(BarAction::Multi { actions }) = &state.confirm {
        // This is the frozen replacement, invalidated by every subsequent draft/target edit.
        let frozen: [Option<BarAction>; 3] = std::array::from_fn(|i| actions.get(i).cloned());
        let mut labels = play.link.action_bar_editor_labels(&frozen);
        for (caption, action) in labels.iter_mut().zip(&frozen) {
            if let Some(BarAction::Spell { aim, .. }) = action {
                caption.push_str(&format!(" ({})", aim.option().label()));
            }
        }
        label(
            &mut commands,
            &skins,
            root,
            &format!(
                "Replace slot {} with: {}?",
                state.slot + 1,
                labels.join("; ")
            ),
        );
        let tools = row(&mut commands, root);
        button(
            &mut commands,
            &skins,
            tools,
            &state,
            Kind::Confirm,
            "Confirm replacement",
        );
        button(&mut commands, &skins, tools, &state, Kind::Cancel, "Cancel");
    } else {
        button(
            &mut commands,
            &skins,
            root,
            &state,
            Kind::Save,
            "Save multiaction",
        );
    }
    button(&mut commands, &skins, root, &state, Kind::Close, "Close");
    state.root = Some(root);
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    let Some(play) = &owner.play else {
        return;
    };
    if control.generation != owner.generation() {
        return;
    }
    // Close precedes modal, draft, target and revision admission.
    if control.kind == Kind::Close {
        play.link.action_bar_editor_close_bound(&control.expected);
        return;
    }
    if !guard.allows(event.entity)
        || owner.modal_active()
        || state.slot != control.slot
        || state.revision != control.revision
        || state.snapshot.as_ref() != Some(&(control.generation, control.expected.clone()))
        || play.link.action_bar_editor_snapshot() != control.expected
    {
        return;
    }
    let mut layout = control.expected.layout.clone();
    let mut layout_edit = false;
    let previous_confirm = state.confirm.take();
    match control.kind {
        Kind::Slot(by) => {
            state.slot = state.slot.saturating_add_signed(isize::from(by)).min(149);
            load(&mut state, &control.expected);
        }
        Kind::Subslot(index) => state.selected = index,
        Kind::Pick(index) => {
            if let Some(choice) = control.expected.choices.get(index) {
                let selected = state.selected;
                state.assigned[selected] = Some(choice.action.clone());
            }
        }
        Kind::Clear(index) => state.assigned[index] = None,
        Kind::Known => state.known_only = !state.known_only,
        Kind::Aim(next) => {
            let selected = state.selected;
            if let Some(BarAction::Spell { aim, .. }) = &mut state.assigned[selected] {
                *aim = next;
            }
        }
        Kind::Use(next) => {
            let selected = state.selected;
            if let Some(BarAction::Use { mode, .. }) = &mut state.assigned[selected] {
                *mode = next;
            }
        }
        Kind::Save | Kind::Confirm => {
            let action = BarAction::Multi {
                actions: state.assigned.iter().flatten().cloned().collect(),
            };
            let replacing = control.expected.actions.0.contains_key(&state.slot)
                || control
                    .expected
                    .bindings
                    .get(state.slot)
                    .is_some_and(|value| !value.is_empty());
            if control.kind == Kind::Save && replacing {
                state.confirm = Some(action);
            } else if (!replacing || previous_confirm.as_ref() == Some(&action))
                && play
                    .link
                    .action_bar_editor_multi_bound(&control.expected, state.slot, action)
            {
                state.message = "Multiaction saved".into();
            } else {
                state.message =
                    "Assign at least one live spell or object; the owner may have changed.".into();
            }
        }
        Kind::Cancel => state.message.clear(),
        Kind::Visible(index) => {
            layout.rows[index].enabled = !layout.rows[index].enabled;
            layout_edit = true;
        }
        Kind::Dock(index) => {
            layout.rows[index].dock = match layout.rows[index].dock {
                BarDock::Bottom => BarDock::Left,
                BarDock::Left => BarDock::Right,
                BarDock::Right => BarDock::Bottom,
            };
            layout_edit = true;
        }
        Kind::Lock(index) => {
            layout.rows[index].locked = !layout.rows[index].locked;
            layout_edit = true;
        }
        Kind::Count(index, by) => {
            let row = &mut layout.rows[index];
            row.count = row.count.saturating_add_signed(by).clamp(1, 50);
            row.offset = row.offset.min(row.count - 1);
            layout_edit = true;
        }
        Kind::Scroll(index, by) => {
            let row = &mut layout.rows[index];
            row.offset = row.offset.saturating_add_signed(by).min(row.count - 1);
            layout_edit = true;
        }
        Kind::Close => return,
    }
    if layout_edit {
        play.link
            .action_bar_editor_layout_bound(&control.expected, layout);
    }
    state.revision = state.revision.wrapping_add(1);
}
fn keyboard(owner: NonSend<HostOwner>, state: Res<State>, keys: Option<Res<ButtonInput<KeyCode>>>) {
    if keys.is_some_and(|keys| keys.just_pressed(KeyCode::Escape))
        && let Some((generation, snapshot)) = &state.snapshot
        && *generation == owner.generation()
        && let Some(play) = &owner.play
    {
        play.link.action_bar_editor_close_bound(snapshot);
    }
}
