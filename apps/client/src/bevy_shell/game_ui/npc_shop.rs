//! Source-shaped NPC trade controls over retained, server-issued quotes.
//! NpcDialog.qml is a floating dialog: opening this panel does not stop world movement.
use super::{
    super::{owner::HostOwner, ui::Skins},
    modal::ModalGuard,
};
use bevy::{input_focus::InputFocus, prelude::*, text::EditableText, ui_widgets::Activate};
use oteryn_client_ui::npc_shop::{LiveShop, NpcTradeSide, Quote};
use oteryn_client_ui_bevy as source;
mod conversation;
mod sales;
mod submission;
#[cfg(test)]
mod tests;
mod widgets;
#[derive(Component)]
struct Panel;
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Field {
    Search,
    Amount,
}
#[derive(Component, Clone, Copy)]
struct Action {
    generation: u64,
    kind: Kind,
}
#[derive(Clone, Copy)]
enum Kind {
    Side(NpcTradeSide),
    Select(Quote),
    Trade,
    Look,
    Cyclopedia,
    SellAll,
    Close,
    IgnoreCapacity,
    OnlyOwned,
    BuyInBackpacks,
    IncludeEquipped,
    SortName,
    SortWeight,
    SortPrice,
    OpenBackpack,
}
#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum Sort {
    #[default]
    Name,
    Price,
    Weight,
}
struct RenderedOffers {
    sell_items: std::collections::BTreeSet<u32>,
    shop: LiveShop,
    side: Option<NpcTradeSide>,
    query: String,
    selection: Option<Quote>,
    rows: Vec<Quote>,
    sort: Sort,
}

#[derive(Resource, Default)]
struct State {
    generation: Option<u64>,
    selection: Option<Quote>,
    side: Option<NpcTradeSide>,
    query: String,
    feedback: String,
    widgets: Option<widgets::Widgets>,
    rendered: Option<RenderedOffers>,
    sort: Sort,
    sale_inventory: Option<(u32, u64)>,
    sales: std::collections::VecDeque<Quote>,
    pending: Option<submission::Pending>,
    conversation: conversation::Ui,
}
pub(super) fn install(app: &mut App) {
    conversation::install(app);
    app.init_resource::<State>()
        .add_systems(Update, (project, conversation::project).chain())
        .add_systems(
            PostUpdate,
            submission::finish.after(bevy::text::EditableTextSystems),
        );
}
fn perform(owner: &mut HostOwner, state: &mut State, action: Action, amount: &str) -> bool {
    if action.generation != owner.generation() || owner.modal_active() {
        return false;
    }
    let Some(play) = &owner.play else {
        return false;
    };
    if matches!(action.kind, Kind::Close) {
        let visible = play.link.npc_shop_is_open() || play.link.npc_dialogue_is_visible();
        state.sales.clear();
        play.link.npc_dialogue_close();
        state.selection = None;
        state.conversation = conversation::Ui::default();
        return visible;
    }
    if !play.link.npc_shop_is_open() {
        return false;
    }
    let panel = play.link.npc_shop_panel();
    if !panel.live.selected && !panel.selected {
        return false;
    }
    if panel.pending || panel.live.pending {
        return false;
    }
    match action.kind {
        Kind::Side(side) => {
            state.side = Some(side);
            state.selection = None;
            state.query.clear();
            true
        }
        Kind::SortPrice | Kind::SortName | Kind::SortWeight => {
            state.sort = match action.kind {
                Kind::SortPrice => Sort::Price,
                Kind::SortWeight => Sort::Weight,
                _ => Sort::Name,
            };
            state.rendered = None;
            true
        }
        Kind::Select(quote) => {
            if !panel
                .live
                .filtered_quotes(state.side, &state.query)
                .contains(&quote)
            {
                return false;
            }
            state.selection = Some(quote);
            state.feedback.clear();
            true
        }
        Kind::IgnoreCapacity | Kind::OnlyOwned | Kind::BuyInBackpacks | Kind::IncludeEquipped => {
            let mut options = panel.live.options;
            match action.kind {
                Kind::IgnoreCapacity => options.ignore_capacity = !options.ignore_capacity,
                Kind::OnlyOwned => options.only_owned = !options.only_owned,
                Kind::BuyInBackpacks if state.side == Some(NpcTradeSide::Buy) => {
                    options.buy_in_backpacks = !options.buy_in_backpacks;
                }
                Kind::IncludeEquipped
                    if state.side == Some(NpcTradeSide::Sell) && state.sales.is_empty() =>
                {
                    options.ignore_equipped = !options.ignore_equipped;
                }
                _ => return false,
            }
            play.link.npc_shop_options(options)
        }
        Kind::Cyclopedia => {
            let Some(quote) = state.selection.filter(|q| panel.live.quotes().contains(q)) else {
                return false;
            };
            if !play
                .link
                .inspect(oteryn_client_ui::inspection::Request::Catalogue(quote.item))
            {
                return false;
            }
            play.link.item_catalogue_set_open(true);
            true
        }
        Kind::OpenBackpack => play.link.npc_shop_open_backpack(),
        Kind::SellAll => {
            if !panel.selected || state.side != Some(NpcTradeSide::Sell) || !state.sales.is_empty()
            {
                return false;
            }
            let mut seen = std::collections::BTreeSet::new();
            state.sales = sales::eligible(&panel)
                .filter(|quote| seen.insert(quote.item))
                .collect();
            if let Some(first) = state.sales.pop_front() {
                let sent = play.link.npc_sell_all_request(first);
                state.sale_inventory =
                    sent.then(|| (first.item, panel.live.sale_owned(first.item)));
                if !sent {
                    state.sales.clear();
                }
                sent
            } else {
                false
            }
        }
        Kind::Trade | Kind::Look => {
            let Some(quote) = state
                .selection
                .filter(|quote| panel.live.quotes().contains(quote))
            else {
                return false;
            };
            match action.kind {
                Kind::Trade => {
                    let Some(amount) = oteryn_client_ui::npc_shop::parse_amount(amount) else {
                        state.feedback =
                            "Amount must be a positive integer within this offer's limit.".into();
                        return false;
                    };
                    if !panel.live.selected || !panel.live.can_trade(quote, amount) {
                        state.feedback =
                            "This amount exceeds the offer limit or current backpack capacity."
                                .into();
                        return false;
                    }
                    play.link.npc_trade_request(quote, amount);
                }
                Kind::Look if panel.selected => play.link.npc_shop_look_request(quote),
                _ => return false,
            }
            state.feedback.clear();
            true
        }
        Kind::Close => false,
    }
}
fn activate(
    event: On<Activate>,
    actions: Query<&Action>,
    mut editors: submission::Editors,
    mut owner: NonSendMut<HostOwner>,
    mut state: ResMut<State>,
    guard: ModalGuard,
) {
    if !guard.allows(event.entity) {
        return;
    }
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    if matches!(action.kind, Kind::Trade | Kind::Look) {
        submission::request(&mut owner, &mut state, event.entity, *action, &editors);
        return;
    }
    state.pending = None;
    if perform(&mut owner, &mut state, *action, "1") && matches!(action.kind, Kind::Side(_)) {
        for (_, field, mut text) in &mut editors {
            if matches!(field, Field::Search) {
                text.pending_edits.clear();
                text.pending_paste = None;
                text.editor_mut().set_text("");
            }
        }
    }
}
#[cfg(test)]
std::thread_local! {
    static CATALOGUE_PASSES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn catalogue_rows(
    shop: &LiveShop,
    side: Option<NpcTradeSide>,
    query: &str,
    sort: Sort,
) -> Vec<Quote> {
    #[cfg(test)]
    CATALOGUE_PASSES.with(|passes| passes.set(passes.get() + 1));
    let mut rows = shop.filtered_quotes(side, query);
    if sort == Sort::Weight {
        rows.sort_by_key(|quote| shop.total_weight(*quote, 1).unwrap_or(u128::MAX));
    } else if sort == Sort::Price {
        rows.sort_by_key(|quote| quote.price);
    } else {
        rows.sort_by_cached_key(|quote| {
            shop.quote_name(*quote)
                .map_or_else(|| format!("item #{}", quote.item), str::to_lowercase)
        });
    }
    rows
}

// One retained model, its fields and source widget entities compose this floating panel.
#[allow(clippy::too_many_arguments)]
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    editors: Query<(&Field, &EditableText)>,
    panels: Query<Entity, With<Panel>>,
    mut focus: ResMut<InputFocus>,
    ancestry: Query<&ChildOf>,
    mut text: Query<&mut Text, Without<widgets::OptionCaption>>,
    mut options: Query<(&widgets::OptionCaption, &mut Text)>,
    controls: Query<(Entity, &Action)>,
) {
    let current = owner
        .play
        .as_ref()
        .filter(|play| play.link.npc_shop_is_open() || play.link.npc_dialogue_is_visible());
    let Some(play) = current else {
        for panel in &panels {
            clear_focus(panel, &mut focus, &ancestry);
            commands.entity(panel).despawn();
        }
        *state = State::default();
        return;
    };
    let panel = play.link.npc_shop_panel();
    if !panel.live.selected && !panel.selected && !play.link.npc_dialogue_is_visible() {
        for panel in &panels {
            clear_focus(panel, &mut focus, &ancestry);
            commands.entity(panel).despawn();
        }
        *state = State::default();
        return;
    }
    let generation = owner.generation();
    let fresh = state.generation != Some(generation);
    if fresh {
        for panel in &panels {
            clear_focus(panel, &mut focus, &ancestry);
            commands.entity(panel).despawn();
        }
        *state = State {
            generation: Some(generation),
            side: Some(NpcTradeSide::Buy),
            ..default()
        };
        state.widgets = Some(widgets::build(
            &mut commands,
            &skins,
            generation,
            panel.live.options_selected,
        ));
    }
    sales::advance(&mut state, &owner, &play.link, &panel);
    if panel.live.weight_pending.is_none() {
        let needed = state
            .selection
            .filter(|q| !panel.live.weights.contains_key(&q.item))
            .or_else(|| {
                (state.sort == Sort::Weight)
                    .then(|| {
                        panel.live.quotes().into_iter().find(|q| {
                            Some(q.side) == state.side && !panel.live.weights.contains_key(&q.item)
                        })
                    })
                    .flatten()
            });
        if let Some(quote) = needed {
            play.link.npc_request_weight(quote);
        }
    }
    let quotes = &panel.live.window.offers;
    if panel.live.window.open.is_some()
        && !quotes.is_empty()
        && !quotes.iter().any(|quote| Some(quote.side) == state.side)
    {
        state.side = Some(
            if quotes.iter().any(|quote| quote.side == NpcTradeSide::Buy) {
                NpcTradeSide::Buy
            } else {
                NpcTradeSide::Sell
            },
        );
        state.selection = None;
    }
    let mut amount = "1".to_owned();
    if !fresh {
        for (field, value) in &editors {
            match field {
                Field::Search if !submission::editing(value) => {
                    state.query = value.value().to_string()
                }
                Field::Search => {}
                Field::Amount => amount = value.value().to_string(),
            }
        }
    }
    let Some(widgets) = state.widgets else {
        return;
    };
    let State {
        rendered,
        sort,
        side,
        query,
        selection,
        ..
    } = &mut *state;
    let changed_rows = match rendered.as_mut() {
        Some(shown)
            if (*sort != Sort::Weight || shown.shop.weights == panel.live.weights)
                && shown.shop.window == panel.live.window
                && (*side == Some(NpcTradeSide::Buy)
                    || (shown.shop.options.only_owned == panel.live.options.only_owned
                        && shown.shop.includes_equipment() == panel.live.includes_equipment()))
                && shown.side == *side
                && shown.query == *query
                && shown.selection == *selection =>
        {
            widgets::inventory_changed_rows(shown, &panel.live)
        }
        _ => Some(catalogue_rows(&panel.live, *side, query, *sort)),
    };
    if let Some(rows) = &changed_rows
        && state.selection.is_some_and(|quote| !rows.contains(&quote))
    {
        state.selection = None;
    }
    for (caption, mut text) in &mut options {
        let checked = match caption.0 {
            Kind::IgnoreCapacity => panel.live.options.ignore_capacity,
            Kind::OnlyOwned => panel.live.options.only_owned,
            Kind::BuyInBackpacks => panel.live.options.buy_in_backpacks,
            Kind::IncludeEquipped => panel.live.includes_equipment(),
            _ => false,
        };
        text.0 = if checked { "[x]" } else { "[ ]" }.into();
    }
    let replacing_rows = changed_rows.is_some();
    if let Some(rows) = changed_rows {
        commands.entity(widgets.rows).despawn_children();
        widgets::rows(
            &mut commands,
            &skins,
            widgets.rows,
            generation,
            &rows,
            &panel.live,
            state.selection,
        );
        state.rendered = Some(RenderedOffers {
            sell_items: quotes
                .iter()
                .filter(|offer| offer.side == NpcTradeSide::Sell && offer.count == 1)
                .map(|offer| offer.item_definition_ref)
                .collect(),
            shop: panel.live.clone(),
            sort: state.sort,
            side: state.side,
            query: state.query.clone(),
            selection: state.selection,
            rows,
        });
    }
    for (entity, action) in &controls {
        // The query still sees controls whose despawn was queued above.
        if action.generation != generation
            || replacing_rows && matches!(action.kind, Kind::Select(_))
        {
            continue;
        }
        let supported = match action.kind {
            Kind::Close => true,
            _ if panel.pending || panel.live.pending || state.pending.is_some() => false,
            Kind::Cyclopedia => state.selection.is_some() && play.link.inspection_panel().selected,
            Kind::Look => panel.selected && state.selection.is_some(),
            Kind::BuyInBackpacks => {
                panel.live.options_selected
                    && panel.live.selected
                    && state.side == Some(NpcTradeSide::Buy)
            }
            Kind::IncludeEquipped => {
                panel.live.options_selected
                    && panel.selected
                    && state.side == Some(NpcTradeSide::Sell)
                    && state.sales.is_empty()
            }
            Kind::SellAll => {
                panel.selected
                    && state.side == Some(NpcTradeSide::Sell)
                    && state.rendered.as_ref().is_some_and(|shown| {
                        panel.live.sale_entries().any(|entry| {
                            let id = entry.item_definition_ref.get();
                            !panel.sale_exclusions.contains(&id) && shown.sell_items.contains(&id)
                        })
                    })
            }
            Kind::Trade => {
                panel.live.selected
                    && state.selection.is_some_and(|quote| {
                        oteryn_client_ui::npc_shop::parse_amount(&amount)
                            .is_some_and(|amount| panel.live.can_trade(quote, amount))
                    })
            }
            _ => true,
        };
        if supported {
            commands
                .entity(entity)
                .remove::<bevy::ui::InteractionDisabled>();
        } else {
            commands
                .entity(entity)
                .insert(bevy::ui::InteractionDisabled);
        }
        if let Kind::Side(side) = action.kind {
            if Some(side) == state.side {
                commands.entity(entity).insert(bevy::ui::Checked);
            } else {
                commands.entity(entity).remove::<bevy::ui::Checked>();
            }
        }
    }
    let status = if !play.link.npc_shop_is_open() {
        String::new()
    } else if !state.feedback.is_empty() {
        state.feedback.clone()
    } else if panel.selected
        && (panel.pending || panel.feedback.is_some() || panel.result.is_some())
    {
        oteryn_client_ui::npc_shop::panel_text(&panel)
    } else if !panel.live.feedback.is_empty() {
        panel.live.feedback.clone()
    } else if panel.live.window.open.is_none() {
        "No open shop. Talk to an NPC and ask for trade.".into()
    } else {
        "Select goods, then use Look to request their details.".into()
    };
    if let Ok(mut caption) = text.get_mut(widgets.status) {
        caption.0 = status;
    }
    let price = state
        .selection
        .and_then(|quote| {
            oteryn_client_ui::npc_shop::parse_amount(&amount)
                .map(|amount| panel.live.total(quote, amount))
        })
        .unwrap_or_else(|| {
            if state.selection.is_some() {
                "Enter an amount within the offer limit"
            } else {
                "Select goods"
            }
            .into()
        });
    if let Ok(mut caption) = text.get_mut(widgets.price) {
        caption.0 = price;
        if let Some(weight) = state.selection.and_then(|q| {
            panel
                .live
                .total_weight(q, oteryn_client_ui::npc_shop::parse_amount(&amount)?)
        }) {
            caption.0.push_str(&format!(
                " · Weight {}.{:02} oz",
                weight / 100,
                weight % 100
            ));
        }
        if let Some(free) = panel.live.free_capacity {
            caption
                .0
                .push_str(&format!(" · Free {}.{:02} oz", free / 100, free % 100));
        }
    }
    let balance = panel
        .live
        .balances
        .map(|balance| {
            balance
                .carried_gold
                .saturating_add(balance.bank_gold)
                .to_string()
        })
        .unwrap_or_else(|| "Unknown".into());
    if let Ok(mut caption) = text.get_mut(widgets.balance) {
        caption.0 = balance;
    }
    if let Ok(mut caption) = text.get_mut(widgets.trade_caption) {
        caption.0 = if state.side == Some(NpcTradeSide::Sell) {
            "Sell"
        } else {
            "Buy"
        }
        .into();
    }
}
fn clear_focus(panel: Entity, focus: &mut InputFocus, ancestry: &Query<&ChildOf>) {
    let mut current = focus.get();
    while let Some(entity) = current {
        if entity == panel {
            focus.clear();
            return;
        }
        current = ancestry.get(entity).ok().map(ChildOf::parent);
    }
}
