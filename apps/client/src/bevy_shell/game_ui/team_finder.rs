//! Team Finder presentation over the retained discovery owners.
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use crate::play::feature_0232_team_finder_members::input::{
    TeamFinderAction as Action, TeamFinderSnapshot as Snapshot,
};
use bevy::{input_focus::InputFocus, prelude::*, text::EditableText, ui_widgets::Activate};
#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    generation: u64,
    revision: u64,
}
#[derive(Component, Clone)]
struct Control {
    key: Key,
    action: Action,
}
#[derive(Component, Clone, Copy)]
enum FormAction {
    Publish,
    Filter,
    Clear,
    Confirm,
    Cancel,
}
#[derive(Component, Clone, Copy)]
struct FormControl {
    key: Key,
    action: FormAction,
}
#[derive(Component)]
struct Field(usize);
#[derive(Component)]
struct Overlay;
#[derive(Resource, Default)]
struct State {
    snapshot: Option<(Key, Snapshot)>,
    revision: u64,
    root: Option<Entity>,
    fields: Vec<String>,
    confirmation: Option<Action>,
    dirty: bool,
    pending_form: Option<(Entity, FormControl)>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (read_fields, keyboard, project).chain())
        .add_systems(
            PostUpdate,
            finish_form.after(bevy::text::EditableTextSystems),
        );
}
pub(in crate::bevy_shell) fn open(owner: &mut HostOwner) {
    if !owner.modal_active()
        && let Some(play) = &owner.play
        && !play.view.gameplay_input_blocked(&play.link)
    {
        let snapshot = play.link.team_finder_snapshot();
        if snapshot.teams.open {
            let _ = play
                .link
                .team_finder_dispatch_bound(&snapshot, Action::Close, false);
        } else {
            play.link.discovery_open(232);
        }
    }
}
fn panel_text(snapshot: &Snapshot) -> String {
    format!(
        "{}\r\n{}",
        oteryn_client_ui::team_finder::panel_text(&snapshot.teams),
        if snapshot.teams.applicants {
            oteryn_client_ui::team_finder_leader::view::panel_text(&snapshot.leader)
        } else {
            String::new()
        }
    )
}
fn read_fields(fields: Query<(&Field, &EditableText)>, mut state: ResMut<State>) {
    if state.fields.len() != 15 {
        state.fields.resize(15, String::new());
    }
    for (field, text) in &fields {
        state.fields[field.0] = text.value().to_string();
    }
}
fn inside(mut entity: Option<Entity>, root: Entity, parents: &Query<&ChildOf>) -> bool {
    while let Some(current) = entity {
        if current == root {
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
    let current = owner
        .play
        .as_ref()
        .map(|play| play.link.team_finder_snapshot());
    let Some(current) = current.filter(|snapshot| snapshot.teams.open) else {
        if let Some(root) = state.root.take() {
            if inside(focus.get(), root, &parents) {
                focus.clear();
            }
            commands.entity(root).despawn();
        }
        state.snapshot = None;
        state.confirmation = None;
        state.pending_form = None;
        state.fields.clear();
        return;
    };
    let changed = state
        .snapshot
        .as_ref()
        .is_none_or(|(key, old)| key.generation != owner.generation() || old != &current);
    if !changed && !state.dirty {
        return;
    }
    if changed {
        state.pending_form = None;
        state.confirmation = None;
        if state
            .snapshot
            .as_ref()
            .is_some_and(|(key, _)| key.generation != owner.generation())
        {
            state.fields.fill(String::new());
        }
    }
    state.dirty = false;
    state.revision = state.revision.wrapping_add(1);
    let key = Key {
        generation: owner.generation(),
        revision: state.revision,
    };
    if let Some(root) = state.root.take() {
        if inside(focus.get(), root, &parents) {
            focus.clear();
        }
        commands.entity(root).despawn();
    }
    state.root = Some(widgets::build(&mut commands, &skins, key, &current, &state));
    state.snapshot = Some((key, current));
}
fn valid(owner: &HostOwner, state: &State, key: Key) -> bool {
    key.generation == owner.generation()
        && !owner.modal_active()
        && state
            .snapshot
            .as_ref()
            .is_some_and(|(current, _)| *current == key)
        && owner
            .play
            .as_ref()
            .is_some_and(|play| !play.view.gameplay_input_blocked(&play.link))
}
fn dispatch(owner: &HostOwner, state: &mut State, key: Key, action: Action, confirmed: bool) {
    if !valid(owner, state, key) {
        return;
    }
    let Some((_, snapshot)) = &state.snapshot else {
        return;
    };
    let Some(play) = &owner.play else {
        return;
    };
    if play.link.team_finder_snapshot() != *snapshot {
        return;
    }
    if action.needs_confirmation() && !confirmed {
        state.confirmation = Some(action);
        state.dirty = true;
        return;
    }
    if play
        .link
        .team_finder_dispatch_bound(snapshot, action, confirmed)
    {
        state.confirmation = None;
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
        && state.confirmation.is_none()
        && let Ok(control) = controls.get(event.entity)
    {
        dispatch(
            &owner,
            &mut state,
            control.key,
            control.action.clone(),
            false,
        );
    }
}
fn form_activate(
    event: On<Activate>,
    controls: Query<&FormControl>,
    fields: Query<(&Field, &EditableText)>,
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
    if !valid(&owner, &state, control.key) {
        return;
    }
    if matches!(control.action, FormAction::Publish | FormAction::Filter)
        && fields.iter().any(|(_, text)| editing(text))
    {
        if state.pending_form.is_none() {
            state.pending_form = Some((event.entity, *control));
        }
        return;
    }
    for (field, text) in &fields {
        state.fields[field.0] = text.value().to_string();
    }
    submit_form(&owner, &mut state, control);
}
fn editing(text: &EditableText) -> bool {
    text.pending_paste.is_some()
        || text
            .pending_edits
            .iter()
            .any(bevy::text::TextEdit::is_destructive)
}
fn finish_form(
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    fields: Query<(&Field, &EditableText)>,
    controls: Query<&FormControl>,
    guard: ModalGuard,
) {
    let Some((source, control)) = state.pending_form.take() else {
        return;
    };
    if !guard.allows(source)
        || !valid(&owner, &state, control.key)
        || !controls
            .get(source)
            .is_ok_and(|current| current.key == control.key)
    {
        return;
    }
    if fields.iter().any(|(_, text)| editing(text)) {
        state.pending_form = Some((source, control));
        return;
    }
    for (field, text) in &fields {
        state.fields[field.0] = text.value().to_string();
    }
    submit_form(&owner, &mut state, &control);
}
fn submit_form(owner: &HostOwner, state: &mut State, control: &FormControl) {
    let action = match control.action {
        FormAction::Cancel => {
            state.confirmation = None;
            state.dirty = true;
            return;
        }
        FormAction::Confirm => {
            if let Some(action) = state.confirmation.clone() {
                dispatch(owner, state, control.key, action, true);
                state.dirty = true;
            }
            return;
        }
        FormAction::Clear => Some(Action::Filter(Default::default())),
        FormAction::Filter => filter(&state.fields).map(Action::Filter),
        FormAction::Publish => {
            listing(&state.fields).map(|listing| Action::Advertise(Some(listing)))
        }
    };
    if state.confirmation.is_none()
        && let Some(action) = action
    {
        dispatch(owner, state, control.key, action, false);
    }
}
fn filter(fields: &[String]) -> Option<oteryn_client_ui::team_finder::TeamFilter> {
    let level: u16 = fields.get(11)?.trim().parse().ok()?;
    let open: u8 = fields.get(14)?.trim().parse().ok()?;
    let target: u8 = fields.get(13)?.trim().parse().ok()?;
    if open > 1 || target > 3 {
        return None;
    }
    Some(oteryn_client_ui::team_finder::TeamFilter {
        name: fields.get(10)?.clone(),
        level: (level != 0).then_some(level),
        vocation_mask: fields.get(12)?.trim().parse().ok()?,
        target,
        only_open: open == 1,
    })
}
fn listing(fields: &[String]) -> Option<oteryn_client_ui::team_finder_leader::LeaderListing> {
    use oteryn_client_ui::team_finder_leader::{LeaderListing, TeamTarget};
    let parse = |i: usize| fields.get(i)?.trim().parse::<u16>().ok();
    Some(LeaderListing {
        min_level: parse(0)?,
        max_level: parse(1)?,
        vocation_mask: u8::try_from(parse(2)?).ok()?,
        slots: parse(3)?,
        free_slots: parse(4)?,
        party_members_join: match parse(5)? {
            0 => false,
            1 => true,
            _ => return None,
        },
        timestamp: fields.get(9)?.trim().parse().ok()?,
        target: match parse(6)? {
            1 => TeamTarget::Boss(parse(7)?),
            2 => TeamTarget::Hunt {
                kind: parse(7)?,
                area: parse(8)?,
            },
            3 => TeamTarget::Quest(parse(7)?),
            _ => return None,
        },
    })
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    for event in events.read() {
        if event.key_code == KeyCode::Escape
            && event.state == bevy::input::ButtonState::Pressed
            && !event.repeat
            && let Some(root) = state.root
            && guard.allows(root)
            && let Some((key, _)) = &state.snapshot
        {
            let key = *key;
            if state.confirmation.take().is_some() {
                state.dirty = true;
            } else {
                dispatch(&owner, &mut state, key, Action::Close, false);
            }
        }
    }
}
