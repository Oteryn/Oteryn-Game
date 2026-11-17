//! Join Team and Manage Team over the negotiated Team Finder member and leader streams.
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{
        owner::{HostOwner, Phase},
        ui::Skins,
    },
    GameModal,
    modal::ModalGuard,
};
use crate::play::feature_0232_team_finder_members::input::{
    TeamControl as Kind, listing_from_fields,
};
use bevy::{input_focus::InputFocus, prelude::*, text::EditableText, ui_widgets::Activate};
use oteryn_client_ui::{
    team_finder::Panel,
    team_finder_leader::{LeaderListing, Panel as Leader},
};
#[derive(Clone, Copy, Default, PartialEq, Eq)]
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
enum Field {
    Name,
    Level,
    /// One leader listing field, in `listing_from_fields` order.
    Listing(usize),
}
#[derive(Resource, Default)]
struct State {
    snapshot: Option<(u64, Panel, Leader)>,
    search: [String; 2],
    submission: Option<Submission>,
    pending_editor_action: Option<Entity>,
    root: Option<Entity>,
    key: Key,
    /// The listing values the editors were last built with.
    shown: [String; widgets::LISTING_FIELDS],
    /// The leader's unpublished listing edits, kept across rebuilds until a reply confirms them.
    draft: Option<[String; widgets::LISTING_FIELDS]>,
}
struct Submission {
    fields: [String; widgets::LISTING_FIELDS],
    listing: Option<LeaderListing>,
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, keyboard)
        .add_systems(
            PostUpdate,
            (finish_edit, project)
                .chain()
                .after(bevy::text::EditableTextSystems)
                .before(bevy::ui::UiSystems::PostLayout)
                .before(bevy::ui::UiSystems::Stack),
        );
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    mut focus: ResMut<InputFocus>,
    parents: Query<&ChildOf>,
    fields: Query<(&Field, &EditableText)>,
) {
    let panel = owner
        .play
        .as_ref()
        .filter(|_| owner.phase == Phase::Playing)
        .map(|play| play.link.discovery_panel())
        .filter(|discovery| discovery.teams.open);
    let snapshot = panel.map(|discovery| (owner.generation(), discovery.teams, discovery.leader));
    let same_session = state.snapshot.is_some()
        && snapshot.is_some()
        && state.key.generation == owner.generation();
    let submission_finished = state.submission.is_some()
        && snapshot
            .as_ref()
            .is_some_and(|(_, _, leader)| !leader.pending);
    if (state.snapshot == snapshot && !submission_finished)
        || (same_session && fields.iter().any(|(_, text)| editing(text)))
    {
        return;
    }
    let Some(revision) = state.key.revision.checked_add(1) else {
        return;
    };
    state.key = Key {
        generation: owner.generation(),
        revision,
    };
    if state.root.is_some() {
        let mut typed = state.shown.clone();
        let mut editing = false;
        for (field, text) in &fields {
            match field {
                Field::Name => state.search[0] = text.value().to_string(),
                Field::Level => state.search[1] = text.value().to_string(),
                Field::Listing(index) => {
                    typed[*index] = text.value().to_string();
                    editing = true;
                }
            }
        }
        if editing && typed != state.shown {
            state.draft = Some(typed);
        }
    }
    if same_session {
        if let Some((_, _, after)) = &snapshot
            && !after.pending
            && let Some(submission) = state.submission.take()
            && after.confirmed == submission.listing
            && state.draft.as_ref() == Some(&submission.fields)
        {
            state.draft = None;
        }
    } else {
        state.search = snapshot
            .as_ref()
            .map(|(_, panel, _)| {
                [
                    panel.filter.name.clone(),
                    panel
                        .filter
                        .level
                        .map(|level| level.to_string())
                        .unwrap_or_default(),
                ]
            })
            .unwrap_or_default();
        state.draft = None;
        state.submission = None;
    }
    state.pending_editor_action = None;
    if let Some(root) = state.root.take() {
        let mut focused = focus.get();
        while let Some(entity) = focused {
            if entity == root {
                focus.clear();
                break;
            }
            focused = parents.get(entity).ok().map(ChildOf::parent);
        }
        commands.entity(root).despawn();
    }
    if let Some((_, panel, leader)) = &snapshot {
        state.shown = state
            .draft
            .clone()
            .unwrap_or_else(|| widgets::listing_values(leader.confirmed));
        state.root = Some(widgets::build(
            &mut commands,
            &skins,
            state.key,
            panel,
            leader,
            &state.shown,
            &state.search,
        ));
    }
    state.snapshot = snapshot;
}
fn perform(owner: &HostOwner, state: &State, control: &Control, deferred: bool) -> bool {
    let Some((generation, expected, leader)) = &state.snapshot else {
        return false;
    };
    if owner.phase != Phase::Playing
        || owner.modal_active()
        || owner.generation() != *generation
        || control.key != state.key
    {
        return false;
    }
    if let Some(play) = &owner.play
        && (control.kind == Kind::Close || !play.view.gameplay_input_blocked(&play.link))
    {
        // An applicants reply changes the member view while a listing edit settles.
        // The listing control stays bound to the same leader observation.
        if deferred && matches!(control.kind, Kind::Publish(_)) {
            let current = play.link.discovery_panel();
            return current.leader == *leader
                && play
                    .link
                    .team_dispatch_bound(&current.teams, leader, control.kind.clone());
        }
        return play
            .link
            .team_dispatch_bound(expected, leader, control.kind.clone());
    }
    false
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    fields: Query<(&Field, &EditableText)>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    if !guard.allows(event.entity) {
        return;
    }
    if let Ok(control) = controls.get(event.entity) {
        state.pending_editor_action = None;
        if control.key != state.key || owner.modal_active() {
            return;
        }
        if waiting_for_editor(control, &fields) {
            state.pending_editor_action = Some(event.entity);
            return;
        }
        dispatch(&owner, &mut state, control, &fields, false);
    }
}
fn editing(text: &EditableText) -> bool {
    text.pending_paste.is_some()
        || text
            .pending_edits
            .iter()
            .any(bevy::text::TextEdit::is_destructive)
}
fn waiting_for_editor(control: &Control, fields: &Query<(&Field, &EditableText)>) -> bool {
    fields.iter().any(|(field, text)| {
        matches!(
            (&control.kind, field),
            (Kind::Filter(_), Field::Name | Field::Level) | (Kind::Publish(_), Field::Listing(_))
        ) && editing(text)
    })
}
fn dispatch(
    owner: &HostOwner,
    state: &mut State,
    control: &Control,
    fields: &Query<(&Field, &EditableText)>,
    deferred: bool,
) {
    let mut control = control.clone();
    if let Kind::Filter(ref mut filter) = control.kind {
        for (field, text) in fields {
            let value = text.value().to_string();
            match field {
                Field::Name => filter.name = value,
                Field::Level => {
                    filter.level = if value.trim().is_empty() {
                        None
                    } else {
                        let Ok(level) = value.trim().parse::<u16>() else {
                            return;
                        };
                        Some(level)
                    };
                }
                Field::Listing(_) => continue,
            }
        }
    }
    if matches!(control.kind, Kind::Publish(_) | Kind::Withdraw) {
        let mut submitted = [const { String::new() }; widgets::LISTING_FIELDS];
        for (field, text) in fields {
            if let Field::Listing(index) = field {
                submitted[*index] = text.value().to_string();
            }
        }
        if let Kind::Publish(values) = &mut control.kind {
            *values = submitted
                .iter()
                .map(|value| value.trim().to_owned())
                .collect();
        }
        if perform(owner, state, &control, deferred) {
            state.submission = Some(Submission {
                listing: match &control.kind {
                    Kind::Publish(values) => listing_from_fields(values),
                    _ => None,
                },
                fields: submitted,
            });
        }
        return;
    }
    perform(owner, state, &control, deferred);
}
fn finish_edit(
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    controls: Query<&Control>,
    fields: Query<(&Field, &EditableText)>,
    guard: ModalGuard,
) {
    if let Some(entity) = state.pending_editor_action.take()
        && guard.allows(entity)
        && let Ok(control) = controls.get(entity)
    {
        if waiting_for_editor(control, &fields) {
            state.pending_editor_action = Some(entity);
        } else {
            dispatch(&owner, &mut state, control, &fields, true);
        }
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    owner: NonSend<HostOwner>,
    state: Res<State>,
    guard: ModalGuard,
) {
    let Some(root) = state.root else {
        events.clear();
        return;
    };
    if guard.allows(root)
        && events.read().any(|event| {
            event.key_code == KeyCode::Escape
                && event.state == bevy::input::ButtonState::Pressed
                && !event.repeat
        })
    {
        perform(
            &owner,
            &state,
            &Control {
                key: state.key,
                kind: Kind::Close,
            },
            false,
        );
    }
}
