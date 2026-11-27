//! One header-only domain-1 type-2 delta carrying a negotiated sidecar (fields 6-8).
use super::*;
use oteryn_protocol_oteryn::world_spatial_entities::{
    DeltaSidecar, encode_world_spatial_entities_sidecar_delta,
};

pub(super) async fn write<S>(
    stream: &mut S,
    session: &mut ServeState,
    sidecar: &DeltaSidecar,
) -> Result<(), ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let view = session
        .visibility
        .as_ref()
        .ok_or_else(|| ServeEnd::server("state_projection_unavailable"))?;
    let (sequence, revision) = write_view(
        stream,
        view,
        session.generation,
        session.sequence,
        session.revision,
        sidecar,
    )
    .await?;
    session.sequence = sequence;
    session.revision = revision;
    session.admitted.continuity.server_sequence = sequence;
    session.admitted.continuity.spatial_revision = revision;
    Ok(())
}

pub(super) async fn write_view<S: AsyncWrite + Unpin>(
    stream: &mut S,
    view: &SessionVisibility,
    generation: u64,
    previous_sequence: u64,
    previous_revision: u64,
    sidecar: &DeltaSidecar,
) -> Result<(u64, u64), ServeEnd> {
    // Repeat the header of the last view sent: the actor may have moved since (a staff
    // relocation during the sidecar read), and the client holds it where that view put it.
    let (content_generation, actor_position) = view
        .published()
        .ok_or_else(|| ServeEnd::server("state_projection_unavailable"))?;
    let payload =
        encode_world_spatial_entities_sidecar_delta(content_generation, actor_position, sidecar)
            .map_err(|_| ServeEnd::server("frame_encode_failed"))?;
    let sequence = previous_sequence
        .checked_add(1)
        .ok_or_else(|| ServeEnd::server("sequence_exhausted"))?;
    let revision = previous_revision
        .checked_add(1)
        .ok_or_else(|| ServeEnd::server("sequence_exhausted"))?;
    let frame = encode_state_delta(
        generation,
        sequence,
        STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
        previous_revision,
        revision,
        2,
        &payload,
    )
    .map_err(|_| ServeEnd::server("state_projection_unavailable"))?;
    write_frame(stream, &frame)
        .await
        .map_err(|error| ServeEnd::Transport(TransportEnd::Write(error.kind())))?;
    Ok((sequence, revision))
}
