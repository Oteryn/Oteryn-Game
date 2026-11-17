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
    // Repeat the header of the last view sent: the actor may have moved since (a staff
    // relocation during the sidecar read), and the client holds it where that view put it.
    let (content_generation, actor_position) = session
        .visibility
        .as_ref()
        .and_then(SessionVisibility::published)
        .ok_or(ServeEnd::Disconnected)?;
    let payload =
        encode_world_spatial_entities_sidecar_delta(content_generation, actor_position, sidecar)
            .map_err(|_| ServeEnd::Disconnected)?;
    let sequence = session
        .sequence
        .checked_add(1)
        .ok_or(ServeEnd::Disconnected)?;
    let revision = session
        .revision
        .checked_add(1)
        .ok_or(ServeEnd::Disconnected)?;
    let frame = encode_state_delta(
        session.generation,
        sequence,
        STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
        session.revision,
        revision,
        2,
        &payload,
    )
    .map_err(|_| ServeEnd::Disconnected)?;
    write_frame(stream, &frame)
        .await
        .map_err(|_| ServeEnd::Disconnected)?;
    session.sequence = sequence;
    session.revision = revision;
    session.admitted.continuity.server_sequence = sequence;
    session.admitted.continuity.spatial_revision = revision;
    Ok(())
}
