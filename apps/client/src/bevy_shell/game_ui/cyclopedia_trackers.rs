//! Cyclopedia tracker presentation over the retained tracker owner.
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{owner::HostOwner, ui::Skins},
    modal::ModalGuard,
};
use crate::play::feature_0197_cyclopedia_tracker::view::{
    TrackerBinding, TrackerViewAction as OwnerAction,
};
use bevy::{input_focus::InputFocus, prelude::*, text::EditableText, ui_widgets::Activate};
use oteryn_client_ui::cyclopedia_tracker::{CyclopediaTrackerPanel as Panel, Intent, TrackerSort};

#[derive(Component)]
struct Overlay;
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Field {
    Key,
    Search(bool),
}
#[derive(Clone, PartialEq, Eq)]
enum Kind {
    Boss,
    Widget(bool),
    Request(Intent),
    Remove(bool, String),
    Sort(bool, TrackerSort),
    Hide(bool),
    Confirm,
    Cancel,
    Close,
}
#[derive(Component, Clone)]
struct Control {
    generation: u64,
    revision: u64,
    kind: Kind,
}
#[derive(Clone, PartialEq, Eq)]
struct Draft {
    key: String,
    search: String,
    boss_search: String,
}
struct Confirmation {
    binding: TrackerBinding,
    draft: Draft,
    key: String,
    boss: bool,
}
#[derive(Resource, Default)]
struct State {
    binding: Option<TrackerBinding>,
    generation: u64,
    revision: u64,
    widgets: Option<widgets::Widgets>,
    confirmation: Option<Confirmation>,
    dirty: bool,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .init_resource::<ButtonInput<KeyCode>>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain())
        .add_systems(PostUpdate, search.after(bevy::text::EditableTextSystems));
}
pub(in crate::bevy_shell) fn open(owner: &HostOwner) {
    if let Some(play) = &owner.play {
        let binding = play.link.cyclopedia_trackers_binding();
        let _ = play.link.cyclopedia_trackers_dispatch_bound(
            &binding,
            OwnerAction::Toggle,
            owner.modal_active() || play.view.gameplay_input_blocked(&play.link),
        );
    }
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    mut focus: ResMut<InputFocus>,
) {
    let binding = owner
        .play
        .as_ref()
        .map(|play| play.link.cyclopedia_trackers_binding());
    let generation = owner.generation();
    let visible = binding.as_ref().is_some_and(|b| b.panel.open);
    if !visible || state.generation != generation {
        if let Some(widgets) = state.widgets.take() {
            if focus.get().is_some_and(|e| widgets.fields.contains(&e)) {
                focus.clear();
            }
            commands.entity(widgets.root).despawn();
        }
        state.binding = None;
        state.confirmation = None;
    }
    let Some(binding) = binding.filter(|b| b.panel.open) else {
        return;
    };
    if !state.dirty
        && state
            .binding
            .as_ref()
            .is_some_and(|old| old.panel == binding.panel)
        && state.widgets.is_some()
    {
        return;
    }
    if state
        .binding
        .as_ref()
        .is_some_and(|old| old.panel != binding.panel)
    {
        state.confirmation = None;
    }
    state.generation = generation;
    let Some(revision) = state.revision.checked_add(1) else {
        return;
    };
    state.revision = revision;
    state.dirty = false;
    if state.widgets.is_none() {
        state.widgets = Some(widgets::build(&mut commands, &skins, &binding.panel));
    }
    let confirmation = state.confirmation.as_ref().map(|c| c.key.clone());
    if let Some(widgets) = state.widgets.as_mut() {
        widgets::render(
            &mut commands,
            &skins,
            widgets,
            generation,
            revision,
            &binding.panel,
            confirmation.as_deref(),
        );
    }
    state.binding = Some(binding);
}
fn values(fields: &Query<(&Field, &EditableText)>) -> Option<Draft> {
    let mut key = None;
    let mut search = None;
    let mut boss_search = None;
    for (field, text) in fields {
        if text.pending_paste.is_some()
            || text
                .pending_edits
                .iter()
                .any(bevy::text::TextEdit::is_destructive)
        {
            return None;
        }
        match field {
            Field::Key => key = Some(text.value().to_string()),
            Field::Search(false) => search = Some(text.value().to_string()),
            Field::Search(true) => boss_search = Some(text.value().to_string()),
        }
    }
    Some(Draft {
        key: key?,
        search: search?,
        boss_search: boss_search?,
    })
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    fields: Query<(&Field, &EditableText)>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if control.generation != owner.generation() || control.revision != state.revision {
        return;
    }
    let blocked = !guard.allows(event.entity) || owner.modal_active();
    let Some(play) = &owner.play else { return };
    if blocked || play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    let Some(binding) = state.binding.clone() else {
        return;
    };
    let Some(draft) = values(&fields) else { return };
    let action = match &control.kind {
        Kind::Close => OwnerAction::Close,
        Kind::Cancel => {
            state.confirmation = None;
            state.dirty = true;
            return;
        }
        Kind::Confirm => {
            let Some(confirm) = state.confirmation.take() else {
                return;
            };
            if confirm.draft != draft {
                state.confirmation = Some(confirm);
                return;
            }
            let accepted = play.link.cyclopedia_trackers_dispatch_bound(
                &confirm.binding,
                OwnerAction::Request {
                    intent: Intent::Remove,
                    boss: confirm.boss,
                    creature_key: confirm.key.clone(),
                    confirmed: true,
                },
                blocked,
            );
            if !accepted {
                state.confirmation = Some(confirm);
            }
            state.dirty = true;
            return;
        }
        Kind::Remove(boss, key) => {
            if state.confirmation.is_some() || binding.panel.pending {
                return;
            }
            state.confirmation = Some(Confirmation {
                binding,
                draft,
                key: key.clone(),
                boss: *boss,
            });
            state.dirty = true;
            return;
        }
        Kind::Request(Intent::Remove) => {
            if state.confirmation.is_some() || binding.panel.pending {
                return;
            }
            state.confirmation = Some(Confirmation {
                boss: binding.panel.boss,
                binding,
                key: draft.key.clone(),
                draft,
            });
            state.dirty = true;
            return;
        }
        Kind::Request(intent) => OwnerAction::Request {
            intent: *intent,
            boss: binding.panel.boss,
            creature_key: draft.key,
            confirmed: false,
        },
        Kind::Boss => OwnerAction::Boss,
        Kind::Widget(boss) => OwnerAction::Widget(*boss),
        Kind::Sort(boss, sort) => filter(&binding.panel, *boss, Some(*sort), false, None),
        Kind::Hide(boss) => filter(&binding.panel, *boss, None, true, None),
    };
    if state.confirmation.is_some() && control.kind != Kind::Close {
        return;
    }
    let _ = play
        .link
        .cyclopedia_trackers_dispatch_bound(&binding, action, blocked);
}
fn filter(
    panel: &Panel,
    boss: bool,
    sort: Option<TrackerSort>,
    toggle: bool,
    search: Option<String>,
) -> OwnerAction {
    let (old_search, old_sort, old_hide) = if boss {
        (
            &panel.boss_search,
            panel.boss_sort,
            panel.boss_hide_completed,
        )
    } else {
        (&panel.search, panel.sort, panel.hide_completed)
    };
    OwnerAction::Filter {
        boss,
        search: search.unwrap_or_else(|| old_search.clone()),
        sort: sort.unwrap_or(old_sort),
        hide_completed: old_hide ^ toggle,
    }
}
fn search(
    fields: Query<(&Field, &EditableText)>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    state: Res<State>,
) {
    let Some(widgets) = &state.widgets else {
        return;
    };
    if state.confirmation.is_some()
        || state.generation != owner.generation()
        || !guard.allows(widgets.root)
        || owner.modal_active()
    {
        return;
    }
    let (Some(binding), Some(draft), Some(play)) = (&state.binding, values(&fields), &owner.play)
    else {
        return;
    };
    if play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    for (boss, value, old) in [
        (false, draft.search, &binding.panel.search),
        (true, draft.boss_search, &binding.panel.boss_search),
    ] {
        if &value != old {
            let _ = play.link.cyclopedia_trackers_dispatch_bound(
                binding,
                filter(&binding.panel, boss, None, false, Some(value)),
                false,
            );
            break;
        }
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    state: Res<State>,
    focus: Res<InputFocus>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    for event in events.read() {
        if event.state != bevy::input::ButtonState::Pressed || event.repeat {
            continue;
        }
        if event.key_code == KeyCode::F8
            && !keys.any_pressed([
                KeyCode::ControlLeft,
                KeyCode::ControlRight,
                KeyCode::AltLeft,
                KeyCode::AltRight,
                KeyCode::ShiftLeft,
                KeyCode::ShiftRight,
                KeyCode::SuperLeft,
                KeyCode::SuperRight,
            ])
            && guard.allows_focus(focus.get())
        {
            open(&owner);
        }
        if event.key_code == KeyCode::Escape
            && let (Some(widgets), Some(binding), Some(play)) =
                (&state.widgets, &state.binding, &owner.play)
        {
            let _ = play.link.cyclopedia_trackers_dispatch_bound(
                binding,
                OwnerAction::Close,
                !guard.allows(widgets.root) || owner.modal_active(),
            );
        }
    }
}
