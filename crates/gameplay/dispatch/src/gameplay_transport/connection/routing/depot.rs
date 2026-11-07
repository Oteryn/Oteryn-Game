//! Depot command body; predicate and connection control flow stay in `connection.rs`.

use super::super::{
    AdmittedSession, ExactActorRef, FreshAdmissionAuthority, SessionItemView, UseCommand,
};
use super::Dispatch;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
    item_view: &mut Option<SessionItemView>,
) -> Option<Dispatch> {
    Some({
        if let Ok(intent) = oteryn_protocol_oteryn::depot::decode_depot_intent(command.payload) {
            let item = match intent.action {
                oteryn_protocol_oteryn::depot::DepotAction::Deposit(handle) => {
                    item_view.as_ref().and_then(|view| view.resolve(handle))
                }
                _ => None,
            };
            // A new attempt replaces the connection's previous view, including when
            // the owner cannot complete its read. Never retain stale access on `None`.
            if let Some(view) = item_view.as_mut() {
                view.open_depot = None;
                view.depot_items.clear();
            }
            let observation = authority
                .depot(
                    actor,
                    UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id: command.command_id,
                        item_fence: admitted.item_fence,
                    },
                    intent,
                    item,
                )
                .await?;
            let reply = observation.reply;
            // A historical MOVED receipt can arrive without a current town/page.
            // Only a freshly opened, encodable view records owner-observed box data.
            // This does not grant reach: the writer must still recheck under the lane.
            if let Some(view) = item_view.as_mut()
                && reply.box_number == intent.box_number
                && reply.page == intent.page
                && matches!(
                    reply.outcome,
                    oteryn_protocol_oteryn::depot::DepotOutcome::Opened
                        | oteryn_protocol_oteryn::depot::DepotOutcome::Moved
                )
                && oteryn_protocol_oteryn::depot::encode_depot_reply(&reply).is_ok()
                && observation.items.len() == reply.entries.len()
                && observation.items.iter().zip(&reply.entries)
                    .all(|((ordinal, _), entry)| *ordinal == entry.ordinal)
                && !observation.items.iter().enumerate().any(|(index, (_, item))|
                    observation.items[..index].iter().any(|(_, previous)| previous == item))
            {
                view.open_depot = reply.town.map(|town| (town, reply.box_number));
                if view.open_depot.is_some() {
                    view.depot_items = observation.items;
                }
            }
            Dispatch::Depot(reply)
        } else {
            Dispatch::Unregistered
        }
    })
}
