//! One header-only domain-1 type-2 delta carrying a negotiated sidecar (fields 6-7).
use super::*;
use oteryn_protocol_oteryn::world_spatial_entities::{
    DeltaSidecar, encode_world_spatial_entities_sidecar_delta,
};

pub(super) async fn write<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
    sidecar: &DeltaSidecar,
) -> Result<(), ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    let position = authority
        .observe(session.actor)
        .await
        .ok_or(ServeEnd::Disconnected)?;
    let payload = encode_world_spatial_entities_sidecar_delta(
        position.content_generation,
        position.actor_position,
        sidecar,
    )
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
