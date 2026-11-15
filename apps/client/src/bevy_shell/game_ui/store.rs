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
    Close,
    Confirm,
    Gift,
    GiftControl(usize),
    History(bool),
    Ledger(bool),
    Page(i8),
    Product(usize),
    Products,
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
    fields: [String; 3],
}
#[derive(Resource, Default)]
struct State {
    key: Option<Key>,
    seen: Option<Snapshot>,
    root: Option<Entity>,
    body: Option<Entity>,
    page: Page,
    confirmation: Option<Confirmation>,
    fields: [String; 3],
    dirty: bool,
    pending: Option<(Action, Entity)>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<TeamUi>()
        .add_systems(Update, project_teams)
        .init_resource::<State>()
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
        BackgroundColor(Color::srgba(0., 0., 0., 0.45)),
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
            actions::perform(
                &owner,
                &mut state,
                Action { key, kind },
                [String::new(), String::new(), String::new()],
            );
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

#[derive(Resource, Default)]
struct TeamUi {
    generation: u64,
    revision: u64,
    open: bool,
    root: Option<Entity>,
    seen: Option<oteryn_client_ui::team_finder::Panel>,
}
#[derive(Component, Clone, Copy)]
struct TeamControl {
    generation: u64,
    revision: u64,
    kind: TeamKind,
}
#[derive(Clone, Copy)]
enum TeamKind {
    Close,
    Select(usize),
    Request(usize),
}
fn team_button(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    state: &TeamUi,
    label: &str,
    kind: TeamKind,
    enabled: bool,
) {
    let entity = commands
        .spawn((
            source::button(skin.buttons.clone(), 170.),
            Name::new(format!("Team Finder: {label}")),
            TeamControl {
                generation: state.generation,
                revision: state.revision,
                kind,
            },
        ))
        .observe(team_activate)
        .id();
    if !enabled {
        commands.entity(entity).insert(InteractionDisabled);
    }
    let caption = commands
        .spawn(source::button_caption(label, skin.font.clone()))
        .id();
    commands.entity(entity).add_child(caption);
    commands.entity(parent).add_child(entity);
}
fn team_activate(
    event: On<Activate>,
    guard: ModalGuard,
    controls: Query<(&TeamControl, Has<InteractionDisabled>)>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<TeamUi>,
) {
    let Ok((control, false)) = controls.get(event.entity) else {
        return;
    };
    if !guard.allows(event.entity)
        || owner.modal_active()
        || !state.open
        || control.generation != owner.generation()
        || control.generation != state.generation
        || control.revision != state.revision
    {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    let panel = play.link.discovery_panel().teams;
    if state.seen.as_ref() != Some(&panel) {
        return;
    }
    match control.kind {
        TeamKind::Close => state.open = false,
        TeamKind::Select(row) => play.link.team_finder_select(row),
        TeamKind::Request(action) => {
            play.link
                .team_finder_control(action, &[], panel.selected_row);
        }
    }
}
fn project_teams(
    mut commands: Commands,
    mut owner: NonSendMut<HostOwner>,
    skin: Res<Skins>,
    mut state: ResMut<TeamUi>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    guard: ModalGuard,
) {
    let request = owner.take_team_finder_request();
    let generation = owner.generation();
    if state.generation != generation || owner.play.is_none() {
        state.open = false;
        state.generation = generation;
        state.seen = None;
    }
    if request == Some(generation) && owner.play.is_some() {
        state.open = true;
    }
    if state.open
        && !owner.modal_active()
        && state.root.is_some_and(|root| guard.allows(root))
        && keys
            .as_ref()
            .is_some_and(|keys| keys.just_pressed(KeyCode::Escape))
    {
        state.open = false;
    }
    if !state.open {
        if let Some(root) = state.root.take() {
            commands.entity(root).despawn();
        }
        state.seen = None;
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    let panel = play.link.discovery_panel().teams;
    if state.seen.as_ref() == Some(&panel) {
        return;
    }
    let Some(revision) = state.revision.checked_add(1) else {
        state.open = false;
        return;
    };
    state.revision = revision;
    let root = *state.root.get_or_insert_with(|| {
        commands
            .spawn((
                GameModal,
                GlobalZIndex(155),
                bevy::ui_widgets::ScrollArea,
                source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
                Node {
                    position_type: PositionType::Absolute,
                    left: percent(20.),
                    top: percent(10.),
                    width: percent(60.),
                    max_height: percent(80.),
                    padding: px(20.).all(),
                    row_gap: px(4.),
                    flex_direction: FlexDirection::Column,
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
            ))
            .id()
    });
    commands.entity(root).despawn_children();
    source::dialog_caption(&mut commands, root, "Team Finder", skin.font.clone());
    let text = commands
        .spawn(source::text(
            oteryn_client_ui::team_finder::panel_text(&panel),
            source::font(skin.font.clone(), false),
        ))
        .insert(TextLayout::default())
        .id();
    commands.entity(root).add_child(text);
    let ready = panel.selected && panel.pending.is_none();
    for (action, label) in [
        (1, "Browse teams"),
        (2, "My applicants"),
        (3, "Next page"),
        (4, "Apply to selected"),
        (5, "Cancel application"),
        (6, "Invite selected"),
        (7, "Accept selected"),
        (8, "Deny selected"),
        (10, "Remove advertisement"),
    ] {
        team_button(
            &mut commands,
            &skin,
            root,
            &state,
            label,
            TeamKind::Request(action),
            ready,
        );
    }
    for (row, name) in panel.visible_names().iter().enumerate() {
        team_button(
            &mut commands,
            &skin,
            root,
            &state,
            name,
            TeamKind::Select(row),
            ready,
        );
    }
    team_button(
        &mut commands,
        &skin,
        root,
        &state,
        "Close",
        TeamKind::Close,
        true,
    );
    state.seen = Some(panel);
}
