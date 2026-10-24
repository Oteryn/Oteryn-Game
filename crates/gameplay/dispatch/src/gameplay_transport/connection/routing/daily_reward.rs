//! Daily reward command body; predicate and connection control flow stay in `connection.rs`.

use super::super::super as transport;
use super::super::{AdmittedSession, ExactActorRef, FreshAdmissionAuthority, UseCommand};
use super::Dispatch;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Dispatch {
    Dispatch::DailyReward(
        transport::daily_reward::dispatch(
            authority,
            actor,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: command.command_id,
                item_fence: admitted.item_fence,
            },
            command.command_type,
            command.payload,
        )
        .await,
    )
}
