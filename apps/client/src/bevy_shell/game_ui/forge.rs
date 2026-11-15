//! Exaltation Forge over the retained candidates, quotes, intents and receipts.
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use crate::play::feature_0152_item_forge::view::{ForgeControl as Intent, forge_action_enabled};
use bevy::{
    input_focus::{FocusCause, InputFocus},
    prelude::*,
    ui_widgets::Activate,
};
use oteryn_client_runtime::cui3::item_forge::{Operation, Request};
use oteryn_client_ui::item_forge::ItemForgePanel as Panel;
use std::time::Duration;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Tab {
    #[default]
    Fusion,
    Transfer,
    Conversion,
    History,
}
impl Tab {
    const ALL: [Self; 4] = [
        Self::Fusion,
        Self::Transfer,
        Self::Conversion,
        Self::History,
    ];
    fn label(self) -> &'static str {
        match self {
            Self::Fusion => "Fusion",
            Self::Transfer => "Transfer",
            Self::Conversion => "Conversion",
            Self::History => "History",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Key {
    generation: u64,
    revision: u64,
    tab: Tab,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Tab(Tab),
    Close,
    Convergence,
    First(u64),
    Second(u64),
    Boost,
    Protect,
    Quote,
    Submit,
    Refresh,
    Next,
    Convert(Operation),
    Confirm(u64),
    Dismiss(u64),
}
#[derive(Component, Clone)]
struct Control {
    key: Key,
    kind: Kind,
}
struct Pending {
    generation: u64,
    request: Option<Request>,
    since: Duration,
    refreshed: bool,
}
#[derive(Resource, Default)]
struct State {
    snapshot: Option<(Key, Panel)>,
    tab: Tab,
    revision: u64,
    widgets: Option<widgets::Widgets>,
    pending: Option<Pending>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, timeout, project).chain());
}
fn timeout(owner: NonSend<HostOwner>, time: Res<Time<Real>>, mut state: ResMut<State>) {
    let Some(play) = &owner.play else {
        state.pending = None;
        return;
    };
    let panel = play.link.item_forge_panel();
    if !panel.open || !panel.pending {
        state.pending = None;
        return;
    }
    let generation = owner.generation();
    if state.pending.as_ref().is_none_or(|pending| {
        pending.generation != generation || pending.request != panel.submitted
    }) {
        state.pending = Some(Pending {
            generation,
            request: panel.submitted,
            since: time.elapsed(),
            refreshed: false,
        });
    }
    if let Some(pending) = &mut state.pending
        && !pending.refreshed
        && time.elapsed().saturating_sub(pending.since) >= Duration::from_secs(5)
    {
        pending.refreshed = play.link.forge_timeout_refresh(&panel);
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
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    mut focus: ResMut<InputFocus>,
    parents: Query<&ChildOf>,
) {
    let panel = owner.play.as_ref().map(|play| play.link.item_forge_panel());
    let Some(panel) = panel.filter(|panel| panel.open) else {
        if let Some(widgets) = state.widgets.take() {
            if inside(focus.get(), widgets.overlay, &parents) {
                focus.clear();
            }
            commands.entity(widgets.overlay).despawn();
        }
        state.snapshot = None;
        state.tab = Tab::Fusion;
        return;
    };
    let generation = owner.generation();
    if state
        .snapshot
        .as_ref()
        .is_none_or(|(key, _)| key.generation != generation)
    {
        state.tab = if matches!(
            panel.operation,
            Operation::Transfer | Operation::ConvergenceTransfer
        ) {
            Tab::Transfer
        } else {
            Tab::Fusion
        };
    }
    if state.snapshot.as_ref().is_some_and(|(key, old)| {
        key.generation == generation && key.tab == state.tab && old == &panel
    }) {
        return;
    }
    if state
        .snapshot
        .as_ref()
        .is_some_and(|(key, _)| key.generation != generation)
        && let Some(widgets) = state.widgets.take()
    {
        if inside(focus.get(), widgets.overlay, &parents) {
            focus.clear();
        }
        commands.entity(widgets.overlay).despawn();
    }
    let Some(revision) = state.revision.checked_add(1) else {
        return;
    };
    state.revision = revision;
    let key = Key {
        generation,
        revision,
        tab: state.tab,
    };
    let widgets = state
        .widgets
        .get_or_insert_with(|| widgets::build(&mut commands, &skins));
    let retained = super::refresh::preserve(&mut commands, widgets.panel, focus.get());
    if widgets
        .content
        .is_some_and(|content| inside(focus.get(), content, &parents))
    {
        focus.set(widgets.panel, FocusCause::Navigated);
    }
    widgets::render(&mut commands, &skins, widgets, key, &panel);
    retained.restore(&mut commands, widgets.panel);
    state.snapshot = Some((key, panel));
}
fn perform(owner: &HostOwner, state: &mut State, control: &Control) {
    let Some((key, panel)) = &state.snapshot else {
        return;
    };
    if *key != control.key || key.generation != owner.generation() || owner.modal_active() {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if control.kind != Kind::Close && play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    let intent = match control.kind {
        Kind::Tab(tab) => {
            if !panel.selected || panel.pending || play.link.item_forge_panel() != *panel {
                return;
            }
            let intent = match tab {
                Tab::Fusion => Some(Intent::Operation(Operation::Fusion)),
                Tab::Transfer => Some(Intent::Operation(Operation::Transfer)),
                Tab::Conversion => None,
                Tab::History => Some(Intent::Convert(Operation::QueryHistory)),
            };
            if let Some(intent) = intent
                && !play.link.forge_dispatch_bound(panel, intent)
            {
                return;
            }
            state.tab = tab;
            return;
        }
        Kind::Close => Intent::Close,
        Kind::Convergence => Intent::Convergence,
        Kind::First(id) => Intent::First(id),
        Kind::Second(id) => Intent::Second(id),
        Kind::Boost => Intent::Boost,
        Kind::Protect => Intent::Protect,
        Kind::Quote => Intent::Quote,
        Kind::Submit => Intent::Submit,
        Kind::Refresh => Intent::Refresh,
        Kind::Next => Intent::Next,
        Kind::Convert(operation) => Intent::Convert(operation),
        Kind::Confirm(serial) => Intent::Confirm(serial),
        Kind::Dismiss(serial) => Intent::Dismiss(serial),
    };
    play.link.forge_dispatch_bound(panel, intent);
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
            kind: Kind::Close,
        };
        perform(&owner, &mut state, &control);
    }
}
