//! IDLEKICK uses the ordinary logout owner and its existing terminal reconciliation.
use super::*;
use oteryn_protocol_oteryn::tutorial_notices::TutorialNotice;
use std::time::Duration;

// canary:src/creatures/players/player.cpp:8644; Global timing: tibia-manual/interface.md:235.
const WARNING: &str = "There was no variation in your behaviour for 15 minutes. You will be disconnected in one minute if there is no change in your actions until then.";

pub(super) async fn tick<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
) -> Result<(), ServeEnd>
where
    S: AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    let Some((training, no_logout)) = authority.gameplay_idle_exemptions(session.admitted).await
    else {
        return Err(ServeEnd::Disconnected);
    };
    let now = authority.gameplay_idle_now();
    let policy = authority.gameplay_idle_policy();
    let idle = &mut session.admitted.continuity.gameplay_idle;
    idle.training(now, training);
    // NOLOGOUT suppresses the warning/removal check, as in both pinned donors, but
    // does not refresh the clock. Leaving the tile re-evaluates the retained deadline.
    if training || no_logout {
        return Ok(());
    }
    if idle.warning_due(now, policy) {
        // Reuse the negotiated status-text channel; no new unsolicited wire message.
        // Operator overrides must not advertise the default fifteen/one-minute timing.
        let warning = if policy == GameplayIdlePolicy::default() {
            WARNING
        } else {
            "You have been inactive. You will be disconnected soon unless you perform a gameplay action."
        };
        tutorial_notices_transport::emit_notice(
            stream,
            &mut session.admitted,
            &mut session.sequence,
            &TutorialNotice::StatusText(warning.into()),
        )
        .await
        .ok_or(ServeEnd::Disconnected)?;
    }
    if session.admitted.continuity.gameplay_idle.elapsed(now)
        >= Duration::from_millis(policy.removal_after_ms)
    {
        let Some(_input) = authority.enter_input().await else {
            return Ok(());
        };
        match authority.logout(session.admitted).await {
            LogoutReply::Result(LogoutResult::Accepted) => return Err(ServeEnd::LoggedOut),
            // Keep the expired timer armed. A NOLOGOUT tile, fight lock or busy release is
            // retried at the next cadence, without repeating the warning.
            LogoutReply::NoLogout
            | LogoutReply::Result(LogoutResult::InFight { .. } | LogoutResult::Busy) => {}
            LogoutReply::Unknown => return Err(ServeEnd::Disconnected),
        }
    }
    Ok(())
}
