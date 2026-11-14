//! Weapon proficiency is a projection of the retained owner, including its advancement HUD.
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{owner::HostOwner, ui::Skins},
    modal::ModalGuard,
};
use crate::play::feature_0195_weapon_proficiency::view::bound::Action;
use bevy::{input_focus::InputFocus, prelude::*, text::EditableText, ui_widgets::Activate};
use oteryn_client_ui::weapon_proficiency::WeaponProficiencyPanel as Panel;
#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    generation: u64,
    session: usize,
    revision: u64,
}
#[derive(Clone, PartialEq, Eq)]
enum Kind {
    Owner(Action),
    Review(usize),
    Confirm,
    Cancel,
}
#[derive(Component, Clone)]
struct Control {
    key: Key,
    kind: Kind,
}
#[derive(Component)]
struct Search;
#[derive(Resource, Default)]
struct State {
    snapshot: Option<(Key, Panel)>,
    revision: u64,
    confirmation: Option<(Panel, usize)>,
    widgets: Option<widgets::Widgets>,
    hud: Option<Entity>,
    dirty: bool,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain());
}
pub(in crate::bevy_shell) fn open(owner: &HostOwner) {
    if owner.modal_active() {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    let (session, panel, _) = play.link.weapon_proficiency_snapshot();
    let action = if panel.open {
        Action::Close
    } else {
        Action::Open
    };
    let _ = play
        .link
        .weapon_proficiency_dispatch_bound(session, &panel, action);
}
fn perform(owner: &HostOwner, state: &mut State, control: &Control) {
    let Some((key, expected)) = &state.snapshot else {
        return;
    };
    if *key != control.key || key.generation != owner.generation() || owner.modal_active() {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    let (session, panel, _) = play.link.weapon_proficiency_snapshot();
    if session != key.session || panel != *expected || !panel.selected {
        return;
    }
    if state.confirmation.is_some()
        && !matches!(
            control.kind,
            Kind::Confirm | Kind::Cancel | Kind::Owner(Action::Close)
        )
    {
        return;
    }
    match &control.kind {
        Kind::Review(id) => {
            if !panel.open || panel.pending {
                return;
            }
            state.confirmation = Some((panel, *id));
            state.dirty = true;
        }
        Kind::Confirm => {
            let Some((confirmed, id)) = &state.confirmation else {
                return;
            };
            if *confirmed != panel {
                return;
            }
            let _ = play.link.weapon_proficiency_dispatch_bound(
                session,
                confirmed,
                Action::Confirmed(*id),
            );
            state.confirmation = None;
            state.dirty = true;
        }
        Kind::Cancel => {
            state.confirmation = None;
            state.dirty = true;
        }
        Kind::Owner(action) => {
            let _ = play
                .link
                .weapon_proficiency_dispatch_bound(session, expected, action.clone());
        }
    }
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    search: Query<&EditableText, With<Search>>,
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
    let mut control = control.clone();
    if matches!(control.kind, Kind::Owner(Action::Search(_))) {
        let Ok(text) = search.single() else {
            return;
        };
        if text.pending_paste.is_some() || !text.pending_edits.is_empty() {
            return;
        }
        control.kind = Kind::Owner(Action::Search(text.editor.text().to_string()));
    }
    perform(&owner, &mut state, &control);
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    mut focus: ResMut<InputFocus>,
    parents: Query<&ChildOf>,
    guard: ModalGuard,
) {
    let Some(play) = &owner.play else {
        clear(&mut commands, &mut state, &mut focus, &parents);
        return;
    };
    let (session, mut panel, mut display) = play.link.weapon_proficiency_snapshot();
    // Do not race a confirmation or another modal with the owner's scheduled refresh.
    if state.confirmation.is_none()
        && !owner.modal_active()
        && !play.view.gameplay_input_blocked(&play.link)
        && state.widgets.as_ref().map_or_else(
            || guard.allows_focus(focus.get()),
            |widgets| guard.allows(widgets.panel),
        )
    {
        play.link
            .weapon_proficiency_tick_bound(session, &panel, std::time::Instant::now());
        (_, panel, display) = play.link.weapon_proficiency_snapshot();
    }
    if state.snapshot.as_ref().is_some_and(|(key, old)| {
        key.generation == owner.generation() && key.session == session && old == &panel
    }) && !state.dirty
    {
        return;
    }
    if state
        .snapshot
        .as_ref()
        .is_some_and(|(key, _)| key.generation != owner.generation() || key.session != session)
    {
        clear(&mut commands, &mut state, &mut focus, &parents);
    }
    if state
        .confirmation
        .as_ref()
        .is_some_and(|(expected, _)| expected != &panel)
    {
        state.confirmation = None;
    }
    let Some(revision) = state.revision.checked_add(1) else {
        return;
    };
    state.revision = revision;
    let key = Key {
        generation: owner.generation(),
        session,
        revision,
    };
    if let Some(hud) = state.hud.take() {
        commands.entity(hud).despawn();
    }
    state.hud = widgets::hud(&mut commands, &skins, key, &panel);
    if panel.open {
        let confirmation = state.confirmation.as_ref().map(|(_, id)| *id);
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
            &panel,
            &display,
            confirmation,
        );
    } else if let Some(widgets) = state.widgets.take() {
        if inside(focus.get(), widgets.overlay, &parents) {
            focus.clear();
        }
        commands.entity(widgets.overlay).despawn();
        state.confirmation = None;
    }
    state.snapshot = Some((key, panel));
    state.dirty = false;
}
fn inside(mut current: Option<Entity>, ancestor: Entity, parents: &Query<&ChildOf>) -> bool {
    while let Some(entity) = current {
        if entity == ancestor {
            return true;
        }
        current = parents.get(entity).ok().map(ChildOf::parent);
    }
    false
}
fn clear(
    commands: &mut Commands,
    state: &mut State,
    focus: &mut InputFocus,
    parents: &Query<&ChildOf>,
) {
    if let Some(widgets) = state.widgets.take() {
        if inside(focus.get(), widgets.overlay, parents) {
            focus.clear();
        }
        commands.entity(widgets.overlay).despawn();
    }
    if let Some(hud) = state.hud.take() {
        commands.entity(hud).despawn();
    }
    state.snapshot = None;
    state.confirmation = None;
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    focus: Res<InputFocus>,
) {
    for event in events
        .read()
        .filter(|e| e.state == bevy::input::ButtonState::Pressed && !e.repeat)
    {
        let allowed = state.widgets.as_ref().map_or_else(
            || guard.allows_focus(focus.get()),
            |w| guard.allows(w.panel),
        );
        if !allowed {
            continue;
        }
        if event.key_code == KeyCode::Escape
            && state.widgets.is_some()
            && let Some((key, _)) = &state.snapshot
        {
            let kind = if state.confirmation.is_some() {
                Kind::Cancel
            } else {
                Kind::Owner(Action::Close)
            };
            let control = Control { key: *key, kind };
            perform(&owner, &mut state, &control);
        }
    }
}
