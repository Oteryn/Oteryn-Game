//! Game Store presentation over the session's retained store owners.
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{
        owner::{HostOwner, Phase},
        ui::Skins,
    },
    GameModal,
    modal::ModalGuard,
};
use crate::play::feature_0154_store::view::{StoreAction, StoreCustomization, StoreSnapshot};
use bevy::{input_focus::InputFocus, prelude::*, text::EditableText, ui_widgets::Activate};
use oteryn_client_ui::store::StorePage;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Refresh,
    Previous,
    Next,
    Category(usize),
    Select(usize),
    ReviewPurchase,
    ConfirmPurchase,
    CancelPurchase,
    History,
    Ledger,
    Filter,
    Gift,
    ReviewGift,
    ConfirmGift,
    RetryGift,
    Receipt,
    Close,
}
#[derive(Component)]
struct Control {
    generation: u64,
    revision: u64,
    action: Action,
}
#[derive(Component)]
struct ProductRow;
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Field {
    Search,
    Name,
    Sex,
    Identity,
    Filter,
    From,
    Until,
    Recipient,
    Amount,
}
#[derive(Component)]
struct Overlay;
#[derive(Resource, Default)]
struct State {
    seen: Option<StoreSnapshot>,
    generation: u64,
    revision: u64,
    root: Option<Entity>,
    consent: Option<(StoreSnapshot, StoreCustomization)>,
    draft: Vec<(Field, String)>,
    dirty: bool,
}
impl State {
    fn value(&self, field: Field) -> &str {
        self.draft
            .iter()
            .find(|(f, _)| *f == field)
            .map_or("", |(_, v)| v.as_str())
    }
    fn set(&mut self, field: Field, value: String) {
        if let Some((_, v)) = self.draft.iter_mut().find(|(f, _)| *f == field) {
            *v = value;
        } else {
            self.draft.push((field, value));
        }
    }
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain());
}
pub(in crate::bevy_shell) fn open(owner: &mut HostOwner) {
    if available(owner)
        && let Some(play) = &owner.play
    {
        play.link.store_open_bound();
    }
}
fn available(owner: &HostOwner) -> bool {
    owner.phase == Phase::Playing
        && !owner.modal_active()
        && owner
            .play
            .as_ref()
            .is_some_and(|p| !p.view.gameplay_input_blocked(&p.link))
}
fn current<'a>(owner: &'a HostOwner, state: &State) -> Option<&'a crate::play::PlayLink> {
    if !available(owner) || owner.generation() != state.generation {
        return None;
    }
    let play = owner.play.as_ref()?;
    let expected = state.seen.as_ref()?;
    let actual = play.link.store_snapshot();
    (actual == *expected
        && actual.panel.open
        && actual.panel.selected
        && !actual.panel.pending
        && !actual.history.pending
        && actual.panel.gift.pending.is_none())
    .then_some(&play.link)
}
fn inside(mut entity: Option<Entity>, root: Entity, parents: &Query<&ChildOf>) -> bool {
    while let Some(current) = entity {
        if current == root {
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
    let snapshot = owner
        .play
        .as_ref()
        .map(|p| p.link.store_snapshot())
        .filter(|s| s.panel.open && owner.phase == Phase::Playing);
    let Some(snapshot) = snapshot else {
        if let Some(root) = state.root.take() {
            commands.entity(root).despawn();
            if inside(focus.get(), root, &parents) {
                focus.clear();
            }
        }
        state.seen = None;
        state.consent = None;
        state.draft.clear();
        return;
    };
    if state.generation != owner.generation()
        || state
            .seen
            .as_ref()
            .is_some_and(|s| !s.same_session(&snapshot))
    {
        state.consent = None;
        state.draft.clear();
    }
    if state.seen.as_ref() == Some(&snapshot)
        && state.generation == owner.generation()
        && !state.dirty
    {
        return;
    }
    if state.consent.as_ref().is_some_and(|(s, _)| *s != snapshot) {
        state.consent = None;
    }
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
        if inside(focus.get(), root, &parents) {
            focus.clear();
        }
    }
    if let Some(quote) = &snapshot.panel.gift.quote
        && state
            .seen
            .as_ref()
            .and_then(|s| s.panel.gift.quote.as_ref())
            != Some(quote)
    {
        state.set(Field::Recipient, quote.recipient.clone());
        state.set(Field::Amount, quote.amount.to_string());
    }
    state.generation = owner.generation();
    state.revision = state.revision.wrapping_add(1);
    state.seen = Some(snapshot);
    state.dirty = false;
    state.root = Some(widgets::build(&mut commands, &skins, &state));
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    fields: Query<(&Field, &EditableText)>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if !guard.allows(event.entity)
        || control.generation != state.generation
        || control.revision != state.revision
    {
        return;
    }
    // Do not read text ahead of Bevy's edit/paste application.
    if fields.iter().any(|(_, t)| {
        t.pending_paste.is_some()
            || t.pending_edits
                .iter()
                .any(bevy::text::TextEdit::is_destructive)
    }) {
        return;
    }
    if current(&owner, &state).is_none() {
        return;
    }
    for (field, text) in fields.iter() {
        state.set(*field, text.value().to_string());
    }
    perform(&owner, &mut state, control.action);
}
fn customization(state: &State) -> Option<StoreCustomization> {
    let raw = state.value(Field::Identity).replace('-', "");
    let identity = if raw.is_empty() {
        None
    } else {
        if raw.len() != 32 || !raw.is_ascii() {
            return None;
        }
        let mut id = [0; 16];
        for (i, b) in id.iter_mut().enumerate() {
            *b = u8::from_str_radix(&raw[i * 2..i * 2 + 2], 16).ok()?;
        }
        Some(id)
    };
    let sex = if state.value(Field::Sex).is_empty() {
        0
    } else {
        state.value(Field::Sex).parse().ok()?
    };
    Some(StoreCustomization {
        name: state.value(Field::Name).into(),
        sex,
        identity,
    })
}
fn perform(owner: &HostOwner, state: &mut State, action: Action) {
    let Some(link) = current(owner, state) else {
        return;
    };
    let Some(expected) = state.seen.as_ref() else {
        return;
    };
    let panel = &expected.panel;
    let dispatch = match action {
        Action::ReviewPurchase => {
            if panel.confirmed_purchase.is_some()
                || panel
                    .catalogue
                    .as_ref()
                    .and_then(|p| p.offers.get(panel.selected_offer))
                    .is_none()
            {
                return;
            }
            let Some(custom) = customization(state) else {
                return;
            };
            state.consent = Some((expected.clone(), custom));
            state.dirty = true;
            return;
        }
        Action::CancelPurchase => {
            state.consent = None;
            state.dirty = true;
            return;
        }
        Action::ConfirmPurchase => {
            let Some((quote, custom)) = &state.consent else {
                return;
            };
            if quote != expected {
                return;
            }
            StoreAction::Purchase(custom.clone())
        }
        Action::Close => StoreAction::Close,
        Action::Select(index) => StoreAction::Select(index),
        Action::History => StoreAction::History(false),
        Action::Ledger => StoreAction::Ledger(false),
        Action::Gift => StoreAction::GiftOpen,
        Action::Filter => StoreAction::Filter {
            ledger: panel.page == StorePage::CoinHistory,
            key: state.value(Field::Filter).into(),
            from: state.value(Field::From).into(),
            until: state.value(Field::Until).into(),
        },
        Action::ReviewGift | Action::ConfirmGift | Action::RetryGift | Action::Receipt => {
            StoreAction::Gift {
                action: match action {
                    Action::ReviewGift => 0,
                    Action::ConfirmGift => 1,
                    Action::RetryGift => 3,
                    _ => 4,
                },
                fields: [
                    state.value(Field::Recipient).into(),
                    state.value(Field::Amount).into(),
                ],
            }
        }
        Action::Refresh | Action::Previous | Action::Next | Action::Category(_) => {
            if matches!(action, Action::Next) && panel.page == StorePage::History {
                StoreAction::History(true)
            } else if matches!(action, Action::Next) && panel.page == StorePage::CoinHistory {
                StoreAction::Ledger(true)
            } else {
                let category = if let Action::Category(i) = action {
                    let Some(c) = panel
                        .browser
                        .page
                        .as_ref()
                        .and_then(|p| p.categories.get(i))
                    else {
                        return;
                    };
                    c.key.clone()
                } else {
                    panel.browser.category.clone()
                };
                StoreAction::Browse {
                    category,
                    search: state.value(Field::Search).into(),
                    sort: panel.browser.sort,
                    step: match action {
                        Action::Previous => -1,
                        Action::Next => 1,
                        _ => 0,
                    },
                }
            }
        }
    };
    if link.store_dispatch_bound(expected, dispatch) {
        state.consent = None;
        state.dirty = true;
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    let close = events.read().any(|e| {
        e.key_code == KeyCode::Escape && e.state == bevy::input::ButtonState::Pressed && !e.repeat
    });
    if close && state.root.is_some_and(|r| guard.allows(r)) {
        perform(&owner, &mut state, Action::Close);
    }
}
