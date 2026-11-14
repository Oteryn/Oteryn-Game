//! Source-shaped inspection over the retained capability-132 owner.
mod widgets;
use super::super::{owner::HostOwner, ui::Skins};
use super::{GameModal, modal::ModalGuard};
use bevy::{clipboard::Clipboard, input_focus::InputFocus, prelude::*, ui_widgets::Activate};
use oteryn_client_ui::inspection::{Panel, Request};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Key {
    generation: u64,
    revision: u64,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Catalogue,
    Inspect,
    Refresh,
    Copy,
    Close,
}
#[derive(Component, Clone, Copy)]
struct Control {
    key: Key,
    kind: Kind,
}
#[derive(Component)]
struct Overlay;
#[derive(Component)]
struct ClipboardStatus;
#[derive(Resource, Default)]
struct State {
    widgets: Option<widgets::Widgets>,
    snapshot: Option<(Key, Panel, Option<oteryn_session::ItemHandle>)>,
    revision: u64,
    clipboard_feedback: String,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .init_resource::<Clipboard>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project, clipboard_status).chain());
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skin: Res<Skins>,
    mut state: ResMut<State>,
    mut focus: ResMut<InputFocus>,
    parents: Query<&ChildOf>,
) {
    let model = owner.play.as_ref().map(|play| play.link.inspection_panel());
    let Some(model) = model.filter(|model| model.open) else {
        if let Some(widgets) = state.widgets.take() {
            if inside(focus.get(), widgets.overlay, &parents) {
                focus.clear();
            }
            commands.entity(widgets.overlay).despawn();
        }
        state.snapshot = None;
        return;
    };
    let selection = owner.play.as_ref().and_then(|play| {
        play.link
            .inventory_panel()
            .selected_item()
            .map(|item| item.handle)
    });
    if state
        .snapshot
        .as_ref()
        .is_some_and(|(key, previous, item)| {
            key.generation == owner.generation() && previous == &model && *item == selection
        })
    {
        return;
    }
    let Some(revision) = state.revision.checked_add(1) else {
        return;
    };
    state.revision = revision;
    let key = Key {
        generation: owner.generation(),
        revision,
    };
    if state
        .snapshot
        .as_ref()
        .is_some_and(|(old, _, _)| old.generation != key.generation)
        && let Some(widgets) = state.widgets.take()
    {
        if inside(focus.get(), widgets.overlay, &parents) {
            focus.clear();
        }
        commands.entity(widgets.overlay).despawn();
    }
    let widgets = state
        .widgets
        .get_or_insert_with(|| widgets::build(&mut commands, &skin));
    if widgets
        .content
        .is_some_and(|content| inside(focus.get(), content, &parents))
    {
        focus.set(widgets.panel, bevy::input_focus::FocusCause::Navigated);
    }
    widgets::render(
        &mut commands,
        &skin,
        widgets,
        key,
        &model,
        selection.is_some(),
    );
    state.snapshot = Some((key, model, selection));
    state.clipboard_feedback.clear();
}
fn inside(mut entity: Option<Entity>, ancestor: Entity, parents: &Query<&ChildOf>) -> bool {
    while let Some(current) = entity {
        if current == ancestor {
            return true;
        }
        entity = parents.get(current).ok().map(ChildOf::parent);
    }
    false
}
fn current(owner: &HostOwner, state: &State, key: Key) -> bool {
    !owner.modal_active()
        && key.generation == owner.generation()
        && state
            .snapshot
            .as_ref()
            .is_some_and(|(current, model, item)| {
                *current == key
                    && owner.play.as_ref().is_some_and(|play| {
                        play.link.inspection_panel() == *model
                            && play
                                .link
                                .inventory_panel()
                                .selected_item()
                                .map(|item| item.handle)
                                == *item
                    })
            })
}
fn read_request(request: Request) -> bool {
    !matches!(request, Request::Permission { .. })
}
fn perform(owner: &HostOwner, state: &mut State, clipboard: &mut Clipboard, control: Control) {
    if !current(owner, state, control.key) {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    let panel = play.link.inspection_panel();
    match control.kind {
        Kind::Close => {
            play.link.inspection_set_open(false);
            state.snapshot = None;
        }
        Kind::Catalogue => super::item_catalogue::open(owner),
        Kind::Inspect if panel.selected && !panel.pending => {
            play.link.discovery_action(132, 1, &[], 0);
        }
        Kind::Refresh if panel.selected && !panel.pending => {
            if let Some(request) = panel.last_request.filter(|request| read_request(*request)) {
                play.link.inspect(request);
            }
        }
        Kind::Copy => {
            if let Some(text) = copy_text(&panel) {
                state.clipboard_feedback = if clipboard.set_text(text).is_ok() {
                    "Copied to clipboard"
                } else {
                    "Clipboard unavailable"
                }
                .into();
            }
        }
        _ => {}
    }
}
fn activate(
    event: On<Activate>,
    guard: ModalGuard,
    controls: Query<&Control>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    mut clipboard: ResMut<Clipboard>,
) {
    if guard.allows(event.entity)
        && let Ok(control) = controls.get(event.entity)
    {
        perform(&owner, &mut state, &mut clipboard, *control);
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    guard: ModalGuard,
    focus: Res<InputFocus>,
    controls: Query<(), With<Control>>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    mut clipboard: ResMut<Clipboard>,
) {
    let Some(panel) = state.widgets.as_ref().map(|widgets| widgets.panel) else {
        events.clear();
        return;
    };
    if !guard.allows(panel) || owner.modal_active() {
        events.clear();
        return;
    }
    for event in events.read() {
        if event.repeat || event.state != bevy::input::ButtonState::Pressed {
            continue;
        }
        let close = event.key_code == KeyCode::Escape
            || (matches!(event.key_code, KeyCode::Enter | KeyCode::NumpadEnter)
                && !focus.get().is_some_and(|entity| controls.contains(entity)));
        if close && let Some((key, _, _)) = &state.snapshot {
            let control = Control {
                key: *key,
                kind: Kind::Close,
            };
            perform(&owner, &mut state, &mut clipboard, control);
        }
    }
}
fn clipboard_status(state: Res<State>, mut labels: Query<&mut Text, With<ClipboardStatus>>) {
    for mut label in &mut labels {
        if label.0 != state.clipboard_feedback {
            label.0.clone_from(&state.clipboard_feedback);
        }
    }
}
fn item_text(item: &oteryn_client_ui::inspection::InspectedItem) -> String {
    let mut lines = vec![
        format!("Count: {}", item.count),
        format!("Tier: {}", item.tier),
    ];
    lines.extend(
        item.attributes
            .iter()
            .map(|(key, value)| format!("{key}: {value}")),
    );
    lines.join("\n")
}
fn copy_text(panel: &Panel) -> Option<String> {
    let reply = panel
        .reply
        .as_ref()
        .filter(|reply| reply.status == 1 && !reply.items.is_empty())?;
    Some(
        reply
            .items
            .iter()
            .map(|item| format!("{}\n{}", item.name, item_text(item)))
            .collect::<Vec<_>>()
            .join("\n\n"),
    )
}
#[cfg(test)]
mod tests;
