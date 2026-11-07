//! Stash rows are server-issued versions, valid only on the connection's open page.
use super::super::{
    AdmittedSession, ExactActorRef, FreshAdmissionAuthority, SessionItemView, UseCommand,
};
use super::Dispatch;
use oteryn_protocol_oteryn::{ClientCommandView, stash::*};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
    item_view: &Option<SessionItemView>,
    open_rows: &mut Option<Vec<[u8; 16]>>,
) -> Option<Dispatch> {
    let Ok(intent) = decode_stash_intent(command.payload) else {
        return Some(Dispatch::Unregistered);
    };
    let selected = match intent.action {
        StashAction::Stow(handle) => item_view.as_ref().and_then(|view| view.resolve(handle)),
        _ => None,
    };
    let row_open = match intent.action {
        StashAction::Withdraw { row, .. } => {
            open_rows.as_ref().is_some_and(|rows| rows.contains(&row))
        }
        StashAction::Stow(_) => open_rows.is_some(),
        StashAction::Browse => false,
    };
    let reply = authority
        .stash(
            actor,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: command.command_id,
                item_fence: admitted.item_fence,
            },
            intent,
            selected,
            row_open,
        )
        .await?;
    *open_rows = (!matches!(
        reply.outcome,
        StashOutcome::NoLocker | StashOutcome::Rejected
    ))
    .then(|| reply.rows.iter().map(|r| r.handle).collect());
    Some(Dispatch::Stash(reply))
}
