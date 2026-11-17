//! S09 client replay: the server-observed logout outcome is answered to `Session::logout`
//! (`session_commands.rs`), which decodes it into `LogoutOutcome` and ends the session after an
//! `Accepted` result. The outcome is the capture's `s09-expected.txt` value encoded by the
//! production logout encoder (the client does not retain the raw `CommandResult` bytes).
use super::*;
use oteryn_protocol_oteryn::logout::LogoutResult;

fn captured() -> Result<(CommandStatus, LogoutResult), BoxError> {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok((CommandStatus::Accepted, LogoutResult::Accepted));
    };
    let text = std::fs::read_to_string(std::path::Path::new(&root).join("s09-expected.txt"))?;
    let line = text
        .lines()
        .find(|line| line.starts_with("logout\t"))
        .ok_or("s09 expected lacks a logout line")?;
    let fields: Vec<&str> = line.split('\t').collect();
    let [_, status, result] = fields.as_slice() else {
        return Err(format!("s09 logout line malformed: {line}").into());
    };
    let status = match status.strip_prefix("status=") {
        Some("Accepted") => CommandStatus::Accepted,
        Some("Rejected") => CommandStatus::Rejected,
        other => return Err(format!("s09 unknown status {other:?}").into()),
    };
    let result = match result.strip_prefix("result=") {
        Some("Some(Accepted)") => LogoutResult::Accepted,
        Some("Some(Busy)") => LogoutResult::Busy,
        other => return Err(format!("s09 unsupported result {other:?}").into()),
    };
    Ok((status, result))
}

#[test]
fn e2e_s09_accepted_logout_ends_the_session_and_a_refusal_does_not() -> Result<(), BoxError> {
    let (status, result) = captured()?;
    assert_eq!(
        (status, result),
        (CommandStatus::Accepted, LogoutResult::Accepted),
        "S09 requires the server to accept the PZ logout"
    );
    block_on(async {
        // The first logout is refused before the logout authority (empty rejected payload).
        let (mut session, server) = logout_session(vec![
            (CommandStatus::Rejected, None),
            (status, Some(result)),
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
