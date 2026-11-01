//! All achievement producers share one bounded account observation and delivery path.
use super::*;
use oteryn_protocol_oteryn::achievement_notices::AchievementEarned;
use std::collections::BTreeSet;

/// Per-connection display state, bounded by the active catalogue. This is not grant truth:
/// the existing account owner and source receipts remain the only durable owners.
pub(super) struct SeenAchievements {
    keys: BTreeSet<String>,
}

impl SeenAchievements {
    pub(super) fn seed(facts: Vec<AchievementEarned>) -> Self {
        Self {
            keys: facts.into_iter().map(|fact| fact.key).collect(),
        }
    }
}

/// Read-only refresh after source commands and on the existing liveness cadence. A bounded
/// set difference observes concurrent grants, including two with the same `earned_at` value.
pub(super) async fn refresh<S: AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
) -> Result<(), ServeEnd> {
    if session.achievement_notices.is_none() {
        return Ok(());
    }
    let observed = authority
        .observe_achievement_notice_facts(session.admitted)
        .await
        .ok_or(ServeEnd::Disconnected)?;
    let facts = observed.facts.ok_or(ServeEnd::Disconnected)?;
    for fact in facts {
        deliver(stream, session, &fact).await?;
    }
    Ok(())
}

/// Keep the immediate chest grant path, but share its new-key gate with account observation.
/// A replay/AlreadyHeld has no earned value and cannot invent either form of notice.
pub(super) async fn immediate<S: AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
    notice: &EarnedNotice,
) -> Result<(), ServeEnd> {
    if notice == &EarnedNotice::Unknown
        && session.notices_selected
        && session
            .admitted
            .continuity
            .achievement_notice_revision
            .is_some()
    {
        return Err(ServeEnd::Disconnected);
    }
    if let EarnedNotice::Earned(earned) = notice {
        if session.achievement_notices.is_some() {
            // The source committed earlier; its transport may have been replaced while
            // the ordinary result/inventory updates were being written. Apply the same
            // current account/session gate before the immediate notice, and take its
            // name/watermark from the catalogue-backed observation as well.
            let observed = authority
                .observe_achievement_notice_facts(session.admitted)
                .await
                .ok_or(ServeEnd::Disconnected)?;
            let current = observed
                .facts
                .ok_or(ServeEnd::Disconnected)?
                .into_iter()
                .find(|fact| fact.key == earned.key)
                .ok_or(ServeEnd::Disconnected)?;
            deliver(stream, session, &current).await?;
        } else {
            // Historical injected authorities expose only a watermark; retain their
            // immediate contract. The production authority always supplies full facts.
            deliver(stream, session, earned).await?;
        }
    }
    Ok(())
}

async fn deliver<S: AsyncWrite + Unpin>(
    stream: &mut S,
    session: &mut ServeState,
    earned: &AchievementEarned,
) -> Result<(), ServeEnd> {
    if session
        .achievement_notices
        .as_ref()
        .is_some_and(|seen| seen.keys.contains(&earned.key))
    {
        return Ok(());
    }
    if let Some(from) = session
        .admitted
        .continuity
        .achievement_notice_revision
        .filter(|_| session.notices_selected)
    {
        let (Some(delta_sequence), Some(to)) =
            (session.sequence.checked_add(1), from.checked_add(1))
        else {
            return Err(ServeEnd::Disconnected);
        };
        let payload = encode_achievement_earned(earned).map_err(|_| ServeEnd::Disconnected)?;
        let delta = encode_state_delta(
            session.generation,
            delta_sequence,
            STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES,
            from,
            to,
            DELTA_TYPE_ACHIEVEMENT_EARNED_V1,
            &payload,
        )
        .map_err(|_| ServeEnd::Disconnected)?;
        // Preserve ACH-NOTIFY-1: a revision possibly sent to the client is never reused.
        session.sequence = delta_sequence;
        session.admitted.continuity.server_sequence = delta_sequence;
        session.admitted.continuity.achievement_notice_revision = Some(to);
        write_frame(stream, &delta)
            .await
            .map_err(|_| ServeEnd::Disconnected)?;
    }
    // Crystal player_achievement.cpp:27-42 uses the ordinary advance/status-text channel.
    // A client selecting only tutorial notices still gets that existing representation.
    let text =
        crate::gameplay_transport::achievement_notification(&EarnedNotice::Earned(earned.clone()))
            .ok_or(ServeEnd::Disconnected)?;
    tutorial_notices_transport::emit_notice(
        stream,
        &mut session.admitted,
        &mut session.sequence,
        &oteryn_protocol_oteryn::tutorial_notices::TutorialNotice::StatusText(text),
    )
    .await
    .ok_or(ServeEnd::Disconnected)?;
    if let Some(seen) = session.achievement_notices.as_mut() {
        seen.keys.insert(earned.key.clone());
    }
    Ok(())
}
