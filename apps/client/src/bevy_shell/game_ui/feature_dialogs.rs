//! Real session-owned server dialogs; Bevy owns only focus and transient selection.
mod build;
mod notices;

use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
};
use bevy::{
    input_focus::{FocusCause, InputFocus},
    prelude::*,
    text::EditableText,
    ui::{Checked, InteractionDisabled},
    ui_widgets::{Activate, ValueChange},
};
use oteryn_client_ui::{
    server_modal::Answer,
    server_modal_input::{self, Entry},
};

#[derive(Component)]
struct Overlay;
#[derive(Component)]
struct ValueField;
#[derive(Component)]
struct Feedback;
#[derive(Component)]
struct RememberIndicator;
#[derive(Component, Clone, Copy)]
struct Action {
    key: Key,
    kind: ActionKind,
}
#[derive(Clone, Copy)]
enum ActionKind {
    Choice(u8),
    Button(u8),
    Remember,
}
#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    generation: u64,
    revision: u64,
}
#[derive(Resource, Default)]
struct State {
    key: Option<Key>,
    selected: Option<u8>,
    remember: bool,
    feedback: String,
    panel: Option<Entity>,
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_systems(Startup, (setup, notices::setup).after(super::setup))
        .add_systems(
            Update,
            (keyboard, project, refresh, notices::project).chain(),
        );
}
fn setup(mut commands: Commands) {
    commands.spawn((
        Overlay,
        GameModal,
        GlobalZIndex(180),
        Node {
            position_type: PositionType::Absolute,
            width: percent(100.),
            height: percent(100.),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            display: Display::None,
            ..default()
        },
        BackgroundColor(Color::srgba(0., 0., 0., 0.55)),
    ));
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skin: Res<Skins>,
    mut state: ResMut<State>,
    mut images: ResMut<Assets<Image>>,
    mut overlay: Query<(Entity, &mut Node), With<Overlay>>,
    mut focus: ResMut<InputFocus>,
) {
    let projected = owner
        .play
        .as_ref()
        .and_then(|play| play.link.server_modal_panel());
    let key = projected.as_ref().map(|panel| Key {
        generation: owner.generation(),
        revision: panel.revision,
    });
    let Ok((overlay, mut node)) = overlay.single_mut() else {
        return;
    };
    node.display = if key.is_some() {
        Display::Flex
    } else {
        Display::None
    };
    if state.key == key {
        return;
    }
    if let Some(panel) = state.panel.take() {
        if focus.get().is_some() {
            focus.clear();
        }
        commands.entity(panel).despawn();
    }
    state.key = key;
    state.selected = None;
    state.remember = false;
    state.feedback.clear();
    let (Some(key), Some(projected)) = (key, projected) else {
        return;
    };
    let (panel, initial_focus) = build::panel(
        &mut commands,
        &skin,
        overlay,
        key,
        &projected.panel,
        &mut images,
    );
    state.panel = Some(panel);
    if let Some(entity) = initial_focus {
        focus.set(entity, FocusCause::Navigated);
    }
}
fn current(owner: &HostOwner, state: &State, key: Key) -> bool {
    state.key == Some(key)
        && owner.generation() == key.generation
        && owner
            .play
            .as_ref()
            .and_then(|p| p.link.server_modal_panel())
            .is_some_and(|p| p.revision == key.revision)
}
fn submit(owner: &HostOwner, state: &mut State, key: Key, button: u8, text: &str) -> bool {
    if !current(owner, state, key) {
        return false;
    }
    let Some(play) = &owner.play else {
        return false;
    };
    let Some(projected) = play.link.server_modal_panel() else {
        return false;
    };
    if projected.queued {
        return false;
    }
    let Some(dialog) = &projected.panel.active else {
        return false;
    };
    let answer = Answer {
        dialog_id: dialog.id,
        button_id: button,
        choice_id: state.selected,
    };
    let cancel = dialog.cancel_button == Some(button);
    let value = if cancel {
        None
    } else {
        projected
            .panel
            .active_input
            .as_ref()
            .and_then(|input| server_modal_input::parse_value(input, text))
    };
    let accepted = play.link.server_modal_submit(
        key.revision,
        answer,
        Entry {
            value,
            dont_show_again: state.remember,
        },
    );
    state.feedback = if accepted {
        "Waiting for server…"
    } else {
        "Select an option and enter a value within the limits."
    }
    .into();
    accepted
}
fn activate(
    event: On<Activate>,
    actions: Query<&Action>,
    input: Query<&EditableText, With<ValueField>>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    if !current(&owner, &state, action.key) {
        return;
    }
    match action.kind {
        ActionKind::Choice(id) => state.selected = Some(id),
        ActionKind::Remember => {}
        ActionKind::Button(id) => {
            let text = input
                .single()
                .map_or_else(|_| String::new(), |text| text.value().to_string());
            submit(&owner, &mut state, action.key, id, &text);
        }
    }
}
fn remember(
    event: On<ValueChange<bool>>,
    actions: Query<&Action>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    if let Ok(action) = actions.get(event.source)
        && current(&owner, &state, action.key)
    {
        state.remember = event.value;
    }
}
fn keyboard(
    mut keys: MessageReader<bevy::input::keyboard::KeyboardInput>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    input: Query<&EditableText, With<ValueField>>,
) {
    let Some(key) = state.key else {
        keys.clear();
        return;
    };
    for event in keys.read() {
        if event.state != bevy::input::ButtonState::Pressed || event.repeat {
            continue;
        }
        if event.key_code == KeyCode::Escape {
            let button = owner
                .play
                .as_ref()
                .and_then(|p| p.link.server_modal_panel())
                .and_then(|p| p.panel.active)
                .and_then(|p| p.cancel_button);
            if let Some(button) = button {
                let text = input
                    .single()
                    .map_or_else(|_| String::new(), |text| text.value().to_string());
                submit(&owner, &mut state, key, button, &text);
            }
        }
    }
}
fn refresh(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    state: Res<State>,
    mut texts: Query<&mut Text, With<Feedback>>,
    buttons: Query<(Entity, &Action, Has<InteractionDisabled>, Has<Checked>)>,
    skin: Res<Skins>,
    mut indicators: Query<&mut ImageNode, With<RememberIndicator>>,
) {
    let projected = owner
        .play
        .as_ref()
        .and_then(|p| p.link.server_modal_panel());
    for mut text in &mut texts {
        let value = projected
            .as_ref()
            .and_then(|p| p.panel.feedback)
            .map_or_else(|| state.feedback.clone(), str::to_owned);
        if text.0 != value {
            text.0 = value;
        }
    }
    let queued = projected.is_some_and(|p| p.queued);
    for mut indicator in &mut indicators {
        indicator.image = if state.remember {
            skin.checked.clone()
        } else {
            skin.unchecked.clone()
        };
    }
    for (entity, action, disabled, checked) in &buttons {
        let selected = match action.kind {
            ActionKind::Remember => state.remember,
            ActionKind::Choice(id) => state.selected == Some(id),
            ActionKind::Button(_) => false,
        };
        if selected && !checked {
            commands.entity(entity).insert(Checked);
        }
        if !selected && checked {
            commands.entity(entity).remove::<Checked>();
        }
        if queued && !disabled {
            commands.entity(entity).insert(InteractionDisabled);
        }
        if !queued && disabled {
            commands.entity(entity).remove::<InteractionDisabled>();
        }
    }
}

#[cfg(test)]
mod tests;
