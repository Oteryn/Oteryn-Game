//! Command effects emitted after the command result.

use super::*;

pub(super) async fn step<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
    outcome: StepOutcome,
) -> Result<(), ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    if let Some(observation) = outcome.moved_to
        && moved_step_frames(
            stream,
            authority,
            session.actor,
            &mut session.admitted,
            session.visibility.as_mut(),
            session.item_view.as_mut(),
            session.world_map.as_mut(),
            session.generation,
            &mut session.sequence,
            &mut session.revision,
            &mut session.appearances,
            observation,
        )
        .await
        .is_none()
    {
        return Err(ServeEnd::Disconnected);
    }
    if tutorial_notices_transport::emit(
        stream,
        authority,
        &mut session.admitted,
        &mut session.sequence,
    )
    .await
    .is_none()
    {
        return Err(ServeEnd::Disconnected);
    }
    Ok(())
}

pub(super) async fn spell<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
    outcome: SpellCastOutcome,
) -> Result<(), ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    // ACTOR_VITALS (domain 3, delta type 1) carries the owner's own per-actor
    // revision: a committed cast advances it by exactly one. Revisions start at 1, so
    // 0 means the join carried no ACTOR_VITALS snapshot: there is no base to delta
    // from, and the session keeps its spell result without the domain.
    let observed = authority
        .observe_vitals(session.actor, session.admitted.game_session_id)
        .await;
    if session.admitted.continuity.vitals_revision != 0
        && let Some((to, value)) = observed
            .filter(|(observed_revision, _)| {
                *observed_revision > session.admitted.continuity.vitals_revision
            })
            .or(outcome.vitals.filter(|(observed_revision, _)| {
                *observed_revision > session.admitted.continuity.vitals_revision
            }))
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
    Ok(())
}

pub(super) async fn item_move<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
    outcome: ItemMoveOutcome,
    deltas: Vec<ItemViewDelta>,
) -> Result<(), ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    for delta in deltas {
        let Some((delta_sequence, frame)) =
            item_view_delta(session.generation, session.sequence, &delta)
        else {
            return Err(ServeEnd::Disconnected);
        };
        session.sequence = delta_sequence;
        session.admitted.continuity.server_sequence = session.sequence;
        if write_frame(stream, &frame).await.is_err() {
            return Err(ServeEnd::Disconnected);
        }
    }
    // ITEM-LOOP-1: a drop or pickup changed a tile in view; domain 17 shows it now.
    if matches!(outcome, ItemMoveOutcome::Moved)
        && let (Some(view), Some(items)) = (session.world_map.as_mut(), session.item_view.as_mut())
    {
        let update = world_map_delta(
            authority,
            session.actor,
            session.generation,
            session.sequence,
            view,
            items,
        )
        .await;
        session.admitted.continuity.item_view = items.continuity();
        let Some(update) = update else {
            return Err(ServeEnd::Disconnected);
        };
        if let Some((delta_sequence, frame)) = update {
            session.sequence = delta_sequence;
            session.admitted.continuity.server_sequence = session.sequence;
            session.admitted.continuity.world_map_revision = view.revision();
            if write_frame(stream, &frame).await.is_err() {
                return Err(ServeEnd::Disconnected);
            }
        }
    }
    Ok(())
}

pub(super) async fn npc_trade<S>(
    stream: &mut S,
    session: &mut ServeState,
    update: NpcUpdate,
    deltas: Vec<ItemViewDelta>,
) -> Result<(), ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    for delta in deltas {
        let Some((delta_sequence, frame)) =
            item_view_delta(session.generation, session.sequence, &delta)
        else {
            return Err(ServeEnd::Disconnected);
        };
        session.sequence = delta_sequence;
        session.admitted.continuity.server_sequence = session.sequence;
        if write_frame(stream, &frame).await.is_err() {
            return Err(ServeEnd::Disconnected);
        }
    }
    let Some(update_frames) = npc_update_frames(
        session.generation,
        &mut session.sequence,
        &mut session.admitted.continuity.npc,
        &update,
        &mut session.npc_shown,
    ) else {
        return Err(ServeEnd::Disconnected);
    };
    session.admitted.continuity.server_sequence = session.sequence;
    for frame in update_frames {
        if write_frame(stream, &frame).await.is_err() {
            return Err(ServeEnd::Disconnected);
        }
    }
    Ok(())
}

pub(super) async fn npc<S>(
    stream: &mut S,
    session: &mut ServeState,
    update: NpcUpdate,
) -> Result<(), ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let Some(update_frames) = npc_update_frames(
        session.generation,
        &mut session.sequence,
        &mut session.admitted.continuity.npc,
        &update,
        &mut session.npc_shown,
    ) else {
        return Err(ServeEnd::Disconnected);
    };
    session.admitted.continuity.server_sequence = session.sequence;
    for frame in update_frames {
        if write_frame(stream, &frame).await.is_err() {
            return Err(ServeEnd::Disconnected);
        }
    }
    Ok(())
}
