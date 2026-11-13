//! Source-shaped options over the existing full preference draft and acceptance owner.
use super::modal::ModalGuard;
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
};
use bevy::{
    input_focus::{InputFocus, tab_navigation::TabGroup},
    prelude::*,
};
use metadata::{Editor, Page, Row, Value};
use oteryn_client_ui_bevy as source;

mod audio_feedback;
mod commands;
mod metadata;
mod projection;
#[cfg(test)]
mod tests;
mod widgets;

#[derive(Resource)]
pub(in crate::bevy_shell) struct Installed;

#[derive(Resource, Default)]
struct OptionsState {
    page: Page,
    advanced: bool,
    choice: Option<&'static str>,
    generation: Option<u64>,
}
#[derive(Resource)]
struct CheckSkins {
    checked: Handle<Image>,
    unchecked: Handle<Image>,
}
#[derive(Component)]
struct OptionsDialog;
#[derive(Component)]
struct Navigation;
#[derive(Component)]
struct PageBody;
#[derive(Component)]
struct Feedback;
#[derive(Component, Clone, Copy)]
struct Indicator(Entity);
#[derive(Component)]
struct ChoiceCaption(&'static str);
#[derive(Component)]
struct NumberCaption(&'static str);
#[derive(Component)]
struct ChoicePopup(&'static str);
#[derive(Component)]
struct Thumb(Entity);
#[derive(Component, Clone, Copy)]
enum Control {
    Page(Page),
    Advanced,
    Check(&'static str),
    Number(&'static str),
    Choice(&'static str),
    Select(&'static str, usize),
    Reset,
    Apply,
    Ok,
    Cancel,
}
pub(super) fn install(app: &mut App) {
    app.insert_resource(Installed)
        .init_resource::<OptionsState>()
        .add_systems(Startup, widgets::setup.after(super::super::ui::setup))
        .add_systems(Update, (keyboard, projection::project).chain())
        .add_systems(Update, audio_feedback::project.after(projection::project));
}
fn top_server_dialog(owner: &HostOwner) -> bool {
    owner
        .play
        .as_ref()
        .is_some_and(|play| play.link.server_modal_active())
}
fn keyboard(
    mut keys: MessageReader<bevy::input::keyboard::KeyboardInput>,
    dialogs: Query<Entity, With<OptionsDialog>>,
    mut owner: NonSendMut<HostOwner>,
    mut state: ResMut<OptionsState>,
    guard: ModalGuard,
) {
    if !dialogs.iter().any(|entity| guard.allows(entity))
        || owner.preferences_draft().is_none()
        || top_server_dialog(&owner)
    {
        keys.clear();
        return;
    }
    for key in keys.read() {
        if key.state != bevy::input::ButtonState::Pressed || key.repeat {
            continue;
        }
        if key.key_code == KeyCode::Escape {
            if state.choice.take().is_none() {
                owner.cancel();
            }
            break;
        }
    }
}
fn activate(
    event: On<bevy::ui_widgets::Activate>,
    controls: Query<&Control>,
    mut owner: NonSendMut<HostOwner>,
    mut state: ResMut<OptionsState>,
    guard: ModalGuard,
) {
    if !guard.allows(event.entity)
        || owner.preferences_draft().is_none()
        || state.generation != Some(owner.generation())
        || top_server_dialog(&owner)
    {
        return;
    }
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    match *control {
        Control::Page(page) => {
            state.page = page;
            state.choice = None;
        }
        Control::Choice(id) => {
            state.choice = if state.choice == Some(id) {
                None
            } else {
                Some(id)
            }
        }
        Control::Select(id, index) => {
            let generation = owner.generation();
            commands::edit(&mut owner, generation, state.page, id, Value::Choice(index));
            state.choice = None;
        }
        Control::Reset => {
            let defaults = crate::play::ReferencePreferences::default();
            owner.edit_preferences(|draft| {
                for row in metadata::rows(state.page) {
                    row.reset(draft, &defaults);
                }
            });
        }
        Control::Apply => owner.apply(false),
        Control::Ok => owner.apply(true),
        Control::Cancel => owner.cancel(),
        _ => {}
    }
}
fn toggle(
    event: On<bevy::ui_widgets::ValueChange<bool>>,
    controls: Query<&Control>,
    mut owner: NonSendMut<HostOwner>,
    mut state: ResMut<OptionsState>,
    guard: ModalGuard,
) {
    if !guard.allows(event.entity)
        || owner.preferences_draft().is_none()
        || state.generation != Some(owner.generation())
        || top_server_dialog(&owner)
    {
        return;
    }
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    match *control {
        Control::Advanced => {
            state.advanced = event.value;
            state.page = if state.advanced {
                Page::Controls
            } else {
                Page::Basic
            };
            state.choice = None;
        }
        Control::Check(id) => {
            let generation = owner.generation();
            commands::edit(
                &mut owner,
                generation,
                state.page,
                id,
                Value::Check(event.value),
            );
        }
        _ => {}
    }
}
fn slide(
    event: On<bevy::ui_widgets::ValueChange<f32>>,
    controls: Query<&Control>,
    mut owner: NonSendMut<HostOwner>,
    state: Res<OptionsState>,
    guard: ModalGuard,
) {
    if !guard.allows(event.entity)
        || owner.preferences_draft().is_none()
        || state.generation != Some(owner.generation())
        || top_server_dialog(&owner)
        || !event.value.is_finite()
    {
        return;
    }
    let Ok(Control::Number(id)) = controls.get(event.entity) else {
        return;
    };
    let Some(row) = metadata::find(state.page, id) else {
        return;
    };
    let Editor::Number { min, max, .. } = row.editor else {
        return;
    };
    let value = event.value.round().clamp(f32::from(min), f32::from(max)) as u16;
    let generation = owner.generation();
    commands::edit(&mut owner, generation, state.page, id, Value::Number(value));
}
