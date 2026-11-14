//! Battle-list configuration projects the existing local preference owner.
use super::{GameModal, modal::ModalGuard};
use crate::bevy_shell::{owner::HostOwner, ui::Skins};
use crate::play::feature_0017_attack::view::{
    BattleConfigAction as Action, BattleConfigBinding as Binding, BattleListPreferences,
    BattleSort, GuildFilter,
};
use bevy::{
    prelude::*,
    text::{EditableText, TextCursorStyle},
    ui_widgets::{Activate, TextInput},
};
use oteryn_client_ui_bevy as source;

#[derive(Resource, Default)]
struct State {
    snapshot: Option<(u64, Binding)>,
    root: Option<Entity>,
    draft: Option<BattleListPreferences>,
    fields: [String; 3],
    confirm: Option<(BattleListPreferences, [String; 3])>,
    redraw: bool,
    feedback: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Select(usize),
    Save,
    Add,
    Remove,
    ConfirmRemove,
    Close,
    Players,
    Creatures,
    Sort,
    Vocation,
    Guild,
}
#[derive(Component, Clone)]
struct Control {
    generation: u64,
    binding: Binding,
    kind: Kind,
}
#[derive(Component)]
struct Field(usize);

pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, invalidate_confirmation, project).chain());
}
pub(in crate::bevy_shell) fn open(owner: &HostOwner) {
    if !owner.modal_active()
        && let Some(play) = &owner.play
    {
        let binding = play.link.battle_config_binding();
        play.link.battle_config_apply(&binding, Action::Open);
    }
}
pub(in crate::bevy_shell) fn input(
    owner: &HostOwner,
    events: &[oteryn_input_actions::NormalizedInputEvent],
) -> bool {
    if !oteryn_client_ui::battle::toggle_key(events) {
        return false;
    }
    if let Some(play) = &owner.play {
        let binding = play.link.battle_config_binding();
        play.link.battle_config_apply(
            &binding,
            if binding.open {
                Action::Close
            } else {
                Action::Open
            },
        );
    }
    true
}
#[derive(Component)]
struct Entry(u64);
pub(super) fn entry(commands: &mut Commands, skins: &Skins, generation: u64) -> Entity {
    let entity = commands
        .spawn((
            Entry(generation),
            source::button(skins.buttons.clone(), 174.),
        ))
        .observe(open_entry)
        .id();
    label(commands, skins, entity, "Configure battle lists");
    entity
}
fn open_entry(
    event: On<Activate>,
    entries: Query<&Entry>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
) {
    if guard.allows(event.entity)
        && entries
            .get(event.entity)
            .is_ok_and(|entry| entry.0 == owner.generation())
    {
        open(&owner);
    }
}
fn label(commands: &mut Commands, skins: &Skins, root: Entity, text: &str) {
    let child = commands
        .spawn(source::text(text, source::font(skins.font.clone(), false)))
        .id();
    commands.entity(root).add_child(child);
}
fn button(commands: &mut Commands, skins: &Skins, root: Entity, control: Control, text: &str) {
    let child = commands
        .spawn((control, source::button(skins.buttons.clone(), 300.)))
        .observe(activate)
        .id();
    label(commands, skins, child, text);
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
        .map(|play| (owner.generation(), play.link.battle_config_binding()))
        .filter(|(_, binding)| binding.open);
    let same = state
        .snapshot
        .as_ref()
        .zip(current.as_ref())
        .is_some_and(|((a, old), (b, new))| a == b && old.same_projection(new));
    if same && !state.redraw || state.snapshot.is_none() && current.is_none() {
        return;
    }
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
    }
    if !same {
        state.draft = current
            .as_ref()
            .and_then(|(_, binding)| binding.lists.preferences.get(binding.lists.active))
            .cloned();
        state.fields = state
            .draft
            .as_ref()
            .map(|draft| {
                [
                    draft.name.clone(),
                    draft.search.clone(),
                    draft.max_health.to_string(),
                ]
            })
            .unwrap_or_default();
        state.confirm = None;
        state.feedback.clear();
    }
    state.snapshot = current.clone();
    state.redraw = false;
    let Some((generation, binding)) = current else {
        return;
    };
    let Some(draft) = state.draft.clone() else {
        return;
    };
    let root = commands
        .spawn((
            GameModal,
            Name::new("battle_config.window"),
            GlobalZIndex(120),
            bevy::ui_widgets::ScrollArea,
            bevy::input_focus::tab_navigation::TabGroup::new(15),
            source::frame(skins.frame.clone(), [2., 17., 2., 2.]),
            Node {
                position_type: PositionType::Absolute,
                left: px(200.),
                top: px(60.),
                width: px(460.),
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
        "Battle lists — Alt+B",
        skins.font.clone(),
    );
    let control = |kind| Control {
        generation,
        binding: binding.clone(),
        kind,
    };
    for (index, list) in binding.lists.preferences.iter().enumerate() {
        button(
            &mut commands,
            &skins,
            root,
            control(Kind::Select(index)),
            &list.name,
        );
    }
    for (index, (caption, value, limit)) in [
        ("List name", state.fields[0].clone(), 32),
        ("Name filter", state.fields[1].clone(), 64),
        ("Maximum HP % (0–100)", state.fields[2].clone(), 3),
    ]
    .into_iter()
    .enumerate()
    {
        label(&mut commands, &skins, root, caption);
        let mut editable = EditableText::new(value);
        editable.max_characters = Some(limit);
        editable.allow_newlines = false;
        let field = commands
            .spawn((
                Field(index),
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
    }
    for (kind, text) in [
        (Kind::Players, format!("Players: {}", draft.players)),
        (Kind::Creatures, format!("Creatures: {}", draft.creatures)),
        (
            Kind::Sort,
            format!(
                "Sort: {}",
                match draft.sort {
                    BattleSort::Name => "Name",
                    BattleSort::HealthAscending => "Health: low first",
                    BattleSort::HealthDescending => "Health: high first",
                    BattleSort::Identity => "Identity",
                }
            ),
        ),
        (
            Kind::Vocation,
            format!("Profession: {}", vocation_label(draft.vocation)),
        ),
        (
            Kind::Guild,
            format!(
                "Guild: {}",
                match draft.guild {
                    GuildFilter::Any => "Any Guild relationship",
                    GuildFilter::SameGuild => "My Guild",
                    GuildFilter::OutsideGuild => "Outside my Guild",
                }
            ),
        ),
    ] {
        button(&mut commands, &skins, root, control(kind), &text);
    }
    label(
        &mut commands,
        &skins,
        root,
        "Specific player filters exclude unknown details. Creature filtering remains separate.",
    );
    for (kind, caption) in [
        (Kind::Save, "Save list"),
        (Kind::Add, "Add list"),
        (Kind::Remove, "Remove list"),
        (Kind::Close, "Close"),
    ] {
        button(&mut commands, &skins, root, control(kind), caption);
    }
    if state.confirm.is_some() {
        button(
            &mut commands,
            &skins,
            root,
            control(Kind::ConfirmRemove),
            "Confirm remove list",
        );
        label(
            &mut commands,
            &skins,
            root,
            &format!(
                "Remove list \"{}\"? Confirm removal below.",
                binding.lists.preferences[binding.lists.active].name
            ),
        );
    }
    label(&mut commands, &skins, root, &state.feedback);
}
fn vocation_label(vocation: Option<u8>) -> &'static str {
    match vocation {
        None => "Any profession",
        Some(0) => "No vocation",
        Some(1) => "Sorcerer",
        Some(2) => "Druid",
        Some(3) => "Paladin",
        Some(4) => "Knight",
        Some(5) => "Monk",
        Some(_) => "Any profession",
    }
}
fn read_fields(values: &[String; 3], fields: &Query<(&Field, &EditableText)>) -> [String; 3] {
    let mut values = values.clone();
    for (field, value) in fields {
        if let Some(target) = values.get_mut(field.0) {
            *target = value.editor.text().to_string();
        }
    }
    values
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    fields: Query<(&Field, &EditableText)>,
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
    if control.kind == Kind::Close {
        play.link
            .battle_config_apply(&control.binding, Action::Close);
        return;
    }
    if !guard.allows(event.entity)
        || owner.modal_active()
        || play.view.gameplay_input_blocked(&play.link)
        || !control
            .binding
            .same_projection(&play.link.battle_config_binding())
    {
        return;
    }
    state.fields = read_fields(&state.fields, &fields);
    let Some(mut draft) = state.draft.clone() else {
        return;
    };
    let action = match control.kind {
        Kind::Select(index) => Some(Action::Select(index)),
        Kind::Save => {
            let Ok(max_health) = state.fields[2].parse() else {
                state.feedback = "Enter a maximum HP percentage from 0 to 100".into();
                state.confirm = None;
                state.redraw = true;
                return;
            };
            draft.name.clone_from(&state.fields[0]);
            draft.search.clone_from(&state.fields[1]);
            draft.max_health = max_health;
            Some(Action::Save(draft.clone()))
        }
        Kind::Add => Some(Action::Add),
        Kind::Remove => {
            if control.binding.lists.preferences.len() <= 1 {
                return;
            }
            state.confirm = Some((draft.clone(), state.fields.clone()));
            state.draft = Some(draft);
            state.redraw = true;
            return;
        }
        Kind::ConfirmRemove => {
            if state.confirm.as_ref() != Some(&(draft.clone(), state.fields.clone())) {
                return;
            }
            Some(Action::Remove(true))
        }
        Kind::Players => {
            draft.players = !draft.players;
            None
        }
        Kind::Creatures => {
            draft.creatures = !draft.creatures;
            None
        }
        Kind::Sort => {
            draft.sort = match draft.sort {
                BattleSort::Name => BattleSort::HealthAscending,
                BattleSort::HealthAscending => BattleSort::HealthDescending,
                BattleSort::HealthDescending => BattleSort::Identity,
                BattleSort::Identity => BattleSort::Name,
            };
            None
        }
        Kind::Vocation => {
            draft.vocation = match draft.vocation {
                None => Some(0),
                Some(0..=4) => draft.vocation.map(|v| v + 1),
                Some(_) => None,
            };
            None
        }
        Kind::Guild => {
            draft.guild = match draft.guild {
                GuildFilter::Any => GuildFilter::SameGuild,
                GuildFilter::SameGuild => GuildFilter::OutsideGuild,
                GuildFilter::OutsideGuild => GuildFilter::Any,
            };
            None
        }
        Kind::Close => return,
    };
    state.confirm = None;
    state.draft = Some(draft);
    state.redraw = true;
    if let Some(action) = action
        && !play.link.battle_config_apply(&control.binding, action)
    {
        state.feedback =
            "Use unique list names (1–32 bytes), a filter up to 64 bytes and HP from 0 to 100"
                .into();
    }
}
fn invalidate_confirmation(fields: Query<(&Field, &EditableText)>, mut state: ResMut<State>) {
    let values = read_fields(&state.fields, &fields);
    if state
        .confirm
        .as_ref()
        .is_some_and(|(_, frozen)| *frozen != values)
    {
        state.confirm = None;
        state.fields = values;
        state.redraw = true;
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    owner: NonSend<HostOwner>,
    state: Res<State>,
) {
    if events.read().any(|event| {
        event.key_code == KeyCode::Escape
            && event.state == bevy::input::ButtonState::Pressed
            && !event.repeat
    }) && let Some((generation, binding)) = &state.snapshot
        && *generation == owner.generation()
        && let Some(play) = &owner.play
    {
        play.link.battle_config_apply(binding, Action::Close);
    }
}
#[cfg(test)]
mod tests;
