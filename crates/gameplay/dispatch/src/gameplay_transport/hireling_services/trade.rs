//! A category/page is a whole bounded native window; its revision binds every shown row.
use super::*;
use crate::durability::npc_trade::{HirelingTradeSelection, NpcTradeError};

pub(super) struct Window {
    category: u8,
    page: u32,
    revision: u64,
    rows: Vec<catalogue::Offer>,
    shown: Vec<NpcOffer>,
}

fn render(
    category: u8,
    page: u32,
    catalogue_revision: u64,
    items: &ItemDefinitionIndex,
) -> Option<Window> {
    let rows = catalogue::page(category, page, MAX_OFFERS)?;
    let mut hash = Sha256::new();
    hash.update(b"oteryn:hireling-window:v1\0");
    hash.update(catalogue::SOURCE_SHA256.as_bytes());
    hash.update(catalogue_revision.to_be_bytes());
    hash.update([category]);
    hash.update(page.to_be_bytes());
    let shown = rows
        .iter()
        .map(|offer| {
            let source = &catalogue::ROWS[offer.row];
            let key = format!("oteryn:item.tibia.i{}", source.item_id);
            let revision = items.revision(&key)?;
            let definition = items.definition_ref("Item", &key, revision)?.get();
            hash.update((offer.row as u64).to_be_bytes());
            hash.update(revision.as_bytes());
            hash.update(definition.to_be_bytes());
            Some(NpcOffer {
                item_definition_ref: definition,
                side: match offer.side {
                    catalogue::Side::Buy => NpcTradeSide::Buy,
                    catalogue::Side::Sell => NpcTradeSide::Sell,
                },
                count: 1,
                unit_price: u32::try_from(offer.price).ok()?,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let digest = hash.finalize();
    let revision = u64::from_be_bytes(digest[..8].try_into().ok()?).max(1);
    Some(Window {
        category,
        page,
        revision,
        rows,
        shown,
    })
}

pub(super) async fn open(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    context: Context,
    category: u8,
    page: u32,
) -> NpcTalkOutcome {
    let runtime = admission.runtime.lock().await;
    if admission.states.npc_conversations.current_npc(
        &admission.states.chat,
        &runtime,
        actor,
        command.game_session_id,
        intent,
    ) != Some(context.clone())
    {
        return rejected();
    }
    let Some(items) = super::super::super::item_ref_admission::pinned_index(
        &runtime,
        admission.active_generation,
    ) else {
        return rejected();
    };
    let Some(window) = render(category, page, context.0, items) else {
        return NpcConversations::bank_reply(
            intent,
            "That page is unavailable in the current catalogue.".into(),
        );
    };
    let mut outcome = NpcConversations::bank_reply(
        intent,
        format!(
            "{} — page {}. Say next, previous, or a category and page number.",
            catalogue::CATEGORIES[usize::from(category)],
            page + 1
        ),
    );
    outcome.window = Some(NpcTradeWindow {
        open: Some((intent.npc_actor, window.revision)),
        offers: window.shown.clone(),
    });
    let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock() else {
        return rejected();
    };
    let Some(state) = sessions.get_mut(&command.game_session_id) else {
        return rejected();
    };
    let Some(hireling) = state.hireling.as_mut() else {
        return rejected();
    };
    if hireling.context != context || !catalogue::allowed(category, hireling.snapshot.skills) {
        return rejected();
    }
    hireling.trade = Some(window);
    state.window = Some(intent.npc_actor);
    outcome
}

pub(in crate::gameplay_transport) fn resolve_trade(
    owner: &NpcConversations,
    session: GameSessionId,
    key: &str,
    intent: &NpcTradeIntent,
    items: &ItemDefinitionIndex,
) -> Result<NpcTradeOffer, Box<NpcTradeRefusal>> {
    let rejected = || Box::new(NpcTradeRefusal::only(NpcIntentDisposition::Rejected));
    let Ok(sessions) = owner.sessions.lock() else {
        return Err(rejected());
    };
    let Some(hireling) = sessions
        .get(&session)
        .and_then(|state| state.hireling.as_ref())
    else {
        return Err(rejected());
    };
    let Some(window) = hireling.trade.as_ref() else {
        return Err(rejected());
    };
    if hireling.context.2 != key
        || hireling.npc != intent.npc_actor
        || !catalogue::allowed(window.category, hireling.snapshot.skills)
        || !(1..=MAX_TRADE_QUANTITY).contains(&intent.quantity)
    {
        return Err(rejected());
    }
    let fresh =
        render(window.category, window.page, hireling.context.0, items).ok_or_else(rejected)?;
    let stale = || {
        Box::new(NpcTradeRefusal {
            disposition: NpcIntentDisposition::Stale,
            update: NpcUpdate {
                conversation: None,
                window: Some(NpcTradeWindow {
                    open: Some((intent.npc_actor, fresh.revision)),
                    offers: fresh.shown.clone(),
                }),
            },
        })
    };
    if intent.catalogue_revision != window.revision || fresh.revision != window.revision {
        return Err(stale());
    }
    let position = usize::try_from(intent.offer_index).map_err(|_| stale())?;
    let shown = window.shown.get(position).ok_or_else(stale)?;
    if shown.side != intent.side || shown.unit_price != intent.expected_unit_price {
        return Err(stale());
    }
    let row = &catalogue::ROWS[window.rows[position].row];
    let item_key = format!("oteryn:item.tibia.i{}", row.item_id);
    let item_revision = items.revision(&item_key).ok_or_else(stale)?.to_owned();
    Ok(NpcTradeOffer {
        npc_key: key.into(),
        catalogue_revision: window.revision,
        offer_index: intent.offer_index,
        side: shown.side,
        quantity: intent.quantity,
        count: 1,
        unit_price: shown.unit_price,
        item_key,
        item_revision,
    })
}

/// Recheck current entitlements before opening the durable native NPC-trade transaction.
/// That transaction repeats this House/identity proof under its authoritative locks.
pub(in crate::gameplay_transport) async fn authorize_trade(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTradeIntent,
) -> Result<Option<HirelingTradeSelection>, NpcTradeError> {
    let context = {
        let runtime = admission.runtime.lock().await;
        let npc = super::super::service_context::live_npc(
            &runtime,
            actor,
            command.game_session_id,
            intent.npc_actor,
        )
        .ok_or(NpcTradeError::AuthorityRejected)?;
        let key = npc.key.as_str();
        let Some(id) = npc_identity(key) else {
            return Ok(None);
        };
        let sessions = admission
            .states
            .npc_conversations
            .sessions
            .lock()
            .map_err(|_| NpcTradeError::AuthorityRejected)?;
        let state = sessions
            .get(&command.game_session_id)
            .and_then(|state| state.hireling.as_ref())
            .ok_or(NpcTradeError::AuthorityRejected)?;
        let window = state
            .trade
            .as_ref()
            .ok_or(NpcTradeError::AuthorityRejected)?;
        let offer = window
            .rows
            .get(intent.offer_index as usize)
            .ok_or(NpcTradeError::AuthorityRejected)?;
        if state.npc != intent.npc_actor
            || state.snapshot.id != id
            || window.revision != intent.catalogue_revision
        {
            return Err(NpcTradeError::AuthorityRejected);
        }
        (id, window.category, offer.row, state.context.clone())
    };
    let fence = command.item_fence.ok_or(NpcTradeError::AuthorityRejected)?;
    let snapshot = admission
        .root
        .load_house_hireling_service(
            admission.character,
            admission.holder,
            fence,
            *admission.channel_id.as_bytes(),
            context.0,
        )
        .await
        .map_err(NpcTradeError::Unavailable)?
        .ok_or(NpcTradeError::AuthorityRejected)?;
    if !catalogue::allowed(context.1, snapshot.skills) {
        return Err(NpcTradeError::RequirementNotMet);
    }
    let runtime = admission.runtime.lock().await;
    if admission.states.npc_conversations.current_npc(
        &admission.states.chat,
        &runtime,
        actor,
        command.game_session_id,
        &NpcTalkIntent {
            npc_actor: intent.npc_actor,
            text: String::new(),
        },
    ) != Some(context.3)
    {
        return Err(NpcTradeError::AuthorityRejected);
    }
    Ok(Some(HirelingTradeSelection {
        hireling_id: context.0,
        source_row: context.2,
        origin_channel: admission.channel_id,
        full_charges: None,
        wrapped_target: None,
    }))
}
