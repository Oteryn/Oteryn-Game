//! Achievements command body; predicate and connection control flow stay in `connection.rs`.

use super::super::{
    AccountAchievementsReply, AccountAchievementsRequest, AdmittedSession, FreshAdmissionAuthority,
    decode_account_achievements_query,
};
use super::Dispatch;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Dispatch {
    match (
        decode_account_achievements_query(command.payload),
        admitted.controller,
    ) {
        (Ok(query), Some(controller)) => Dispatch::Achievements(
            authority
                .account_achievements(AccountAchievementsRequest {
                    account_id: controller.account_id,
                    page: query.page,
                    game_session_id: *admitted.game_session_id.as_bytes(),
                    command_id: command.command_id,
                })
                .await,
        ),
        _ => Dispatch::Achievements(AccountAchievementsReply::Rejected),
    }
}
