//! Depot, supply stash and search present the retained owners' confirmed server data.
mod actions;
#[cfg(test)]
mod owner_tests;
#[cfg(test)]
mod tests;
mod widgets;
use super::super::{owner::HostOwner, ui::Skins};
use super::modal::ModalGuard;
use crate::play::feature_0037_depot::view::DepotBinding;
use crate::play::feature_0062_stash::view::StashControl;
use bevy::{input_focus::InputFocus, prelude::*, text::EditableText, ui_widgets::Activate};
use oteryn_client_ui::{depot, depot_search, stash};

#[derive(Clone)]
enum Snapshot {
    Depot(DepotBinding<depot::DepotPanel>),
    Stash(DepotBinding<stash::StashPanel>),
    Search(DepotBinding<depot_search::DepotSearchPanel>),
}
impl Snapshot {
    fn open(&self) -> bool {
        match self {
            Self::Depot(p) => p.panel.open,
            Self::Stash(p) => p.panel.open,
            Self::Search(p) => p.panel.open,
        }
    }
    fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Depot(a), Self::Depot(b)) => a.same(b),
            (Self::Stash(a), Self::Stash(b)) => a.same(b),
            (Self::Search(a), Self::Search(b)) => a.same(b),
            _ => false,
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    generation: u64,
    revision: u64,
    window: usize,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Close,
    Browse { box_number: u8, page: u32 },
    Deposit(oteryn_session::ItemHandle),
    Retrieve(usize),
    Refresh,
    Previous,
    Next,
    Filter,
    Withdraw(usize),
    Search,
}
#[derive(Component, Clone)]
struct Control {
    key: Key,
    kind: Kind,
}
#[derive(Component)]
struct Field(usize);
struct Window {
    key: Key,
    snapshot: Snapshot,
    root: Entity,
}
#[derive(Resource, Default)]
struct State {
    windows: [Option<Window>; 3],
    revision: u64,
    pending: Option<(Control, Entity, Entity)>,
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (actions::keyboard, project).chain())
        .add_systems(
            PostUpdate,
            actions::finish.after(bevy::text::EditableTextSystems),
        );
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    mut focus: ResMut<InputFocus>,
    fields: Query<(Entity, &Field)>,
) {
    let generation = owner.generation();
    let panels = owner.play.as_ref().map(|play| {
        [
            Snapshot::Depot(play.link.depot_binding(generation)),
            Snapshot::Stash(play.link.stash_binding(generation)),
            Snapshot::Search(play.link.depot_search_binding(generation)),
        ]
    });
    for index in 0..3 {
        let panel = panels
            .as_ref()
            .map(|panels| &panels[index])
            .filter(|panel| panel.open());
        let unchanged = panel.is_some_and(|panel| {
            state.windows[index]
                .as_ref()
                .is_some_and(|old| old.key.generation == generation && old.snapshot.same(panel))
        });
        if unchanged {
            continue;
        }
        if let Some(window) = state.windows[index].take() {
            if fields
                .iter()
                .any(|(entity, field)| field.0 == index && focus.get() == Some(entity))
            {
                focus.clear();
            }
            commands.entity(window.root).despawn();
        }
        let Some(panel) = panel else {
            continue;
        };
        let Some(revision) = state.revision.checked_add(1) else {
            continue;
        };
        state.revision = revision;
        let key = Key {
            generation,
            revision,
            window: index,
        };
        let root = widgets::build(&mut commands, &skins, key, panel);
        state.windows[index] = Some(Window {
            key,
            snapshot: panel.clone(),
            root,
        });
    }
}
fn current<'a>(owner: &HostOwner, state: &'a State, control: &Control) -> Option<&'a Snapshot> {
    if owner.phase != crate::bevy_shell::owner::Phase::Playing
        || owner.generation() != control.key.generation
        || owner.modal_active()
    {
        return None;
    }
    let play = owner.play.as_ref()?;
    if play.view.gameplay_input_blocked(&play.link) {
        return None;
    }
    state
        .windows
        .get(control.key.window)?
        .as_ref()
        .filter(|window| window.key == control.key)
        .map(|window| &window.snapshot)
}
