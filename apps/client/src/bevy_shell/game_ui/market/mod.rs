//! Read-only named catalogue and offer books over the retained MARKET_V1 owner.
mod actions;
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use bevy::{input_focus::InputFocus, prelude::*, text::EditableText, ui_widgets::Activate};
use oteryn_client_ui::market::{
    MarketPanel,
    details::{Query as MarketQuery, Ware},
};
use oteryn_client_ui_bevy as source;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Key {
    generation: u64,
    revision: u64,
}
#[derive(Clone, Copy, Component)]
struct Action {
    key: Key,
    kind: Kind,
}
#[derive(Clone, Copy)]
enum Kind {
    Close,
    Search,
    Category(usize),
    Item(usize),
    Catalogue(i32),
    Book(i32),
    Statistics,
}
#[derive(Component)]
struct Overlay;
#[derive(Component)]
struct Search;
#[derive(Resource, Default)]
struct State {
    key: Option<Key>,
    seen: Option<MarketPanel>,
    selected: Option<Ware>,
    root: Option<Entity>,
    pending: Option<(Action, Entity, Entity)>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_systems(Startup, setup.after(super::setup))
        .add_systems(Update, (keyboard, project).chain())
        .add_systems(
            PostUpdate,
            actions::finish.after(bevy::text::EditableTextSystems),
        );
}
pub(super) fn open(owner: &mut HostOwner) {
    if owner.modal_active() {
        return;
    }
    if let Some(play) = &owner.play {
        play.link.market_set_open(true);
    }
}
fn setup(mut commands: Commands) {
    commands.spawn((
        Overlay,
        GameModal,
        GlobalZIndex(157),
        Node {
            display: Display::None,
            position_type: PositionType::Absolute,
            width: percent(100.),
            height: percent(100.),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0., 0., 0., 0.45)),
    ));
}
// The projection reads the retained owner, hierarchy and text-focus owner together.
#[allow(clippy::too_many_arguments)]
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skin: Res<Skins>,
    mut state: ResMut<State>,
    mut overlay: Query<(Entity, &mut Node), With<Overlay>>,
    mut focus: ResMut<InputFocus>,
    fields: Query<(), With<Search>>,
) {
    let Ok((overlay, mut node)) = overlay.single_mut() else {
        return;
    };
    let panel = owner.play.as_ref().map(|play| play.link.market_panel());
    let panel = panel.filter(|panel| panel.open);
    let Some(panel) = panel else {
        node.display = Display::None;
        if let Some(root) = state.root.take() {
            commands.entity(root).despawn();
        }
        if focus.get().is_some_and(|entity| fields.contains(entity)) {
            focus.clear();
        }
        *state = State::default();
        return;
    };
    node.display = Display::Flex;
    let generation = owner.generation();
    if state.key.is_some_and(|key| key.generation == generation)
        && state.seen.as_ref() == Some(&panel)
    {
        return;
    }
    if focus.get().is_some_and(|entity| fields.contains(entity)) {
        focus.clear();
    }
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
    }
    let revision = state.key.map_or(0, |key| key.revision.wrapping_add(1));
    if state.key.is_none_or(|key| key.generation != generation) {
        state.selected = panel
            .details
            .items
            .iter()
            .find(|ware| ware.definition == panel.definition)
            .cloned();
        if panel.selected
            && panel.details.items.is_empty()
            && panel.details.pending.is_none()
            && panel.details.feedback.is_none()
        {
            if let Some(play) = &owner.play {
                play.link.market_query(MarketQuery::Catalogue {
                    filter: panel.details.filter.clone(),
                    page: panel.details.page,
                });
            }
        }
    }
    let key = Key {
        generation,
        revision,
    };
    state.key = Some(key);
    state.pending = None;
    state.root = Some(widgets::build(
        &mut commands,
        &skin,
        overlay,
        key,
        &panel,
        state.selected.as_ref(),
    ));
    state.seen = Some(panel);
}
fn activate(
    event: On<Activate>,
    actions: Query<&Action>,
    fields: Query<(Entity, &EditableText), With<Search>>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    guard: ModalGuard,
) {
    if guard.allows(event.entity)
        && let Ok(action) = actions.get(event.entity)
    {
        actions::submit(&owner, &mut state, *action, event.entity, &fields);
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    focus: Res<InputFocus>,
    fields: Query<(Entity, &EditableText), With<Search>>,
    overlays: Query<Entity, With<Overlay>>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    guard: ModalGuard,
) {
    if owner.modal_active() {
        events.clear();
        return;
    }
    for event in events.read() {
        if event.state != bevy::input::ButtonState::Pressed || event.repeat {
            continue;
        }
        let Some(key) = state.key else {
            continue;
        };
        let Some(overlay) = overlays.iter().find(|entity| guard.allows(*entity)) else {
            continue;
        };
        let kind = match event.key_code {
            KeyCode::Escape => Kind::Close,
            KeyCode::Enter | KeyCode::NumpadEnter
                if focus.get().is_some_and(|id| fields.contains(id)) =>
            {
                Kind::Search
            }
            _ => continue,
        };
        actions::submit(&owner, &mut state, Action { key, kind }, overlay, &fields);
    }
}
