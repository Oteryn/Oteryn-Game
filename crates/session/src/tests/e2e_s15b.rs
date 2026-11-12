//! S15b replay: the in-fight logout refusal. `Session::logout` (`session_commands.rs`) decodes the
//! server's LOGOUT result into `LogoutOutcome`; a refusal (`InFight`) keeps the session playing and
//! only an `Accepted` result ends it. The raw result is not retained, so the capture's
//! `s15b-expected.txt` (server-observed status, result and retry wait) is encoded with the
//! production logout encoder by `logout_session` before it reaches the real session path.
use super::*;
use oteryn_protocol_oteryn::logout::LogoutResult;

fn status(text: &str) -> Result<CommandStatus, BoxError> {
    match text.strip_prefix("status=") {
        Some("Accepted") => Ok(CommandStatus::Accepted),
        Some("Rejected") => Ok(CommandStatus::Rejected),
        other => Err(format!("s15b unknown status {other:?}").into()),
    }
}

/// The refusal (status, retry wait) and the later accepted status.
fn observed() -> Result<(CommandStatus, u32, CommandStatus), BoxError> {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok((CommandStatus::Rejected, 1_500, CommandStatus::Accepted));
    };
    let text = std::fs::read_to_string(std::path::Path::new(&root).join("s15b-expected.txt"))?;
    let mut refusal = None;
    let mut accepted = None;
    for line in text.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        match fields.as_slice() {
            ["logout-refusal", state, "result=InFight", retry] => {
                refusal = Some((
                    status(state)?,
                    retry
                        .strip_prefix("retry_after_ms=")
                        .ok_or("s15b retry field")?
                        .parse()?,
                ));
            }
            ["logout-accepted", state, "result=Accepted"] => accepted = Some(status(state)?),
            _ => return Err(format!("s15b expected line malformed: {line:?}").into()),
        }
    }
    let (refused, retry) = refusal.ok_or("s15b expected lacks the refusal")?;
    Ok((
        refused,
        retry,
        accepted.ok_or("s15b expected lacks the accept")?,
    ))
}

#[test]
fn e2e_s15b_in_fight_logout_refusal_keeps_the_session_playing() -> Result<(), BoxError> {
    let (refused_status, retry_after_ms, accepted_status) = observed()?;
    assert_eq!(
        (refused_status, accepted_status),
        (CommandStatus::Rejected, CommandStatus::Accepted),
        "S15b requires an in-fight refusal followed by an accepted retry"
    );
    block_on(async {
        let (mut session, server) = logout_session(vec![
            (
                refused_status,
                Some(LogoutResult::InFight { retry_after_ms }),
            ),
            (accepted_status, Some(LogoutResult::Accepted)),
        ])
        .await?;
        let refused = session.logout().await?;
        assert_eq!(
            (refused.command_id, refused.status, refused.result),
            (
                7,
                CommandStatus::Rejected,
                Some(LogoutResult::InFight { retry_after_ms })
            )
        );
        // The refusal keeps the session usable: the next command id is issued, not poisoned.
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
        Ok(())
    })?
}
