//! Offer editing uses the retained market intents and the currently displayed server rows.
mod widgets;
use super::*;
use bevy::ui::InteractionDisabled;
use oteryn_client_ui::market::market::{self, MarketRow};
pub(super) use widgets::{editor, list, select_button};
#[derive(Clone, Copy, Component)]
struct Control {
    key: Key,
    operation: Operation,
}
#[derive(Clone, Copy)]
enum Operation {
    Select(usize),
    Cancel,
    Accept,
    Create,
    Sell(bool),
}
#[derive(Clone, Copy, Component, PartialEq, Eq)]
pub(super) enum Field {
    Amount,
    Price,
    Quantity,
}
#[derive(Component)]
struct Summary;
#[derive(Resource, Default)]
struct Pending(Option<(Entity, Control, Vec<Entity>)>);
pub(super) fn install(app: &mut App) {
    app.init_resource::<Pending>().add_systems(
        PostUpdate,
        (finish, validate)
            .chain()
            .after(bevy::text::EditableTextSystems),
    );
}
fn current(owner: &HostOwner, state: &State, key: Key) -> Option<MarketPanel> {
    let play = owner.play.as_ref()?;
    let panel = play.link.market_panel();
    (state.key == Some(key)
        && key.generation == owner.generation()
        && owner.phase == crate::bevy_shell::owner::Phase::Playing
        && !owner.modal_active()
        && !play.view.gameplay_input_blocked(&play.link)
        && panel.open
        && panel.selected
        && panel.pending.is_none()
        && panel.details.pending.is_none()
        && state.seen.as_ref() == Some(&panel))
    .then_some(panel)
}
fn selected<'a>(state: &State, panel: &'a MarketPanel) -> Option<&'a MarketRow> {
    panel.rows.get(state.offer?).filter(|row| {
        row.state == 1 && row.amount > 0 && row.price > 0 && matches!(row.side, 1 | 2)
    })
}
fn limit(panel: &MarketPanel, row: &MarketRow) -> u32 {
    let mut maximum = row.amount.min(64_000);
    if row.side == 1
        && let Some(ware) = panel
            .details
            .items
            .iter()
            .find(|ware| ware.definition == row.definition)
    {
        maximum = maximum.min(u32::try_from(ware.depot_units).unwrap_or(u32::MAX));
    }
    maximum
}
fn amount(panel: &MarketPanel, row: &MarketRow, value: &str) -> Option<u32> {
    let maximum = limit(panel, row);
    if maximum == 0 {
        return None;
    }
    Some(value.trim().parse::<u32>().ok()?.clamp(1, maximum))
}
fn create(
    state: &State,
    panel: &MarketPanel,
    quantity: &str,
    price: &str,
) -> Option<market::MarketRequest> {
    let ware = state.selected.as_ref()?;
    if panel.view != MarketView::Browse || !panel.details.items.contains(ware) {
        return None;
    }
    let request = market::MarketRequest {
        action: if state.sell.unwrap_or(panel.sell) {
            6
        } else {
            5
        },
        definition: ware.definition,
        offer: [0; 16],
        amount: quantity.trim().parse().ok()?,
        price: price.trim().parse().ok()?,
        anonymous: false,
        page: 0,
    };
    market::encode_request(request).ok()?;
    Some(request)
}
fn required(operation: Operation) -> &'static [Field] {
    match operation {
        Operation::Create => &[Field::Amount, Field::Price],
        Operation::Accept => &[Field::Quantity],
        Operation::Cancel | Operation::Select(_) | Operation::Sell(_) => &[],
    }
}
fn editing(text: &EditableText) -> bool {
    text.pending_paste.is_some()
        || text
            .pending_edits
            .iter()
            .any(bevy::text::TextEdit::is_destructive)
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    fields: Query<(Entity, &Field, &EditableText)>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    mut pending: ResMut<Pending>,
    guard: ModalGuard,
) {
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if !guard.allows(event.entity) || current(&owner, &state, control.key).is_none() {
        return;
    }
    pending.0 = None;
    match control.operation {
        Operation::Select(index) => {
            let Some(panel) = current(&owner, &state, control.key) else {
                return;
            };
            if panel.view != MarketView::History
                && panel.rows.get(index).is_some_and(|row| row.state == 1)
            {
                state.offer = Some(index);
                state.dirty = true;
            }
        }
        Operation::Sell(sell) => state.sell = Some(sell),
        operation => {
            let mut editors = Vec::new();
            for needed in required(operation) {
                let Some((entity, _, _)) = fields.iter().find(|(_, field, _)| *field == needed)
                else {
                    return;
                };
                editors.push(entity);
            }
            pending.0 = Some((event.entity, *control, editors));
        }
    }
}
fn finish(
    owner: NonSend<HostOwner>,
    state: Res<State>,
    mut pending: ResMut<Pending>,
    fields: Query<(Entity, &Field, &EditableText)>,
    controls: Query<&Control>,
    guard: ModalGuard,
) {
    let Some((source, control, editors)) = pending.0.take() else {
        return;
    };
    let Some(panel) = current(&owner, &state, control.key) else {
        return;
    };
    if !guard.allows(source) || !controls.get(source).is_ok_and(|c| c.key == control.key) {
        return;
    }
    let mut values = Vec::new();
    for (entity, expected) in editors.iter().zip(required(control.operation)) {
        let Ok((_, field, text)) = fields.get(*entity) else {
            return;
        };
        if field != expected || !guard.allows(*entity) {
            return;
        }
        if editing(text) {
            pending.0 = Some((source, control, editors));
            return;
        }
        values.push(text.value().to_string());
    }
    let Some(play) = &owner.play else {
        return;
    };
    let intent = match control.operation {
        Operation::Create => {
            let [quantity, price] = values.as_slice() else {
                return;
            };
            let Some(request) = create(&state, &panel, quantity, price) else {
                return;
            };
            if !play.link.market_details(
                &request.definition.to_string(),
                quantity,
                price,
                request.action == 6,
                false,
            ) {
                return;
            }
            Intent::CreateOffer
        }
        Operation::Accept | Operation::Cancel => {
            let Some(row) = selected(&state, &panel) else {
                return;
            };
            let intent = if matches!(control.operation, Operation::Cancel) {
                if panel.view != MarketView::Own {
                    return;
                }
                Intent::CancelOffer
            } else {
                if panel.view != MarketView::Browse || row.definition != panel.definition {
                    return;
                }
                let [quantity] = values.as_slice() else {
                    return;
                };
                let Some(amount) = amount(&panel, row, quantity) else {
                    return;
                };
                if row.side == 2 {
                    play.link.market_buy_bound(&panel, row, amount);
                    return;
                }
                if !play.link.market_details(
                    &row.definition.to_string(),
                    &amount.to_string(),
                    "1",
                    panel.sell,
                    panel.anonymous,
                ) {
                    return;
                }
                Intent::AcceptOffer
            };
            let Some(index) = oteryn_client_ui::market::visible_rows(&panel)
                .iter()
                .position(|index| Some(*index) == state.offer)
            else {
                return;
            };
            play.link.market_select_offer(index);
            if play.link.market_panel().selected_offer != Some(row.offer) {
                return;
            }
            intent
        }
        Operation::Select(_) | Operation::Sell(_) => return,
    };
    play.link.market_request(intent);
}
fn validate(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    state: Res<State>,
    controls: Query<(Entity, &Control, Has<InteractionDisabled>)>,
    mut fields: Query<(&Field, &mut EditableText)>,
    mut summary: Query<&mut Text, With<Summary>>,
) {
    let Some(key) = state.key else {
        return;
    };
    let panel = current(&owner, &state, key);
    if let Some(panel) = &panel
        && let Some(row) = selected(&state, panel)
    {
        for (field, mut text) in &mut fields {
            if *field == Field::Quantity
                && !editing(&text)
                && let Some(amount) = amount(panel, row, &text.value().to_string())
                && text.value().to_string() != amount.to_string()
            {
                text.editor_mut().set_text(&amount.to_string());
            }
        }
    }
    let value = |wanted| {
        fields
            .iter()
            .find(|(field, _)| **field == wanted)
            .filter(|(_, text)| !editing(text))
            .map(|(_, text)| text.value().to_string())
    };
    let request = panel
        .as_ref()
        .and_then(|panel| create(&state, panel, &value(Field::Amount)?, &value(Field::Price)?));
    for mut text in &mut summary {
        text.0 = if let Some(request) = request {
            format!(
                "{} offer · Total: {} gold",
                if request.action == 6 { "Sell" } else { "Buy" },
                u128::from(request.amount) * u128::from(request.price)
            )
        } else {
            "Enter an item, amount (1–64000) and price ≥ 1; total ≤ 999999999999 gold.".into()
        };
    }
    for (entity, control, disabled) in &controls {
        let enabled = panel.as_ref().is_some_and(|panel| {
            control.key == key
                && match control.operation {
                    Operation::Create => request.is_some(),
                    Operation::Accept => {
                        panel.view == MarketView::Browse
                            && selected(&state, panel).is_some_and(|row| {
                                value(Field::Quantity)
                                    .and_then(|value| amount(panel, row, &value))
                                    .is_some()
                            })
                    }
                    Operation::Cancel => {
                        panel.view == MarketView::Own && selected(&state, panel).is_some()
                    }
                    Operation::Select(index) => {
                        panel.view != MarketView::History
                            && panel.rows.get(index).is_some_and(|row| row.state == 1)
                    }
                    Operation::Sell(_) => panel.view == MarketView::Browse,
                }
        });
        if enabled && disabled {
            commands.entity(entity).remove::<InteractionDisabled>();
        } else if !enabled && !disabled {
            commands.entity(entity).insert(InteractionDisabled);
        }
    }
}
