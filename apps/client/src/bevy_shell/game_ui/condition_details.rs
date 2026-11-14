//! Conditions, rest/food and Battle Instinct details from the retained status owners.
mod widgets;
use super::super::{owner::HostOwner, ui::Skins};
use super::modal::ModalGuard;
pub(in crate::bevy_shell) use crate::play::feature_0276_condition_status::protocol::details::Target;
use crate::play::feature_0276_condition_status::protocol::details::{Action, Snapshot};
use bevy::{input_focus::InputFocus, prelude::*, ui_widgets::Activate};

pub(in crate::bevy_shell) fn toolbar_window(
    control: Option<&crate::bevy_shell::ui::Control>,
) -> bool {
    use crate::bevy_shell::ui::Control;
    matches!(
        control,
        Some(
            Control::Details(_)
                | Control::Market
                | Control::Party
                | Control::Quests
                | Control::Spells
        )
    )
}
impl Target {
    pub(in crate::bevy_shell) fn id(self) -> &'static str {
        match self {
            Self::Conditions => "shell.conditions",
            Self::Rest => "shell.rest_food",
            Self::BattleInstinct => "shell.battle_instinct",
        }
    }
    fn title(self) -> &'static str {
        match self {
            Self::Conditions => "Conditions",
            Self::Rest => "Rest and food",
            Self::BattleInstinct => "Battle Instinct",
        }
    }
    fn shortcut(self) -> KeyCode {
        match self {
            Self::Conditions => KeyCode::KeyC,
            Self::Rest => KeyCode::KeyR,
            Self::BattleInstinct => KeyCode::KeyB,
        }
    }
}
#[derive(Component, Clone, Copy)]
struct Control {
    target: Target,
    generation: u64,
    revision: u64,
    action: Action,
}
#[derive(Default)]
struct Window {
    widgets: Option<widgets::Widgets>,
    snapshot: Option<Snapshot>,
    generation: u64,
    revision: u64,
}
#[derive(Resource, Default)]
struct State {
    windows: [Window; 3],
}
impl State {
    fn window(&mut self, target: Target) -> &mut Window {
        &mut self.windows[match target {
            Target::Conditions => 0,
            Target::Rest => 1,
            Target::BattleInstinct => 2,
        }]
    }
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<OpeningWindow>()
        .add_systems(Update, (window_focus, keyboard, project).chain());
}
#[derive(Resource, Default)]
struct OpeningWindow {
    focused: bool,
}
fn window_focus(
    mut state: ResMut<OpeningWindow>,
    windows: Query<&bevy::window::Window, With<bevy::window::PrimaryWindow>>,
) {
    state.focused = windows.iter().all(|window| window.focused);
}
#[derive(bevy::ecs::system::SystemParam)]
pub(in crate::bevy_shell) struct OpeningInput<'w, 's> {
    focus: Res<'w, InputFocus>,
    editors: Query<'w, 's, (), With<bevy::text::EditableText>>,
    // Keep Window queries in this window's installed systems. Shared toolbar
    // observers must not register Window in other windows' headless fixtures.
    window: Option<Res<'w, OpeningWindow>>,
}
impl OpeningInput<'_, '_> {
    pub(in crate::bevy_shell) fn toggle(&self, owner: &HostOwner, target: Target) {
        let blocked = self
            .focus
            .get()
            .is_some_and(|entity| self.editors.contains(entity))
            || self.window.as_ref().is_some_and(|window| !window.focused);
        toggle(owner, target, blocked);
    }
}
pub(in crate::bevy_shell) fn toggle(owner: &HostOwner, target: Target, blocked: bool) {
    if blocked || owner.phase != crate::bevy_shell::owner::Phase::Playing {
        return;
    }
    if let Some(play) = &owner.play {
        let snapshot = play.link.condition_details_snapshot(target);
        let action = if snapshot.open() {
            Action::Close
        } else {
            Action::Open
        };
        play.link
            .condition_details_dispatch_bound(&snapshot, action, owner.modal_active());
    }
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skin: Res<Skins>,
    mut state: ResMut<State>,
    mut focus: ResMut<InputFocus>,
    parents: Query<&ChildOf>,
) {
    for target in Target::ALL {
        let window = state.window(target);
        let model = owner
            .play
            .as_ref()
            .map(|play| play.link.condition_details_snapshot(target));
        let Some(model) = model.filter(Snapshot::open) else {
            remove(&mut commands, window, &mut focus, &parents);
            window.snapshot = None;
            continue;
        };
        if window.generation == owner.generation() && window.snapshot.as_ref() == Some(&model) {
            continue;
        }
        // A replacement connection can have the same public values and host generation;
        // the snapshot also retains the exact mailbox allocation identity.
        if window.generation != owner.generation() {
            remove(&mut commands, window, &mut focus, &parents);
        }
        let Some(revision) = window.revision.checked_add(1) else {
            continue;
        };
        window.revision = revision;
        window.generation = owner.generation();
        let widgets = window
            .widgets
            .get_or_insert_with(|| widgets::build(&mut commands, &skin, target));
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
            &model,
            window.generation,
            revision,
        );
        window.snapshot = Some(model);
    }
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
fn remove(
    commands: &mut Commands,
    window: &mut Window,
    focus: &mut InputFocus,
    parents: &Query<&ChildOf>,
) {
    if let Some(widgets) = window.widgets.take() {
        if inside(focus.get(), widgets.root, parents) {
            focus.clear();
        }
        commands.entity(widgets.root).despawn();
    }
}
fn perform(owner: &HostOwner, state: &mut State, control: Control, blocked: bool) {
    let window = state.window(control.target);
    if control.generation != owner.generation()
        || window.generation != control.generation
        || window.revision != control.revision
    {
        return;
    }
    if let (Some(play), Some(snapshot)) = (&owner.play, &window.snapshot) {
        play.link.condition_details_dispatch_bound(
            snapshot,
            control.action,
            blocked || owner.modal_active(),
        );
    }
}
fn activate(
    event: On<Activate>,
    guard: ModalGuard,
    controls: Query<&Control>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    if let Ok(control) = controls.get(event.entity) {
        perform(&owner, &mut state, *control, !guard.allows(event.entity));
    }
}
fn keyboard(
    input: OpeningInput,
    keys: Res<ButtonInput<KeyCode>>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    let chord = (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
        && (keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight))
        && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight));
    for target in Target::ALL {
        let window = state.window(target);
        let allowed = window.widgets.as_ref().map_or_else(
            || guard.allows_focus(None),
            |widgets| guard.allows(widgets.panel),
        );
        if !allowed {
            continue;
        }
        if chord && keys.just_pressed(target.shortcut()) {
            input.toggle(&owner, target);
        } else if keys.just_pressed(KeyCode::Escape)
            && window.snapshot.as_ref().is_some_and(Snapshot::open)
        {
            let control = Control {
                target,
                generation: window.generation,
                revision: window.revision,
                action: Action::Close,
            };
            perform(&owner, &mut state, control, false);
        }
    }
}
#[cfg(test)]
mod tests;
