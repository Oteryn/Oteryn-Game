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
    let Ok((intent, filter)) = decode_stash_query(command.payload) else {
        return Some(Dispatch::Unregistered);
    };
    if intent.action.needs_stash_actions()
        && !admitted.continuity.selected_capabilities.contains(CAPABILITY_STASH_ACTIONS_V1)
    {
        return Some(Dispatch::Unregistered);
    }
    let selected = intent.action.stow_handle()
        .and_then(|handle| item_view.as_ref()?.resolve(handle));
    let row_open = match intent.action {
        StashAction::Withdraw { row, .. } => {
            open_rows.as_ref().is_some_and(|rows| rows.contains(&row))
        }
        StashAction::Browse => false,
        _ => open_rows.is_some(),
    };
    let reply = authority
        .stash_filtered(
            actor,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: command.command_id,
                item_fence: admitted.item_fence,
            },
            intent,
            filter,
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

pub(in super::super) async fn row_action<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
    open_rows: Option<&[[u8; 16]]>,
) -> Option<Dispatch> {
    use crate::foundation::CommandStatus;
    use oteryn_protocol_oteryn::{market::CAPABILITY_MARKET_V1, quick_loot::CAPABILITY_QUICK_LOOT_V1};

    let Ok(intent) = decode_stash_row_intent(command.payload) else {
        return Some(Dispatch::Unregistered);
    };
    let selected = admitted.continuity.selected_capabilities;
    if (intent.action == StashRowAction::Market && !selected.contains(CAPABILITY_MARKET_V1))
        || (intent.action.changes_loot_list() && !selected.contains(CAPABILITY_QUICK_LOOT_V1))
    {
        return Some(Dispatch::Unregistered);
    }
    let row_open = open_rows.is_some_and(|rows| rows.contains(&intent.row));
    match authority.stash_row_action(
        actor,
        UseCommand {
            game_session_id: admitted.game_session_id,
            command_id: command.command_id,
            item_fence: admitted.item_fence,
        },
        intent,
        row_open,
    ).await.ok()? {
        Some(reference) => Some(Dispatch::Reply(CommandStatus::Accepted, encode_stash_row_result(reference))),
        None => Some(Dispatch::Unregistered),
    }
}
