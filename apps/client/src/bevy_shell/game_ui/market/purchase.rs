//! Local purchase consent is bound to the exact accepted book; settlement remains server-owned.
mod widgets;
use super::*;
use bevy::input::keyboard::KeyboardInput;
use oteryn_client_runtime::market::market::MarketRow;
#[derive(Component, Clone)]
struct Row {
    key: Key,
    offer: MarketRow,
}
#[derive(Clone)]
struct Quote {
    key: Key,
    panel: MarketPanel,
    ware: Ware,
    offer: MarketRow,
}
#[derive(Component)]
struct Quantity;
#[derive(Clone, Copy, Component)]
struct Control {
    revision: u64,
    kind: ControlKind,
}
#[derive(Clone, Copy)]
enum ControlKind {
    Review,
    Confirm,
    Cancel,
}
#[derive(Resource, Default)]
struct PurchaseState {
    quote: Option<Quote>,
    confirmation: Option<(u32, u64)>,
    quantity: String,
    feedback: String,
    revision: u64,
    dirty: bool,
    root: Option<Entity>,
    pending: Option<(u64, Entity, Entity)>,
}
impl PurchaseState {
    fn clear(&mut self) {
        self.quote = None;
        self.confirmation = None;
        self.pending = None;
        self.revision = self.revision.wrapping_add(1);
        self.dirty = true;
    }
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<PurchaseState>()
        .add_message::<KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain().after(super::project))
        .add_systems(PostUpdate, finish.after(bevy::text::EditableTextSystems));
}
pub(super) fn row(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    key: Key,
    offer: &MarketRow,
) {
    let entity = commands
        .spawn((
            source::button(skin.buttons.clone(), 70.),
            Row {
                key,
                offer: offer.clone(),
            },
            Name::new("market.purchase.row"),
        ))
        .observe(select)
        .id();
    let caption = commands
        .spawn(source::button_caption("Buy…", skin.font.clone()))
        .id();
    commands.entity(entity).add_child(caption);
    commands.entity(parent).add_child(entity);
}
fn available(owner: &HostOwner) -> bool {
    owner.phase == crate::bevy_shell::owner::Phase::Playing
        && !owner.modal_active()
        && owner
            .play
            .as_ref()
            .is_some_and(|play| !play.view.gameplay_input_blocked(&play.link))
}
fn valid(owner: &HostOwner, market: &State, quote: &Quote) -> bool {
    available(owner)
        && owner.generation() == quote.key.generation
        && market.key == Some(quote.key)
        && owner
            .play
            .as_ref()
            .is_some_and(|play| play.link.market_panel() == quote.panel)
}
fn select(
    event: On<Activate>,
    rows: Query<&Row>,
    owner: NonSend<HostOwner>,
    market: Res<State>,
    mut state: ResMut<PurchaseState>,
    guard: ModalGuard,
) {
    let Ok(row) = rows.get(event.entity) else {
        return;
    };
    if !guard.allows(event.entity)
        || !available(&owner)
        || market.key != Some(row.key)
        || owner.generation() != row.key.generation
    {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    let panel = play.link.market_panel();
    let Some(ware) = market
        .selected
        .as_ref()
        .filter(|ware| ware.definition == row.offer.definition)
    else {
        return;
    };
    if market.seen.as_ref() != Some(&panel)
        || !panel.open
        || !panel.selected
        || panel.view != oteryn_client_ui::market::MarketView::Browse
        || panel.pending.is_some()
        || panel.details.pending.is_some()
        || panel.definition != row.offer.definition
        || row.offer.side != 2
        || row.offer.state != 1
        || row.offer.amount == 0
        || row.offer.price == 0
        || !panel.rows.contains(&row.offer)
    {
        return;
    }
    state.clear();
    state.quote = Some(Quote {
        key: row.key,
        panel,
        ware: ware.clone(),
        offer: row.offer.clone(),
    });
    state.quantity = "1".into();
    state.feedback.clear();
}
fn editing(text: &EditableText) -> bool {
    text.pending_paste.is_some()
        || text
            .pending_edits
            .iter()
            .any(bevy::text::TextEdit::is_destructive)
}
fn review(state: &mut PurchaseState, value: &str) {
    let Some(quote) = &state.quote else {
        return;
    };
    state.quantity = value.into();
    state.feedback.clear();
    let amount = value
        .trim()
        .parse::<u32>()
        .ok()
        .filter(|amount| (1..=64_000).contains(amount) && *amount <= quote.offer.amount);
    let total = amount
        .and_then(|amount| quote.offer.price.checked_mul(u64::from(amount)))
        .filter(|total| *total <= 999_999_999_999);
    if let (Some(amount), Some(total)) = (amount, total) {
        state.confirmation = Some((amount, total));
    } else {
        state.feedback = format!(
            "Enter an amount between 1 and {}.",
            quote.offer.amount.min(64_000)
        );
    }
    state.revision = state.revision.wrapping_add(1);
    state.dirty = true;
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    fields: Query<(Entity, &EditableText), With<Quantity>>,
    owner: NonSend<HostOwner>,
    market: Res<State>,
    mut state: ResMut<PurchaseState>,
    guard: ModalGuard,
) {
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if !guard.allows(event.entity) || control.revision != state.revision {
        return;
    }
    if matches!(control.kind, ControlKind::Cancel) {
        state.clear();
        return;
    }
    let Some(quote) = state
        .quote
        .as_ref()
        .filter(|quote| valid(&owner, &market, quote))
    else {
        state.clear();
        return;
    };
    match control.kind {
        ControlKind::Review => {
            let Ok((field, text)) = fields.single() else {
                return;
            };
            if !guard.allows(field) {
                return;
            }
            if editing(text) {
                state.pending = Some((state.revision, event.entity, field));
                return;
            }
            review(&mut state, &text.value().to_string());
        }
        ControlKind::Confirm => {
            let Some((amount, _)) = state.confirmation else {
                return;
            };
            if let Some(play) = &owner.play {
                play.link
                    .market_buy_bound(&quote.panel, &quote.offer, amount);
            }
            // Consume this consent even if authority changed during the atomic owner check.
            state.clear();
        }
        ControlKind::Cancel => {}
    }
}
fn finish(
    owner: NonSend<HostOwner>,
    market: Res<State>,
    mut state: ResMut<PurchaseState>,
    fields: Query<&EditableText, With<Quantity>>,
    controls: Query<&Control>,
    guard: ModalGuard,
) {
    let Some((revision, source, field)) = state.pending.take() else {
        return;
    };
    if revision != state.revision
        || !guard.allows(source)
        || !guard.allows(field)
        || !controls
            .get(source)
            .is_ok_and(|control| control.revision == revision)
        || !state
            .quote
            .as_ref()
            .is_some_and(|quote| valid(&owner, &market, quote))
    {
        return;
    }
    let Ok(text) = fields.get(field) else {
        return;
    };
    if editing(text) {
        state.pending = Some((revision, source, field));
        return;
    }
    review(&mut state, &text.value().to_string());
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    market: Res<State>,
    skin: Res<Skins>,
    mut state: ResMut<PurchaseState>,
    mut focus: ResMut<InputFocus>,
    fields: Query<(), With<Quantity>>,
) {
    if state
        .quote
        .as_ref()
        .is_some_and(|quote| !valid(&owner, &market, quote))
    {
        state.clear();
    }
    if !state.dirty {
        return;
    }
    if focus.get().is_some_and(|entity| fields.contains(entity)) {
        focus.clear();
    }
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
    }
    state.dirty = false;
    if let Some(quote) = &state.quote {
        state.root = Some(widgets::build(&mut commands, &skin, quote, &state));
    }
}
fn keyboard(
    mut events: MessageReader<KeyboardInput>,
    mut state: ResMut<PurchaseState>,
    guard: ModalGuard,
) {
    for event in events.read() {
        if event.state == bevy::input::ButtonState::Pressed
            && !event.repeat
            && event.key_code == KeyCode::Escape
            && state.root.is_some_and(|root| guard.allows(root))
        {
            state.clear();
        }
    }
}
