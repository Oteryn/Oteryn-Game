use super::*;
use oteryn_protocol_oteryn::pvp_expert::{
    STATE_DOMAIN_EXPERT_FIGHT_MODES, encode_expert_fight_modes,
};

pub(super) async fn refresh<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
) -> Result<(), ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    if !session
        .admitted
        .continuity
        .selected_capabilities
        .domain_selected(STATE_DOMAIN_EXPERT_FIGHT_MODES)
    {
        return Ok(());
    }
    let Some(state) = authority
        .observe_expert_modes(session.actor, session.admitted.game_session_id)
        .await
    else {
        return Ok(());
    };
    let combat = &mut session.admitted.continuity.combat;
    if combat.expert_sent == Some(state) {
        return Ok(());
    }
    let from = combat.expert_revision;
    let to = from.checked_add(1).ok_or(ServeEnd::Disconnected)?;
    let sequence = session
        .sequence
        .checked_add(1)
        .ok_or(ServeEnd::Disconnected)?;
    let frame = oteryn_protocol_oteryn::encode_state_delta(
        session.generation,
        sequence,
        STATE_DOMAIN_EXPERT_FIGHT_MODES,
        from,
        to,
        1,
        &encode_expert_fight_modes(&state),
    )
    .map_err(|_| ServeEnd::Disconnected)?;
    combat.expert_revision = to;
    combat.expert_sent = Some(state);
    session.sequence = sequence;
    session.admitted.continuity.server_sequence = sequence;
    write_frame(stream, &frame)
        .await
        .map_err(|_| ServeEnd::Disconnected)
}
