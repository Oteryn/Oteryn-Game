//! Conditions, rest/food and Battle Instinct details from the retained status owners.
mod toolbar;
mod widgets;
use super::super::{owner::HostOwner, ui::Skins};
use super::modal::ModalGuard;
pub(in crate::bevy_shell) use crate::play::feature_0276_condition_status::protocol::details::Target;
use crate::play::feature_0276_condition_status::protocol::details::{Action, Snapshot};
use bevy::{input_focus::InputFocus, prelude::*, ui_widgets::Activate};
pub(in crate::bevy_shell) use toolbar::setup_toolbar;

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
    toolbar::install(app);
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (escape, project).chain());
}
pub(in crate::bevy_shell) fn open_input(link: &crate::play::PlayLink, target: Target) {
    let snapshot = link.condition_details_snapshot(target);
    link.condition_details_dispatch_bound(&snapshot, Action::Open, false);
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
fn escape(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    for event in events.read() {
        if event.repeat
            || event.state != bevy::input::ButtonState::Pressed
            || event.key_code != KeyCode::Escape
        {
            continue;
        }
        for target in Target::ALL {
            let window = state.window(target);
            if window.snapshot.as_ref().is_some_and(Snapshot::open)
                && window
                    .widgets
                    .as_ref()
                    .is_some_and(|widgets| guard.allows(widgets.panel))
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
}
#[cfg(test)]
mod tests;
