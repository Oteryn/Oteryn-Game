//! Both players receive their own trade projection on the visibility cadence.
use super::*;
use oteryn_protocol_oteryn::player_trade as wire;
pub(super) async fn refresh<S: AsyncRead + AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
) -> Result<(), ServeEnd> {
    if !session
        .selected_capabilities
        .contains(wire::CAPABILITY_PLAYER_TRADE_V1)
    {
        return Ok(());
    }
    let (revision, state, inventory) = authority
        .player_trade_observation(session.admitted.game_session_id)
        .await
        .ok_or(ServeEnd::Disconnected)?;
    let base = session.admitted.continuity.player_trade_revision;
    if revision < base {
        return Err(ServeEnd::Disconnected);
    }
    if revision > base {
        let payload = wire::encode_state(&state).map_err(|_| ServeEnd::Disconnected)?;
        let sequence = session
            .sequence
            .checked_add(1)
            .ok_or(ServeEnd::Disconnected)?;
        let frame = oteryn_protocol_oteryn::encode_state_delta(
            session.generation,
            sequence,
            wire::STATE_DOMAIN_PLAYER_TRADE,
            base,
            revision,
            1,
            &payload,
        )
        .map_err(|_| ServeEnd::Disconnected)?;
        write_frame(stream, &frame)
            .await
            .map_err(|_| ServeEnd::Disconnected)?;
        session.sequence = sequence;
        session.admitted.continuity.server_sequence = sequence;
        session.admitted.continuity.player_trade_revision = revision;
    }
    if inventory {
        let view = session.item_view.as_mut().ok_or(ServeEnd::Disconnected)?;
        let deltas = authority
            .player_trade_inventory(session.admitted, view, session.container_views.as_mut())
            .await
            .ok_or(ServeEnd::Disconnected)?;
        session.admitted.continuity.item_view = view.continuity();
        for delta in deltas {
            let (sequence, frame) = item_view_delta(session.generation, session.sequence, &delta)
                .ok_or(ServeEnd::Disconnected)?;
            write_frame(stream, &frame)
                .await
                .map_err(|_| ServeEnd::Disconnected)?;
            session.sequence = sequence;
            session.admitted.continuity.server_sequence = sequence;
        }
    }
    authority.player_trade_delivered(session.admitted.game_session_id, revision);
    Ok(())
}
