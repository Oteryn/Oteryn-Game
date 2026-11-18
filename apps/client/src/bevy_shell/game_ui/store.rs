//! Coin store window over the retained catalogue, purchase and coin-transfer owners.
mod actions;
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use bevy::{
    input_focus::InputFocus, prelude::*, text::EditableText, ui::InteractionDisabled,
    ui_widgets::Activate,
};
use oteryn_client_runtime::cui3::store::StoreCataloguePage;
use oteryn_client_ui::{
    purchase_history,
    store::{self, StorePanel},
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Buy,
    Cancel,
    Category(usize),
    ClearSearch,
    Close,
    Confirm,
    Gift,
    GiftControl(usize),
    History(bool),
    Ledger(bool),
    Page(i8),
    Product(usize),
    Products,
    RetryPurchase,
    Search,
    Sort(u8),
    TokenPayment,
    Transfer,
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Page {
    #[default]
    Products,
    Gift,
    History,
    Ledger,
    Transfer,
}
#[derive(Component)]
struct Overlay;
#[derive(Component)]
struct Field(usize);
#[derive(Clone, PartialEq, Eq)]
struct Snapshot {
    store: StorePanel,
    history: purchase_history::Panel,
}
impl Snapshot {
    fn read(link: &crate::play::PlayLink) -> Self {
        Self {
            store: link.store_panel(),
            history: link.discovery_panel().history,
        }
    }
}
struct Confirmation {
    quote: StoreCataloguePage,
    fields: [String; 4],
}
#[derive(Resource, Default)]
struct State {
    key: Option<Key>,
    seen: Option<Snapshot>,
    root: Option<Entity>,
    body: Option<Entity>,
    page: Page,
    confirmation: Option<Confirmation>,
    fields: [String; 4],
    dirty: bool,
    pending: Option<(Action, Entity)>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Startup, setup.after(super::setup))
        .add_systems(Update, (keyboard, project).chain())
        .add_systems(
            PostUpdate,
            actions::finish.after(bevy::text::EditableTextSystems),
        );
}
fn setup(mut commands: Commands) {
    commands.spawn((
        Overlay,
        GameModal,
        crate::bevy_shell::ui::window_layout::WindowLayer("store"),
        GlobalZIndex(158),
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
// Projection jointly owns the hierarchy, snapshot and focus invalidation.
#[allow(clippy::too_many_arguments)]
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skin: Res<Skins>,
    mut state: ResMut<State>,
    mut overlays: Query<(Entity, &mut Node), With<Overlay>>,
    mut focus: ResMut<InputFocus>,
    fields: Query<(Entity, &Field, &EditableText)>,
) {
    let Ok((overlay, mut node)) = overlays.single_mut() else {
        return;
    };
    let snapshot = owner
        .play
        .as_ref()
        .map(|p| Snapshot::read(&p.link))
        .filter(|s| s.store.open);
    let Some(snapshot) = snapshot else {
        node.display = Display::None;
        if let Some(root) = state.root.take() {
            commands.entity(root).despawn();
        }
        if focus.get().is_some_and(|id| fields.contains(id)) {
            focus.clear();
        }
        *state = State::default();
        return;
    };
    node.display = Display::Flex;
    if snapshot.store.page == store::StorePage::History
        && state
            .seen
            .as_ref()
            .is_none_or(|seen| seen.store.page != store::StorePage::History)
    {
        state.page = Page::History;
    }
    let generation = owner.generation();
    let same_session = state.key.is_some_and(|key| key.generation == generation);
    if same_session && state.seen.as_ref() == Some(&snapshot) && !state.dirty {
        return;
    }
    if !same_session {
        if let Some(root) = state.root.take() {
            commands.entity(root).despawn();
        }
        *state = State::default();
        state.fields[3] = snapshot.store.browser.search.clone();
    } else {
        // Actions already captured their input; do not overwrite a newly selected form.
        if !state.dirty {
            for (_, field, text) in &fields {
                state.fields[field.0] = text.value().to_string();
            }
        }
        if matches!(state.page, Page::Gift | Page::Transfer)
            && state
                .seen
                .as_ref()
                .is_some_and(|seen| seen.store.gift_fields != snapshot.store.gift_fields)
        {
            state.fields[0] = snapshot.store.gift_fields[0].clone();
            state.fields[1] = snapshot.store.gift_fields[1].clone();
        }
        if state.seen.as_ref() != Some(&snapshot) {
            state.confirmation = None;
        }
    }
    if focus.get().is_some_and(|id| fields.contains(id)) {
        focus.clear();
    }
    if !same_session && snapshot.store.page == store::StorePage::History {
        state.page = Page::History;
    }
    let key = Key {
        generation,
        revision: state.key.map_or(0, |key| key.revision.wrapping_add(1)),
    };
    state.key = Some(key);
    state.pending = None;
    state.dirty = false;
    let root = *state
        .root
        .get_or_insert_with(|| widgets::build(&mut commands, &skin, overlay));
    if let Some(body) = state.body.take() {
        commands.entity(body).despawn();
    }
    state.body = Some(widgets::render(
        &mut commands,
        &skin,
        root,
        key,
        &state,
        &snapshot,
    ));
    // Refresh negotiated token balance only after the current store request completes.
    if snapshot.store.token_balance_refresh
        && !snapshot.store.pending
        && let Some(play) = &owner.play
    {
        play.link.store_request(6187, Vec::new());
    }
    state.seen = Some(snapshot);
}
fn activate(
    event: On<Activate>,
    actions: Query<(&Action, Has<InteractionDisabled>)>,
    fields: Query<(Entity, &Field, &EditableText)>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    guard: ModalGuard,
) {
    if guard.allows(event.entity)
        && let Ok((action, false)) = actions.get(event.entity)
    {
        actions::submit(&owner, &mut state, *action, event.entity, &fields);
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    overlays: Query<Entity, With<Overlay>>,
    guard: ModalGuard,
) {
    for event in events.read() {
        if event.state != bevy::input::ButtonState::Pressed
            || event.repeat
            || event.key_code != KeyCode::Escape
        {
            continue;
        }
        if let Some(key) = state.key
            && overlays.iter().any(|e| guard.allows(e))
        {
            let kind = if state.confirmation.is_some() {
                Kind::Cancel
            } else {
                Kind::Close
            };
            actions::perform(&owner, &mut state, Action { key, kind }, Default::default());
        }
    }
}
fn buy_enabled(panel: &StorePanel) -> bool {
    if !panel.selected || panel.pending || panel.gift.pending.is_some() {
        return false;
    }
    let Some(quote) = store::confirmation_quote(panel) else {
        return false;
    };
    let Some(offer) = quote.offers.first() else {
        return false;
    };
    let balance = match offer.currency.as_str() {
        "TIBIA_COIN" => Some(quote.total_coins),
        "COSMETIC_TOKEN" if panel.token_supported => panel.cosmetic_tokens,
        _ => None,
    };
    balance.is_some_and(|balance| balance >= offer.price)
}
