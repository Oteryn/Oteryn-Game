//! Source-shaped Manage Loot Containers over confirmed capability-98 state.
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use crate::play::feature_0098_quick_loot::view::QuickLootControl as Kind;
use bevy::{
    input_focus::{FocusCause, InputFocus},
    prelude::*,
    ui_widgets::Activate,
};
use oteryn_client_ui::quick_loot::{
    Action, Panel, QuickLootCategory as Category, QuickLootFilterMode as Mode,
    QuickLootRecipient as Recipient,
};
#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    generation: u64,
    revision: u64,
}
#[derive(Component, Clone)]
struct Control {
    key: Key,
    kind: Kind,
}
#[derive(Component)]
struct Overlay;
#[derive(Resource, Default)]
struct State {
    snapshot: Option<(Key, Panel)>,
    revision: u64,
    widgets: Option<widgets::Widgets>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain());
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
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    mut focus: ResMut<InputFocus>,
    parents: Query<&ChildOf>,
) {
    let panel = owner.play.as_ref().map(|play| play.link.quick_loot_panel());
    let Some(panel) = panel.filter(|panel| panel.open) else {
        if let Some(widgets) = state.widgets.take() {
            if inside(focus.get(), widgets.overlay, &parents) {
                focus.clear();
            }
            commands.entity(widgets.overlay).despawn();
        }
        state.snapshot = None;
        return;
    };
    if state
        .snapshot
        .as_ref()
        .is_some_and(|(key, old)| key.generation == owner.generation() && old == &panel)
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
        .is_some_and(|(old, _)| old.generation != key.generation)
        && let Some(widgets) = state.widgets.take()
    {
        if inside(focus.get(), widgets.overlay, &parents) {
            focus.clear();
        }
        commands.entity(widgets.overlay).despawn();
    }
    let widgets = state
        .widgets
        .get_or_insert_with(|| widgets::build(&mut commands, &skins));
    if widgets
        .content
        .is_some_and(|content| inside(focus.get(), content, &parents))
    {
        focus.set(widgets.panel, FocusCause::Navigated);
    }
    widgets::render(&mut commands, &skins, widgets, key, &panel);
    state.snapshot = Some((key, panel));
}
fn perform(owner: &HostOwner, state: &State, control: &Control) {
    let Some((key, expected)) = &state.snapshot else {
        return;
    };
    if control.key != *key || key.generation != owner.generation() || owner.modal_active() {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if control.kind != Kind::Close && play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    let _ = play
        .link
        .quick_loot_dispatch_bound(expected, control.kind.clone());
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    state: Res<State>,
) {
    if guard.allows(event.entity)
        && let Ok(control) = controls.get(event.entity)
    {
        perform(&owner, &state, control);
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    state: Res<State>,
) {
    let Some(widgets) = &state.widgets else {
        events.clear();
        return;
    };
    if !guard.allows(widgets.panel) {
        events.clear();
        return;
    }
    if events.read().any(|event| {
        event.key_code == KeyCode::Escape
            && event.state == bevy::input::ButtonState::Pressed
            && !event.repeat
    }) && let Some((key, _)) = &state.snapshot
    {
        perform(
            &owner,
            &state,
            &Control {
                key: *key,
                kind: Kind::Close,
            },
        );
    }
}
