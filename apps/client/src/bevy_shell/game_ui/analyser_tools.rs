//! Native analyser form over the existing session owner and its local calculations.
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use crate::play::feature_0010_analyser::input::{
    ANALYSER_TOOLS_LABELS, AnalyserToolsSnapshot, requires_confirmation,
};
use bevy::{
    prelude::*,
    text::{EditableText, TextCursorStyle},
    ui_widgets::{Activate, TextInput},
};
use oteryn_client_ui_bevy as source;

#[derive(Resource, Default)]
struct State {
    shown: Option<(u64, AnalyserToolsSnapshot)>,
    root: Option<Entity>,
    confirmation: Option<(usize, [String; 4])>,
    confirmation_root: Option<Entity>,
}
#[derive(Component)]
struct Field(usize);
#[derive(Component)]
struct OwnerText;
#[derive(Component, Clone)]
struct Control {
    generation: u64,
    expected: AnalyserToolsSnapshot,
    action: Option<usize>,
    confirmed: bool,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain())
        .add_systems(
            PostUpdate,
            invalidate_confirmation.after(bevy::text::EditableTextSystems),
        );
}
pub(in crate::bevy_shell) fn open(owner: &HostOwner) {
    if !owner.modal_active()
        && let Some(play) = &owner.play
        && !play.view.gameplay_input_blocked(&play.link)
    {
        play.link.analyser_tools_open_bound();
    }
}
fn label(commands: &mut Commands, skins: &Skins, root: Entity, text: &str) -> Entity {
    let child = commands
        .spawn(source::text(text, source::font(skins.font.clone(), false)))
        .id();
    commands.entity(root).add_child(child);
    child
}
fn button(commands: &mut Commands, skins: &Skins, root: Entity, control: Control, text: &str) {
    let child = commands
        .spawn((control, source::button(skins.buttons.clone(), 260.)))
        .observe(activate)
        .id();
    let caption = commands
        .spawn(source::button_caption(text, skins.font.clone()))
        .id();
    commands.entity(child).add_child(caption);
    commands.entity(root).add_child(child);
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    mut texts: Query<&mut Text, With<OwnerText>>,
    fields: Query<(&Field, &EditableText)>,
) {
    let next = owner
        .play
        .as_ref()
        .filter(|p| p.link.analyser_panel().open)
        .map(|p| (owner.generation(), p.link.analyser_tools_snapshot()));
    if let Some((_, snapshot)) = &next {
        for mut text in &mut texts {
            text.0 = snapshot.text.replace("\r\n", "\n");
        }
    }
    if state
        .shown
        .as_ref()
        .zip(next.as_ref())
        .is_some_and(|((a, x), (b, y))| a == b && x.same(y))
        || (state.shown.is_none() && next.is_none())
    {
        return;
    }
    let draft = state
        .shown
        .as_ref()
        .zip(next.as_ref())
        .filter(|((a, old), (b, new))| a == b && old.fields == new.fields)
        .map(|((_, old), _)| values(&fields, old));
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
    }
    state.confirmation = None;
    state.confirmation_root = None;
    state.shown = next.clone();
    let Some((generation, expected)) = next else {
        return;
    };
    let root = commands
        .spawn((
            GameModal,
            Name::new("analyser_tools.window"),
            GlobalZIndex(165),
            bevy::ui_widgets::ScrollArea,
            bevy::input_focus::tab_navigation::TabGroup::new(15),
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                position_type: PositionType::Absolute,
                left: percent(15.),
                top: percent(5.),
                width: percent(70.),
                max_height: percent(90.),
                overflow: Overflow::scroll_y(),
                padding: UiRect::all(px(24.)),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.),
                ..default()
            },
        ))
        .id();
    state.root = Some(root);
    source::dialog_caption(&mut commands, root, "Analysers", skins.font.clone());
    let text = label(
        &mut commands,
        &skins,
        root,
        &expected.text.replace("\r\n", "\n"),
    );
    commands.entity(text).insert(OwnerText);
    for (index, caption) in [
        "XP still needed for your level goal (blank disables)",
        "Item / creature race number, boss death id, or party member name",
        "Unit item value (gold), kill target, or party extra cost",
        "Session export / party summary file path",
    ]
    .into_iter()
    .enumerate()
    {
        label(&mut commands, &skins, root, caption);
        let mut editable =
            EditableText::new(draft.as_ref().unwrap_or(&expected.fields)[index].clone());
        editable.max_characters = Some(7996);
        editable.allow_newlines = false;
        let child = commands
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
        commands.entity(root).add_child(child);
    }
    for (action, caption) in ANALYSER_TOOLS_LABELS.into_iter().enumerate() {
        button(
            &mut commands,
            &skins,
            root,
            Control {
                generation,
                expected: expected.clone(),
                action: Some(action),
                confirmed: false,
            },
            caption,
        );
    }
    button(
        &mut commands,
        &skins,
        root,
        Control {
            generation,
            expected,
            action: None,
            confirmed: false,
        },
        "Close",
    );
}
fn values(
    fields: &Query<(&Field, &EditableText)>,
    expected: &AnalyserToolsSnapshot,
) -> [String; 4] {
    let mut values = expected.fields.clone();
    for (field, value) in fields {
        values[field.0] = value.editor.text().to_string();
    }
    values
}
// Activation uses the displayed connection and target, then revalidates under the owner lock.
#[allow(clippy::too_many_arguments)]
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    fields: Query<(&Field, &EditableText)>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    mut commands: Commands,
    skins: Res<Skins>,
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
    // Close precedes draft, modal, capability and pending validation.
    let Some(action) = control.action else {
        play.link.analyser_tools_close_bound(&control.expected);
        return;
    };
    if !guard.allows(event.entity)
        || owner.modal_active()
        || play.view.gameplay_input_blocked(&play.link)
        || !control.expected.same(&play.link.analyser_tools_snapshot())
    {
        return;
    }
    let values = values(&fields, &control.expected);
    if requires_confirmation(action) {
        if !control.confirmed {
            if let Some(root) = state.confirmation_root.take() {
                commands.entity(root).despawn();
            }
            state.confirmation = Some((action, values.clone()));
            if let Some(root) = state.root {
                let prompt = commands
                    .spawn(Node {
                        flex_direction: FlexDirection::Column,
                        ..default()
                    })
                    .id();
                commands.entity(root).add_child(prompt);
                state.confirmation_root = Some(prompt);
                label(
                    &mut commands,
                    &skins,
                    prompt,
                    &format!(
                        "Confirm {}\nGoal: {}\nItem / race / death / member: {}\nValue / target / extra cost: {}\nFile: {}",
                        ANALYSER_TOOLS_LABELS[action], values[0], values[1], values[2], values[3]
                    ),
                );
                button(
                    &mut commands,
                    &skins,
                    prompt,
                    Control {
                        confirmed: true,
                        ..control.clone()
                    },
                    "Confirm",
                );
            }
            return;
        }
        if state.confirmation.as_ref() != Some(&(action, values.clone())) {
            return;
        }
    }
    play.link
        .analyser_tools_submit_bound(&control.expected, action, values, control.confirmed);
    state.confirmation = None;
    if let Some(root) = state.confirmation_root.take() {
        commands.entity(root).despawn();
    }
}
fn invalidate_confirmation(
    mut commands: Commands,
    fields: Query<(&Field, &EditableText)>,
    mut state: ResMut<State>,
) {
    if let Some((_, expected)) = &state.shown
        && let Some((_, frozen)) = &state.confirmation
        && values(&fields, expected) != *frozen
    {
        state.confirmation = None;
        if let Some(root) = state.confirmation_root.take() {
            commands.entity(root).despawn();
        }
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
    }) && let Some((generation, expected)) = &state.shown
        && *generation == owner.generation()
        && let Some(play) = &owner.play
    {
        play.link.analyser_tools_close_bound(expected);
    }
}

pub(super) fn select(owner: &HostOwner, tool: oteryn_client_ui::analyser::Tool) {
    open(owner);
    if let Some(play) = &owner.play
        && let Some(action) = oteryn_client_ui::analyser::Tool::ALL
            .iter()
            .position(|t| *t == tool)
    {
        let expected = play.link.analyser_tools_snapshot();
        play.link
            .analyser_tools_submit_bound(&expected, action, expected.fields.clone(), false);
    }
}

#[cfg(test)]
mod tests;
