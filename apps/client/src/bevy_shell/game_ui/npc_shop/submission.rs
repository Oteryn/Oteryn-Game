//! One transient NPC action waits for upstream edits without changing the selected quote.
use super::*;
use oteryn_client_ui::npc_shop::ShopOptions;
pub(super) type Editors<'w, 's> =
    Query<'w, 's, (Entity, &'static Field, &'static mut EditableText)>;
#[derive(Clone, Copy)]
pub(super) struct Pending {
    anchor: Entity,
    action: Action,
    quote: Quote,
    side: Option<NpcTradeSide>,
    options: ShopOptions,
    amount: Entity,
    search: Entity,
}
pub(super) fn editing(editor: &EditableText) -> bool {
    editor.pending_paste.is_some()
        || editor
            .pending_edits
            .iter()
            .any(bevy::text::TextEdit::is_destructive)
}
fn waiting(pending: &Pending, fields: &Editors) -> Option<bool> {
    let (_, amount, amount_editor) = fields.get(pending.amount).ok()?;
    let (_, search, search_editor) = fields.get(pending.search).ok()?;
    (*amount == Field::Amount && *search == Field::Search)
        .then(|| editing(amount_editor) || editing(search_editor))
}
fn current(owner: &HostOwner, state: &State, pending: &Pending) -> bool {
    if owner.generation() != pending.action.generation
        || state.generation != Some(pending.action.generation)
        || owner.modal_active()
        || state.side != pending.side
        || state.selection != Some(pending.quote)
    {
        return false;
    }
    let Some(play) = &owner.play else {
        return false;
    };
    if !play.link.npc_shop_is_open() {
        return false;
    }
    let panel = play.link.npc_shop_panel();
    let supported = match pending.action.kind {
        Kind::Trade => panel.live.selected,
        Kind::Look | Kind::SellAll => panel.selected,
        _ => false,
    };
    supported
        && !panel.pending
        && !panel.live.pending
        && panel.live.options == pending.options
        && panel.live.quotes().contains(&pending.quote)
}
pub(super) fn request(
    owner: &mut HostOwner,
    state: &mut State,
    anchor: Entity,
    action: Action,
    fields: &Editors,
) {
    if state.pending.is_some() {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    let Some(quote) = state.selection else {
        return;
    };
    let Some(amount) = fields
        .iter()
        .find_map(|(entity, field, _)| (*field == Field::Amount).then_some(entity))
    else {
        return;
    };
    let Some(search) = fields
        .iter()
        .find_map(|(entity, field, _)| (*field == Field::Search).then_some(entity))
    else {
        return;
    };
    let pending = Pending {
        anchor,
        action,
        quote,
        side: state.side,
        options: play.link.npc_shop_panel().live.options,
        amount,
        search,
    };
    if !current(owner, state, &pending) {
        return;
    }
    match waiting(&pending, fields) {
        Some(true) => state.pending = Some(pending),
        Some(false) => apply(owner, state, &pending, fields),
        None => {}
    }
}
fn apply(owner: &mut HostOwner, state: &mut State, pending: &Pending, fields: &Editors) {
    let (Ok((_, _, amount)), Ok((_, _, search))) =
        (fields.get(pending.amount), fields.get(pending.search))
    else {
        return;
    };
    // Only committed input reaches command validation; pending values were checked first.
    let query = search.value().to_string();
    let Some(play) = &owner.play else {
        return;
    };
    if !play
        .link
        .npc_shop_panel()
        .live
        .filtered_quotes(pending.side, &query)
        .contains(&pending.quote)
    {
        return;
    }
    state.query = query;
    perform(owner, state, pending.action, &amount.value().to_string());
}
pub(super) fn finish(
    mut owner: NonSendMut<HostOwner>,
    mut state: ResMut<State>,
    fields: Editors,
    guard: ModalGuard,
) {
    let Some(pending) = state.pending.take() else {
        return;
    };
    if !current(&owner, &state, &pending) || !guard.allows(pending.anchor) {
        return;
    }
    match waiting(&pending, &fields) {
        Some(true) => state.pending = Some(pending),
        Some(false) => apply(&mut owner, &mut state, &pending, &fields),
        None => {}
    }
}
