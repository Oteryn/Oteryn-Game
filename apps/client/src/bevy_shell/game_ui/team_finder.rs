//! Join Team and Manage Team over the negotiated Team Finder member and leader streams.
#[cfg(test)]
mod input_tests;
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
use bevy::{
    input_focus::InputFocus,
    picking::events::PointerState,
    prelude::*,
    text::EditableText,
    ui::{InteractionDisabled, Pressed, UiSystems},
    ui_widgets::Activate,
};
use oteryn_client_ui::{
    localization::{Language, translate},
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
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Field {
    Name,
    Level,
    /// One leader listing field, in `listing_from_fields` order.
    Listing(usize),
}
type Snapshot = (u64, Panel, Leader);

#[derive(Resource, Default)]
struct State {
    snapshot: Option<Snapshot>,
    submission: Option<Submission>,
    pending_editor_action: Option<DeferredAction>,
    root: Option<Entity>,
    widgets: Option<widgets::Widgets>,
    key: Key,
    /// The listing editor values used to detect changes to the local draft.
    shown: [String; widgets::LISTING_FIELDS],
    /// The leader's unpublished listing edits, kept across tabs until a reply confirms them.
    draft: Option<[String; widgets::LISTING_FIELDS]>,
}
struct DeferredAction {
    control: Control,
    snapshot: Snapshot,
}

struct Submission {
    fields: [String; widgets::LISTING_FIELDS],
    listing: Option<LeaderListing>,
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(
            Update,
            (release_ended_pointer_presses, keyboard, project, widgets::captions).chain(),
        )
        .add_systems(
            PostUpdate,
            sync_listing_editors
                .after(bevy::text::EditableTextSystems)
                .before(UiSystems::PostLayout),
        )
        // Wait for text edits and the modal guard's end-of-frame snapshot.
        .add_systems(Last, finish_edit);
}

// Picking may cancel a pointer without delivering Cancel to the original button.
fn release_ended_pointer_presses(
    mut commands: Commands,
    pointers: Option<Res<PointerState>>,
    pressed: Query<Entity, (With<Control>, With<Pressed>)>,
) {
    let Some(pointers) = pointers else {
        return;
    };
    for entity in &pressed {
        if !pointers
            .pointer_buttons
            .values()
            .any(|button| button.pressing.contains_key(&entity))
        {
            commands.entity(entity).remove::<Pressed>();
        }
    }
}

// The dialog and its editors live for the session's entire open window.
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
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
    if state.snapshot == snapshot {
        return;
    }
    let same_view =
        same_session
            && state.snapshot.as_ref().zip(snapshot.as_ref()).is_some_and(
                |((_, before, _), (_, after, _))| before.applicants == after.applicants,
            );
    let Some(revision) = state.key.revision.checked_add(1) else {
        return;
    };
    let key = Key {
        generation: owner.generation(),
        revision,
    };
    state.key = key;
    if !same_view {
        state.pending_editor_action = None;
    } else if let Some(action) = &mut state.pending_editor_action {
        // Keep the original mailbox binding when the control's revision advances.
        action.control.key = key;
    }

    if !same_session {
        if let Some(root) = state.root.take() {
            commands.entity(root).despawn();
        }
        state.widgets = None;
        state.draft = None;
        state.submission = None;
    }

    if let Some(snapshot @ (_, panel, leader)) = &snapshot {
        if !same_session {
            let level = panel
                .filter
                .level
                .map(|level| level.to_string())
                .unwrap_or_default();
            let listing = widgets::listing_values(leader.confirmed);
            let (root, widgets) = widgets::build(
                &mut commands,
                &skins,
                [&panel.filter.name, &level],
                &listing,
            );
            state.root = Some(root);
            state.widgets = Some(widgets);
            state.shown = listing;
        }
        if let Some(widgets) = &mut state.widgets {
            widgets.project(&mut commands, &skins, key, snapshot);
        }
    }
    state.snapshot = snapshot;
}

fn sync_listing_editors(
    mut state: ResMut<State>,
    focus: Res<InputFocus>,
    mut editors: Query<(Entity, &Field, &mut EditableText)>,
) {
    let Some((_, _, leader)) = &state.snapshot else {
        return;
    };
    let confirmed = leader.confirmed;
    let pending = leader.pending;
    // The mailbox stores the reply caption in the language active at receipt.
    let accepted = state.submission.is_some()
        && !pending
        && Language::ALL
            .into_iter()
            .any(|language| leader.feedback == translate(language, "Advertisement confirmed"));

    let mut draft = None;
    for (_, field, text) in &editors {
        if let Field::Listing(index) = *field
            && text.value() != state.shown[index].as_str()
        {
            draft.get_or_insert_with(|| state.shown.clone())[index] = text.value().to_string();
        }
    }
    if let Some(draft) = draft {
        state.shown.clone_from(&draft);
        state.draft = Some(draft);
    }
    if !pending
        && let Some(submission) = state.submission.take()
        && accepted
        && confirmed == submission.listing
        && state.draft.as_ref() == Some(&submission.fields)
    {
        state.draft = None;
    }

    // The latest confirmation remains pending for each editor until its input has settled.
    let listing = state
        .draft
        .clone()
        .unwrap_or_else(|| widgets::listing_values(confirmed));
    for (entity, field, mut text) in &mut editors {
        if let Field::Listing(index) = *field
            && text.value() != listing[index].as_str()
            && focus.get() != Some(entity)
            && !text.is_composing()
            && text.pending_paste.is_none()
            && text.pending_edits.is_empty()
        {
            text.editor_mut().set_text(&listing[index]);
            state.shown[index].clone_from(&listing[index]);
        }
    }
}

fn perform(
    owner: &HostOwner,
    state: &State,
    control: &Control,
    binding: Option<&Snapshot>,
) -> bool {
    let Some((generation, expected, leader)) = binding.or(state.snapshot.as_ref()) else {
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
        return play
            .link
            .team_dispatch_bound(expected, leader, control.kind.clone());
    }
    false
}
fn activate(
    event: On<Activate>,
    controls: Query<(&Control, Has<InteractionDisabled>)>,
    fields: Query<(&Field, &EditableText)>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    if !guard.allows(event.entity) {
        return;
    }
    if let Ok((control, false)) = controls.get(event.entity) {
        state.pending_editor_action = None;
        if control.key != state.key || owner.modal_active() {
            return;
        }
        if waiting_for_editor(control, &fields) {
            state.pending_editor_action = state.snapshot.clone().map(|snapshot| DeferredAction {
                control: control.clone(),
                snapshot,
            });
            return;
        }
        dispatch(&owner, &mut state, control, &fields, None);
    }
}
fn editing(text: &EditableText) -> bool {
    text.is_composing()
        || text.pending_paste.is_some()
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
    binding: Option<&Snapshot>,
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
        if perform(owner, state, &control, binding) {
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
    perform(owner, state, &control, binding);
}
fn finish_edit(
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    fields: Query<(&Field, &EditableText)>,
    guard: ModalGuard,
) {
    if let Some(action) = state.pending_editor_action.take()
        && state.root.is_some_and(|root| guard.allows(root))
    {
        if waiting_for_editor(&action.control, &fields) {
            state.pending_editor_action = Some(action);
        } else {
            dispatch(
                &owner,
                &mut state,
                &action.control,
                &fields,
                Some(&action.snapshot),
            );
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
            None,
        );
    }
}
