//! Exaltation Forge presentation over capability 152's retained owner and server previews.
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use crate::play::feature_0152_item_forge::view::{ForgeControl, ForgeSnapshot};
use bevy::{
    input_focus::{FocusCause, InputFocus},
    prelude::*,
    ui_widgets::Activate,
};
use oteryn_client_runtime::cui3::item_forge::Operation;
use oteryn_client_ui::item_forge::ItemForgePanel as Panel;
#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    generation: u64,
    revision: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Owner(ForgeControl),
    Review,
    Confirm,
    Cancel,
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
    snapshot: Option<(Key, ForgeSnapshot)>,
    revision: u64,
    review: Option<Key>,
    dirty: bool,
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
    let snapshot = owner
        .play
        .as_ref()
        .map(|play| play.link.item_forge_snapshot());
    let Some(snapshot) = snapshot.filter(|s| s.panel.open) else {
        if let Some(widgets) = state.widgets.take() {
            if inside(focus.get(), widgets.overlay, &parents) {
                focus.clear();
            }
            commands.entity(widgets.overlay).despawn();
        }
        state.snapshot = None;
        state.review = None;
        return;
    };
    let unchanged = state
        .snapshot
        .as_ref()
        .is_some_and(|(key, old)| key.generation == owner.generation() && old == &snapshot);
    if unchanged && !state.dirty {
        return;
    }
    if !unchanged {
        let Some(revision) = state.revision.checked_add(1) else {
            return;
        };
        state.revision = revision;
        state.review = None;
    }
    state.dirty = false;
    let key = Key {
        generation: owner.generation(),
        revision: state.revision,
    };
    let reviewing = state.review == Some(key);
    let widgets = state
        .widgets
        .get_or_insert_with(|| widgets::build(&mut commands, &skins));
    if widgets
        .content
        .is_some_and(|content| inside(focus.get(), content, &parents))
    {
        focus.set(widgets.panel, FocusCause::Navigated);
    }
    widgets::render(
        &mut commands,
        &skins,
        widgets,
        key,
        &snapshot.panel,
        reviewing,
    );
    state.snapshot = Some((key, snapshot));
}
fn perform(owner: &HostOwner, state: &mut State, control: &Control) {
    let Some((key, expected)) = &state.snapshot else {
        return;
    };
    if control.key != *key
        || key.generation != owner.generation()
        || owner.modal_active()
        || owner.phase != crate::bevy_shell::owner::Phase::Playing
    {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    let action = match control.kind {
        Kind::Owner(action) => action,
        Kind::Review => {
            if play
                .link
                .item_forge_dispatch_bound(expected, ForgeControl::Review)
            {
                state.review = Some(*key);
                state.dirty = true;
            }
            return;
        }
        Kind::Confirm if state.review == Some(*key) => ForgeControl::Forge,
        Kind::Cancel => {
            state.review = None;
            state.dirty = true;
            return;
        }
        Kind::Confirm => return,
    };
    if play.link.item_forge_dispatch_bound(expected, action) {
        state.review = None;
        state.dirty = true;
    }
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    if guard.allows(event.entity)
        && let Ok(control) = controls.get(event.entity)
    {
        perform(&owner, &mut state, control);
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
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
        let control = Control {
            key: *key,
            kind: Kind::Owner(ForgeControl::Close),
        };
        perform(&owner, &mut state, &control);
    }
}
