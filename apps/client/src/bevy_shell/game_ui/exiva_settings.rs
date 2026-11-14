//! Exiva privacy/list editor over the retained owner and its confirmed server list.
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use crate::play::feature_0147_exiva_settings::view::{ExivaControl, ExivaSnapshot};
use bevy::{input_focus::InputFocus, prelude::*, text::EditableText, ui_widgets::Activate};
#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    generation: u64,
    revision: u64,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Select([u8; 16]),
    Toggle(u8),
    Add,
    Remove,
    ConfirmRemove,
    CancelRemove,
    Refresh,
    Close,
}
#[derive(Component, Clone, Copy)]
struct Control {
    key: Key,
    kind: Kind,
}
#[derive(Component)]
struct NameField;
#[derive(Resource, Default)]
struct State {
    snapshot: Option<(Key, ExivaSnapshot)>,
    revision: u64,
    selected: Option<[u8; 16]>,
    confirmation: Option<[u8; 16]>,
    // Consume a displayed control immediately, including when its reply arrives before projection.
    used: bool,
    pending_add: Option<(Control, Entity, Entity)>,
    widgets: Option<widgets::Widgets>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain())
        .add_systems(
            PostUpdate,
            finish_add.after(bevy::text::EditableTextSystems),
        );
}
pub(in crate::bevy_shell) fn open_exiva_settings(owner: &HostOwner) {
    if !available(owner) {
        return;
    }
    if let Some(play) = &owner.play {
        let snapshot = play.link.exiva_snapshot();
        let _ = play
            .link
            .exiva_dispatch_bound(&snapshot, ExivaControl::Open, false);
    }
}
fn available(owner: &HostOwner) -> bool {
    owner.phase == crate::bevy_shell::owner::Phase::Playing
        && !owner.modal_active()
        && owner
            .play
            .as_ref()
            .is_some_and(|play| !play.view.gameplay_input_blocked(&play.link))
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
    fields: Query<&EditableText, With<NameField>>,
) {
    let snapshot = owner.play.as_ref().map(|play| play.link.exiva_snapshot());
    let Some(snapshot) = snapshot.filter(|s| s.live && s.panel.open) else {
        if let Some(widgets) = state.widgets.take() {
            if inside(focus.get(), widgets.overlay, &parents) {
                focus.clear();
            }
            commands.entity(widgets.overlay).despawn();
        }
        state.snapshot = None;
        state.pending_add = None;
        state.selected = None;
        state.confirmation = None;
        state.used = false;
        return;
    };
    if !state.used
        && state
            .snapshot
            .as_ref()
            .is_some_and(|(key, old)| key.generation == owner.generation() && *old == snapshot)
    {
        return;
    }
    let replaced = state.snapshot.as_ref().is_some_and(|(key, old)| {
        key.generation != owner.generation() || !old.same_session(&snapshot)
    });
    let changed = state
        .snapshot
        .as_ref()
        .is_some_and(|(_, old)| old != &snapshot);
    if changed {
        state.pending_add = None;
        state.confirmation = None;
    }
    if replaced {
        state.selected = None;
        if let Some(widgets) = state.widgets.take() {
            if inside(focus.get(), widgets.overlay, &parents) {
                focus.clear();
            }
            commands.entity(widgets.overlay).despawn();
        }
    }
    state.selected = state.selected.filter(|id| {
        snapshot
            .panel
            .list
            .as_ref()
            .is_some_and(|list| list.players.iter().any(|(held, _)| held == id))
    });
    let Some(revision) = state.revision.checked_add(1) else {
        return;
    };
    state.revision = revision;
    let key = Key {
        generation: owner.generation(),
        revision,
    };
    let draft = if replaced {
        String::new()
    } else {
        fields
            .iter()
            .next()
            .map(|field| field.value().to_string())
            .unwrap_or_default()
    };
    let selected = state.selected;
    let confirmation = state.confirmation;
    let widgets = state
        .widgets
        .get_or_insert_with(|| widgets::build(&mut commands, &skins));
    if widgets
        .content
        .is_some_and(|content| inside(focus.get(), content, &parents))
    {
        focus.clear();
    }
    widgets::render(
        &mut commands,
        &skins,
        widgets,
        key,
        &snapshot.panel,
        selected,
        confirmation,
        &draft,
    );
    state.snapshot = Some((key, snapshot));
    state.used = false;
}
fn perform(owner: &HostOwner, state: &mut State, control: &Control, name: Option<String>) {
    let Some((key, expected)) = &state.snapshot else {
        return;
    };
    if state.used
        || control.key != *key
        || key.generation != owner.generation()
        || !available(owner)
    {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if play.link.exiva_snapshot() != *expected {
        return;
    }
    let request = match control.kind {
        Kind::Close => ExivaControl::Close,
        Kind::Refresh if state.confirmation.is_none() => ExivaControl::Refresh,
        Kind::Toggle(bit) if state.confirmation.is_none() => ExivaControl::Toggle(bit),
        Kind::Add if state.confirmation.is_none() => {
            let Some(name) = name else {
                return;
            };
            ExivaControl::AddName(name)
        }
        Kind::Select(id) if state.confirmation.is_none() && !expected.panel.pending => {
            if !expected
                .panel
                .list
                .as_ref()
                .is_some_and(|list| list.players.iter().any(|(held, _)| *held == id))
            {
                return;
            }
            state.selected = Some(id);
            state.used = true;
            return;
        }
        Kind::Remove if !expected.panel.pending && state.confirmation.is_none() => {
            let Some(id) = state.selected else {
                return;
            };
            state.confirmation = Some(id);
            state.used = true;
            return;
        }
        Kind::CancelRemove => {
            state.confirmation = None;
            state.used = true;
            return;
        }
        Kind::ConfirmRemove => {
            let Some(id) = state.confirmation.filter(|id| Some(*id) == state.selected) else {
                return;
            };
            ExivaControl::Remove(id)
        }
        _ => return,
    };
    if play
        .link
        .exiva_dispatch_bound(expected, request, control.kind == Kind::ConfirmRemove)
    {
        state.used = true;
        state.confirmation = None;
    }
}
fn editing(field: &EditableText) -> bool {
    field.pending_paste.is_some()
        || field
            .pending_edits
            .iter()
            .any(bevy::text::TextEdit::is_destructive)
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    fields: Query<(Entity, &EditableText), With<NameField>>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    if !guard.allows(event.entity) {
        return;
    }
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if control.kind == Kind::Add {
        if state.pending_add.is_some() {
            return;
        }
        let Ok((entity, field)) = fields.single() else {
            return;
        };
        if !guard.allows(entity) {
            return;
        }
        if editing(field) {
            state.pending_add = Some((*control, event.entity, entity));
            return;
        }
        perform(&owner, &mut state, control, Some(field.value().to_string()));
    } else {
        perform(&owner, &mut state, control, None);
    }
}
fn finish_add(
    fields: Query<&EditableText, With<NameField>>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    let Some((control, button, entity)) = state.pending_add else {
        return;
    };
    if !available(&owner) || !guard.allows(button) || !guard.allows(entity) {
        state.pending_add = None;
        return;
    }
    let Ok(field) = fields.get(entity) else {
        state.pending_add = None;
        return;
    };
    if editing(field) {
        return;
    }
    state.pending_add = None;
    perform(
        &owner,
        &mut state,
        &control,
        Some(field.value().to_string()),
    );
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    for event in events
        .read()
        .filter(|event| event.state == bevy::input::ButtonState::Pressed && !event.repeat)
    {
        if event.key_code == KeyCode::Escape
            && state
                .widgets
                .as_ref()
                .is_some_and(|widgets| guard.allows(widgets.panel))
            && let Some((key, _)) = &state.snapshot
        {
            let kind = if state.confirmation.is_some() {
                Kind::CancelRemove
            } else {
                Kind::Close
            };
            let control = Control { key: *key, kind };
            perform(&owner, &mut state, &control, None);
        }
    }
}
