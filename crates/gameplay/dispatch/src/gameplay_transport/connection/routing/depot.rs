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
    item_view: &Option<SessionItemView>,
) -> Option<Dispatch> {
    Some({
        if let Ok(intent) = oteryn_protocol_oteryn::depot::decode_depot_intent(command.payload) {
            let item = match intent.action {
                oteryn_protocol_oteryn::depot::DepotAction::Deposit(handle) => {
                    item_view.as_ref().and_then(|view| view.resolve(handle))
                }
                _ => None,
            };
            let reply = authority
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
            Dispatch::Depot(reply)
        } else {
            Dispatch::Unregistered
        }
    })
}
