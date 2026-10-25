//! Committed use effects, in their original inventory/vitals/overlay/notices order.

use super::*;

pub(super) async fn use_effects<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
    outcome: UseOutcome,
) -> Result<(), ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    if command.command_type == COMMAND_TYPE_BED_SLEEP_INTENT
        && outcome.disposition == UseDisposition::OfflineTraining
    {
        return Err(ServeEnd::Disconnected);
    }
    // ITEM-VIEW-1b: a committed `USE` (the chest MINT) may have changed the
    // backpack or open bags; their deltas follow the durable commit when changed.
    // An unreadable backpack fails closed; the resumed snapshot restores it.
    if outcome.disposition == UseDisposition::Committed
        && let Some(view) = session.item_view.as_mut()
        && item_use_view_refresh::refresh(
            stream,
            authority,
            session.actor,
            &mut session.admitted,
            view,
            session.container_views.as_mut(),
            &mut session.sequence,
        )
        .await
        .is_none()
    {
        return Err(ServeEnd::Disconnected);
    }
    // ITEM-USE-1c: a committed food or potion changed the owner's vitals revision
    // (fed time or health/mana); its ACTOR_VITALS delta follows at once, as a spell's.
    if outcome.disposition == UseDisposition::Committed
        && session.admitted.continuity.vitals_revision != 0
        && let Some((to, value)) = authority
            .observe_vitals(session.actor, session.admitted.game_session_id)
            .await
            .filter(|(observed_revision, _)| {
                *observed_revision > session.admitted.continuity.vitals_revision
            })
    {
        let Some((delta_sequence, delta)) = vitals_delta(
            session.generation,
            session.sequence,
            session.admitted.continuity.vitals_revision,
            to,
            &value,
        ) else {
            return Err(ServeEnd::Disconnected);
        };
        session.sequence = delta_sequence;
        session.admitted.continuity.server_sequence = session.sequence;
        if write_frame(stream, &delta).await.is_err() {
            return Err(ServeEnd::Disconnected);
        }
        session.admitted.continuity.vitals_revision = to;
    }
    // WORLD_OBJECT_OVERLAY (domain 2, delta type 1) is Channel-global, not
    // per-connection: its "from" revision is always the committed entry's own
    // revision minus one, since `LocalObjectRuntime::attempt_use` only ever commits
    // by advancing its revision by exactly one.
    if let Some(entry) = outcome.committed {
        // MAP-WIRE-1 contract §3: a bundle World's object states are tile changes
        // of domain 17 and it commits no domain 2 entry. One committed for a session
        // that selected capability 18 has no domain 17 form: fail closed, never send
        // domain 2 or drop the state.
        if !session.overlay_served {
            return Err(ServeEnd::Disconnected);
        }
        let (Some(delta_sequence), Some(prior_revision)) = (
            session.sequence.checked_add(1),
            entry.revision.checked_sub(1),
        ) else {
            return Err(ServeEnd::Disconnected);
        };
        let Ok(payload) = encode_world_object_overlay_delta(&entry) else {
            return Err(ServeEnd::Disconnected);
        };
        let Ok(delta) = encode_state_delta(
            session.generation,
            delta_sequence,
            STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
            prior_revision,
            entry.revision,
            DELTA_TYPE_WORLD_OBJECT_OVERLAY_V1,
            &payload,
        ) else {
            return Err(ServeEnd::Disconnected);
        };
        session.sequence = delta_sequence;
        session.admitted.continuity.server_sequence = session.sequence;
        if write_frame(stream, &delta).await.is_err() {
            return Err(ServeEnd::Disconnected);
        }
        // r4122508665: only record the new overlay revision once the delta that
        // carries it has actually been transmitted.
        session.admitted.continuity.overlay_revision = entry.revision;
    }
    // ACHIEVEMENT-0 §5: one delta per committed `Granted`, after the commit, only
    // with capability 8. The revision advances before the write, so a revision
    // that may have reached the client is never reused, even if the write fails.
    // ACH-NOTIFY-2: an `Unknown` notice with capability 8 fails closed after the
    // result; the resumed connection's snapshot restores the watermark.
    let notice_revision = session
        .admitted
        .continuity
        .achievement_notice_revision
        .filter(|_| session.notices_selected);
    if notice_revision.is_some() && outcome.earned == EarnedNotice::Unknown {
        return Err(ServeEnd::Disconnected);
    }
    if let (EarnedNotice::Earned(earned), Some(from)) = (outcome.earned, notice_revision) {
        let (Some(delta_sequence), Some(to)) =
            (session.sequence.checked_add(1), from.checked_add(1))
        else {
            return Err(ServeEnd::Disconnected);
        };
        let Ok(payload) = encode_achievement_earned(&earned) else {
            return Err(ServeEnd::Disconnected);
        };
        let Ok(delta) = encode_state_delta(
            session.generation,
            delta_sequence,
            STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES,
            from,
            to,
            DELTA_TYPE_ACHIEVEMENT_EARNED_V1,
            &payload,
        ) else {
            return Err(ServeEnd::Disconnected);
        };
        session.sequence = delta_sequence;
        session.admitted.continuity.server_sequence = session.sequence;
        session.admitted.continuity.achievement_notice_revision = Some(to);
        if write_frame(stream, &delta).await.is_err() {
            return Err(ServeEnd::Disconnected);
        }
    }
    Ok(())
}
