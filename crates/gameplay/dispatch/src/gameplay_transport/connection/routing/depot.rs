//! Depot command body; predicate and connection control flow stay in `connection.rs`.

use super::super::{
    AdmittedSession, ExactActorRef, FreshAdmissionAuthority, ItemKey, SessionItemView, UseCommand,
};
use super::Dispatch;
use oteryn_protocol_oteryn::{
    ClientCommandView,
    depot::{DepotAction, DepotOutcome},
};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
    item_view: &mut Option<SessionItemView>,
) -> Option<Dispatch> {
    Some({
        // Capability 510 admits a deposit's box slot (Manual Sort Mode).
        let manual_sort = admitted
            .continuity
            .selected_capabilities
            .contains(oteryn_protocol_oteryn::item_view::CAPABILITY_CONTAINER_MANUAL_SORT_V1);
        if let Ok((intent, slot)) =
            oteryn_protocol_oteryn::depot::decode_depot_intent_at(command.payload, manual_sort)
        {
            let item = match intent.action {
                DepotAction::Deposit(handle) => {
                    item_view.as_ref().and_then(|view| view.resolve(handle))
                }
                DepotAction::WithdrawDisplayed(ordinal) => item_view.as_ref().and_then(|view| {
                    let scope = (intent.town?, intent.box_number, intent.page);
                    if view.depot_selection != Some(scope) {
                        return None;
                    }
                    view.depot_items.iter().find_map(|(shown, item)| {
                        (*shown == ordinal).then_some(ItemKey::Instance(*item))
                    })
                }),
                DepotAction::Withdraw(ordinal) => item_view.as_ref().and_then(|view| {
                    let (town, number) = view.open_depot?;
                    if view.depot_withdraw_needs_browse
                        || intent.town != Some(town)
                        || intent.box_number != number
                    {
                        return None;
                    }
                    view.depot_items.iter().find_map(|(shown, item)| {
                        (*shown == ordinal).then_some(ItemKey::Instance(*item))
                    })
                }),
                _ => None,
            };
            // A new attempt replaces the connection's previous view, including when
            // the owner cannot complete its read. Never retain stale access on `None`.
            let previous_items = item_view
                .as_mut()
                .map(|view| {
                    view.open_depot = None;
                    view.depot_selection = None;
                    std::mem::take(&mut view.depot_items)
                })
                .unwrap_or_default();
            let mut observation = authority
                .depot_at(
                    actor,
                    UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id: command.command_id,
                        item_fence: admitted.item_fence,
                    },
                    intent,
                    slot,
                    item,
                )
                .await?;
            let mut reply = observation.reply;
            let opened = matches!(reply.outcome, DepotOutcome::Opened | DepotOutcome::Moved);
            // A historical MOVED receipt can arrive without a current town or page.
            // An encodable current page can issue selection ordinals. Text writing still
            // records only successful opens and moves, and independently rechecks reach.
            if let Some(view) = item_view.as_mut()
                && reply.box_number == intent.box_number
                && reply.page == intent.page
                && (opened || manual_sort)
                && oteryn_protocol_oteryn::depot::encode_depot_reply(&reply).is_ok()
                && observation.items.len() == reply.entries.len()
                && observation
                    .items
                    .iter()
                    .zip(&reply.entries)
                    .all(|((ordinal, _), entry)| *ordinal == entry.ordinal)
                && !observation
                    .items
                    .iter()
                    .enumerate()
                    .any(|(index, (_, item))| {
                        observation.items[..index]
                            .iter()
                            .any(|(_, previous)| previous == item)
                    })
                && let Some(town) = reply.town
            {
                if manual_sort {
                    // A refreshed page always gets new ordinals, even when a
                    // durable receipt is replayed or the box order did not change.
                    let next = view.reserve_depot_ordinals(reply.entries.len())?;
                    for (offset, (entry, (ordinal, _))) in reply
                        .entries
                        .iter_mut()
                        .zip(&mut observation.items)
                        .enumerate()
                    {
                        *ordinal = next - offset as u64;
                        entry.ordinal = *ordinal;
                    }
                    view.depot_selection = Some((town, reply.box_number, reply.page));
                } else if intent.action == DepotAction::Browse {
                    view.depot_withdraw_needs_browse = false;
                } else {
                    // Legacy pages expose storage ordinals, which a move can reassign.
                    view.depot_withdraw_needs_browse |=
                        previous_items.iter().any(|(old_ordinal, old_id)| {
                            observation
                                .items
                                .iter()
                                .any(|(ordinal, id)| ordinal == old_ordinal && id != old_id)
                        });
                }
                if opened {
                    view.open_depot = Some((town, reply.box_number));
                }
                view.depot_items = observation.items;
            } else if manual_sort && !reply.entries.is_empty() {
                return None;
            }
            Dispatch::Depot(reply)
        } else {
            Dispatch::Unregistered
        }
    })
}
