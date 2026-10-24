//! Highscores command body; predicate and connection control flow stay in `connection.rs`.

use super::super::{
    AdmittedSession, FreshAdmissionAuthority, HighscoresReply, decode_highscores_query,
};
use super::Dispatch;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Dispatch {
    // HIGHSCORES0 §6: the World and the asking Character are the admitted session's.
    match decode_highscores_query(command.payload) {
        Ok(query) => Dispatch::Highscores(
            authority
                .highscores(admitted.world_id, admitted.game_session_id, query)
                .await,
        ),
        Err(_) => Dispatch::Highscores(HighscoresReply::Rejected),
    }
}
