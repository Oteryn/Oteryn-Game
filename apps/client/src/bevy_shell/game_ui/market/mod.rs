//! Named catalogue, offer books and confirmed purchases over the retained MARKET_V1 owner.
mod actions;
mod offers;
mod purchase;
mod refresh;
mod statistics;
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
    Intent, MarketPanel, MarketView,
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
    Intent(Intent),
    Browse,
}
#[derive(Component)]
struct Overlay;
#[derive(Component)]
struct Search;
type Editors = Or<(
    With<Search>,
    With<offers::Field>,
    With<widgets::FilterEditor>,
)>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum DraftRole {
    Search,
    Filter(widgets::FilterKind),
    Depot,
    Offer(Option<u32>, offers::Field),
}

#[derive(Component)]
struct Draft {
    role: DraftRole,
    initial: String,
}

// Keep the editor entity (including selection, focus and pending edits) when
// the same field still represents the same retained value after a projection.
#[derive(Default)]
struct Drafts<'a>(Vec<(Entity, &'a Draft)>);

impl Drafts<'_> {
    fn field(
        &mut self,
        commands: &mut Commands,
        role: DraftRole,
        initial: &str,
        spawn: impl FnOnce(&mut Commands) -> Entity,
    ) -> Entity {
        if let Some(index) = self
            .0
            .iter()
            .position(|(_, d)| d.role == role && d.initial == initial)
        {
            return self.0.swap_remove(index).0;
        }
        let field = spawn(commands);
        commands.entity(field).insert(Draft {
            role,
            initial: initial.into(),
        });
        field
    }
}
#[derive(Resource, Default)]
struct State {
    key: Option<Key>,
    seen: Option<MarketPanel>,
    selected: Option<Ware>,
    offer: Option<usize>,
    sell: Option<bool>,
    dirty: bool,
    root: Option<Entity>,
    body: Option<Entity>,
    pending: Option<(Action, Entity, Entity)>,
}
pub(super) fn install(app: &mut App) {
    offers::install(app);
    purchase::install(app);
    app.init_resource::<State>()
        .init_resource::<refresh::Refresh>()
        .add_systems(Startup, setup.after(super::setup))
        .add_systems(Update, (keyboard, refresh::own, project).chain())
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
        crate::bevy_shell::ui::window_layout::WindowLayer("market"),
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
    fields: Query<(), Editors>,
    previous: Query<(Entity, &Draft)>,
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
        && !state.dirty
    {
        return;
    }
    let same_session = state.key.is_some_and(|key| key.generation == generation);
    let mut drafts = Drafts(previous.iter().filter(|_| same_session).collect());
    if state.seen.as_ref() != Some(&panel) {
        state.offer = None;
    }
    state.dirty = false;
    let revision = state.key.map_or(0, |key| key.revision.wrapping_add(1));
    if state.key.is_none_or(|key| key.generation != generation) {
        if let Some(root) = state.root.take() {
            commands.entity(root).despawn();
        }
        state.body = None;
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
            && let Some(play) = &owner.play
        {
            play.link.market_query(MarketQuery::Catalogue {
                filter: panel.details.filter.clone(),
                page: panel.details.page,
            });
        }
    }
    let key = Key {
        generation,
        revision,
    };
    state.key = Some(key);
    state.pending = None;
    let root = *state
        .root
        .get_or_insert_with(|| widgets::build(&mut commands, &skin, overlay));
    if let Some(body) = state.body.take() {
        // Retain live editors while the old scroll areas publish their final offsets.
        for (field, _) in &drafts.0 {
            commands.entity(*field).remove::<ChildOf>();
        }
        commands.entity(body).despawn();
    }
    state.body = Some(widgets::render(
        &mut commands,
        &skin,
        root,
        key,
        &panel,
        state.selected.as_ref(),
        state.offer,
        state.sell.unwrap_or(panel.sell),
        &mut drafts,
    ));
    for (obsolete, _) in drafts.0 {
        commands.entity(obsolete).despawn();
    }
    if let Some(entity) = focus.get().filter(|id| fields.contains(*id)) {
        commands.queue(move |world: &mut World| {
            if world.get_entity(entity).is_err()
                && world.resource::<InputFocus>().get() == Some(entity)
            {
                world.resource_mut::<InputFocus>().clear();
            }
        });
    }
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
