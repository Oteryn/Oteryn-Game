use super::super::{AdmittedSession, ExactActorRef, FreshAdmissionAuthority, UseCommand};
use super::Dispatch;
use oteryn_protocol_oteryn::{ClientCommandView, CommandStatus};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Dispatch {
    let accepted = crate::gameplay_transport::appearance_change::dispatch_mount(
        authority,
        actor,
        UseCommand {
            game_session_id: admitted.game_session_id,
            command_id: command.command_id,
            item_fence: admitted.item_fence,
        },
        command.payload,
    )
    .await
        == Some(true);
    Dispatch::Reply(
        if accepted {
            CommandStatus::Accepted
        } else {
            CommandStatus::Rejected
        },
        Vec::new(),
    )
}
