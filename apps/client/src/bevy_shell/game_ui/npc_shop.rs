//! Source-shaped NPC trade controls over retained, server-issued quotes.
//! NpcDialog.qml is a floating dialog: opening this panel does not stop world movement.
use super::{
    super::{owner::HostOwner, ui::Skins},
    modal::ModalGuard,
};
use bevy::{input_focus::InputFocus, prelude::*, text::EditableText, ui_widgets::Activate};
use oteryn_client_ui::npc_shop::{NpcTradeSide, Quote};
use oteryn_client_ui_bevy as source;
mod conversation;
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
    SellAll,
    Close,
    IgnoreCapacity,
    IgnoreEquipped,
    OnlyOwned,
    OpenBackpack,
}
#[derive(Resource, Default)]
struct State {
    generation: Option<u64>,
    selection: Option<Quote>,
    side: Option<NpcTradeSide>,
    query: String,
    feedback: String,
    widgets: Option<widgets::Widgets>,
    rendered: Option<(Vec<Quote>, Option<Quote>)>,
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
        Kind::IgnoreCapacity | Kind::IgnoreEquipped | Kind::OnlyOwned => {
            let mut options = panel.live.options;
            match action.kind {
                Kind::IgnoreCapacity => options.ignore_capacity = !options.ignore_capacity,
                Kind::IgnoreEquipped => options.ignore_equipped = !options.ignore_equipped,
                Kind::OnlyOwned => options.only_owned = !options.only_owned,
                _ => {}
            }
            play.link.npc_shop_options(options)
        }
        Kind::OpenBackpack => play.link.npc_shop_open_backpack(),
        Kind::Trade | Kind::Look | Kind::SellAll => {
            let Some(quote) = state
                .selection
                .filter(|quote| panel.live.quotes().contains(quote))
            else {
                return false;
            };
            match action.kind {
                Kind::Trade => {
                    let Some(amount) = oteryn_client_ui::npc_shop::parse_amount(amount) else {
                        state.feedback = "Amount must be an integer from 1 to 100.".into();
                        return false;
                    };
                    if !panel.live.selected || !panel.live.can_trade(quote, amount) {
                        state.feedback =
                            "This amount cannot be traded with the current shop and backpack state."
                                .into();
                        return false;
                    }
                    play.link.npc_trade_request(quote, amount);
                }
                Kind::Look if panel.selected => play.link.npc_shop_look_request(quote),
                Kind::SellAll
                    if panel.selected && quote.side == NpcTradeSide::Sell && quote.count == 1 =>
                {
                    play.link.npc_sell_all_request(quote)
                }
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
    if matches!(action.kind, Kind::Trade | Kind::Look | Kind::SellAll) {
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
        state.widgets = Some(widgets::build(&mut commands, &skins, generation));
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
    let rows = panel.live.filtered_quotes(state.side, &state.query);
    if state.selection.is_some_and(|quote| !rows.contains(&quote)) {
        state.selection = None;
    }
    for (caption, mut text) in &mut options {
        let checked = match caption.0 {
            Kind::IgnoreCapacity => panel.live.options.ignore_capacity,
            Kind::IgnoreEquipped => panel.live.options.ignore_equipped,
            Kind::OnlyOwned => panel.live.options.only_owned,
            _ => false,
        };
        text.0 = if checked { "[x]" } else { "[ ]" }.into();
    }
    for (entity, action) in &controls {
        let supported = match action.kind {
            Kind::Close => true,
            _ if panel.pending || panel.live.pending || state.pending.is_some() => false,
            Kind::Look => panel.selected && state.selection.is_some(),
            Kind::SellAll => {
                panel.selected
                    && state
                        .selection
                        .is_some_and(|quote| quote.side == NpcTradeSide::Sell && quote.count == 1)
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
    let Some(widgets) = state.widgets else {
        return;
    };
    let snapshot = (rows.clone(), state.selection);
    if state.rendered.as_ref() != Some(&snapshot) {
        commands.entity(widgets.rows).despawn_children();
        widgets::rows(
            &mut commands,
            &skins,
            widgets.rows,
            generation,
            &rows,
            state.selection,
        );
        state.rendered = Some(snapshot);
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
                "Enter 1–100"
            } else {
                "Select goods"
            }
            .into()
        });
    if let Ok(mut caption) = text.get_mut(widgets.price) {
        caption.0 = price;
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
