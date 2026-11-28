//! Unit coverage for the client logout state transition. Live S09 replay runs inside its indexed row.
use super::*;
use oteryn_protocol_oteryn::logout::LogoutResult;

#[test]
fn accepted_logout_ends_the_session_and_an_empty_refusal_does_not() -> Result<(), BoxError> {
    block_on(async {
        // The first logout is refused before the logout authority (empty rejected payload).
        let (mut session, server) = logout_session(vec![
            (CommandStatus::Rejected, None),
            (CommandStatus::Accepted, Some(LogoutResult::Accepted)),
        ])
        .await?;
        let refused = session.logout().await?;
        assert_eq!(
            (refused.command_id, refused.status, refused.result),
            (7, CommandStatus::Rejected, None)
        );
        assert_eq!(session.next_command_id(), 8);
        let accepted = session.logout().await?;
        server.await??;
        assert_eq!(
            (accepted.command_id, accepted.status, accepted.result),
            (8, CommandStatus::Accepted, Some(LogoutResult::Accepted))
        );
        assert!(matches!(
            session.logout().await,
            Err(SessionError::SessionUnusable)
        ));
        assert_eq!(session.next_command_id(), 9);
        Ok(())
    })?
}
