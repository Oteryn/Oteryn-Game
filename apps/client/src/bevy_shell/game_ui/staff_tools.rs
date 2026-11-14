//! Signed staff envelopes projected from the retained staff owner.
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use crate::play::feature_0051_staff::view::StaffToolsSnapshot;
use bevy::{input_focus::InputFocus, prelude::*, text::EditableText, ui_widgets::Activate};
use oteryn_client_ui::staff::Panel;

#[derive(Component)]
struct Overlay;
#[derive(Component)]
struct Field(usize);
#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    generation: u64,
    revision: u64,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Submit(bool),
    Confirm,
    Cancel,
    Close,
}
#[derive(Component, Clone, Copy)]
struct Control {
    key: Key,
    kind: Kind,
}
#[derive(Resource, Default)]
struct State {
    snapshot: Option<StaffToolsSnapshot>,
    key: Option<Key>,
    revision: u64,
    overlay: Option<Entity>,
    confirmation: Option<(StaffToolsSnapshot, bool)>,
    rendered_confirmation: bool,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain());
}
pub(in crate::bevy_shell) fn open(owner: &mut HostOwner) {
    if !owner.modal_active()
        && let Some(play) = &owner.play
    {
        let _ = play.link.staff_tools_open_bound();
    }
}
pub(in crate::bevy_shell) fn toolbar_visible(owner: &HostOwner) -> bool {
    owner
        .play
        .as_ref()
        .is_some_and(|play| play.link.staff_panel().selected)
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
        .map(|play| play.link.staff_tools_snapshot());
    let snapshot = snapshot.filter(|snapshot| snapshot.panel.open && snapshot.panel.selected);
    if state.confirmation.as_ref().is_some_and(|(expected, _)| {
        snapshot
            .as_ref()
            .is_none_or(|current| !expected.matches(current))
    }) {
        state.confirmation = None;
    }
    let confirming = state.confirmation.is_some();
    if snapshot.as_ref().is_some_and(|current| {
        state
            .snapshot
            .as_ref()
            .is_some_and(|old| old.matches(current))
    }) && state
        .key
        .is_some_and(|key| key.generation == owner.generation())
        && confirming == state.rendered_confirmation
    {
        return;
    }
    if let Some(overlay) = state.overlay.take() {
        if inside(focus.get(), overlay, &parents) {
            focus.clear();
        }
        commands.entity(overlay).despawn();
    }
    state.snapshot = snapshot;
    state.key = None;
    if state.snapshot.is_none() {
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
    state.key = Some(key);
    state.rendered_confirmation = confirming;
    if let Some(snapshot) = &state.snapshot {
        state.overlay = Some(widgets::build(
            &mut commands,
            &skins,
            key,
            &snapshot.panel,
            state.confirmation.as_ref().map(|(_, receipt)| *receipt),
        ));
    }
}
fn perform(
    owner: &HostOwner,
    state: &mut State,
    control: Control,
    fields: &Query<(&Field, &EditableText)>,
) {
    if state.key != Some(control.key)
        || owner.generation() != control.key.generation
        || owner.modal_active()
    {
        return;
    }
    let (Some(play), Some(expected)) = (&owner.play, &state.snapshot) else {
        return;
    };
    if !play.link.staff_tools_snapshot().matches(expected) {
        return;
    }
    match control.kind {
        Kind::Close => {
            let _ = play.link.staff_tools_close_bound(expected);
        }
        Kind::Cancel => {
            state.confirmation = None;
        }
        Kind::Submit(receipt) => {
            if state.confirmation.is_some() || expected.panel.pending {
                return;
            }
            let mut values: [String; 3] = Default::default();
            let mut seen = [false; 3];
            for (field, text) in fields.iter() {
                if field.0 >= 3
                    || seen[field.0]
                    || text.pending_paste.is_some()
                    || text
                        .pending_edits
                        .iter()
                        .any(bevy::text::TextEdit::is_destructive)
                {
                    return;
                }
                seen[field.0] = true;
                values[field.0] = text.value().to_string();
            }
            if seen != [true; 3] {
                return;
            }
            if play.link.staff_tools_edit_bound(expected, values) {
                state.confirmation = Some((play.link.staff_tools_snapshot(), receipt));
            }
        }
        Kind::Confirm => {
            if let Some((confirmed, receipt)) = &state.confirmation {
                let _ = play
                    .link
                    .staff_tools_submit_bound(confirmed, *receipt, true);
            }
            state.confirmation = None;
        }
    }
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    fields: Query<(&Field, &EditableText)>,
) {
    if guard.allows(event.entity)
        && let Ok(control) = controls.get(event.entity)
    {
        perform(&owner, &mut state, *control, &fields);
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    focus: Res<InputFocus>,
    fields: Query<(&Field, &EditableText)>,
) {
    for event in events.read() {
        if event.state != bevy::input::ButtonState::Pressed || event.repeat {
            continue;
        }
        if !guard.allows_focus(state.overlay.or(focus.get())) {
            continue;
        }
        if event.key_code == KeyCode::Escape
            && let Some(key) = state.key
        {
            let kind = if state.confirmation.is_some() {
                Kind::Cancel
            } else {
                Kind::Close
            };
            perform(&owner, &mut state, Control { key, kind }, &fields);
        }
    }
}
