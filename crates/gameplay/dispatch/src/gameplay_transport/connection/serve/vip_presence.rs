use super::*;
use oteryn_protocol_oteryn::vip_presence as wire;

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
        .selected_capabilities
        .contains(wire::CAPABILITY_VIP_PRESENCE_V1)
    {
        return Ok(());
    }
    let Some(payload) = authority.vip_presence_refresh(session.admitted).await else {
        return Ok(());
    };
    let Some((sequence, frame, revision)) = chat_delta(
        session.generation,
        session.sequence,
        session.admitted.continuity.chat_revision,
        wire::DELTA_TYPE_VIP_PRESENCE_V1,
        &payload,
    ) else {
        return Err(ServeEnd::Disconnected);
    };
    session.sequence = sequence;
    session.admitted.continuity.server_sequence = sequence;
    session.admitted.continuity.chat_revision = revision;
    // A blocked writer cannot retain a confirmed ONLINE beyond its bounded visibility window.
    tokio::time::timeout(
        std::time::Duration::from_millis(u64::from(wire::CONFIRMED_FOR_MS)),
        write_frame(stream, &frame),
    )
    .await
    .map_err(|_| ServeEnd::Disconnected)?
    .map_err(|_| ServeEnd::Disconnected)
}
