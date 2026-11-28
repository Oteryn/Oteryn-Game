use super::*;
use oteryn_protocol_oteryn::party_skull::{CAPABILITY_PARTY_SKULL_V1, encode_party_icons};

pub(super) async fn refresh<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
    retry_busy: bool,
) -> Result<(), ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    if !session
        .selected_capabilities
        .contains(CAPABILITY_PARTY_SKULL_V1)
    {
        session.party_icons_retry_at = None;
        return Ok(());
    }
    let Some(view) = session.visibility.as_ref() else {
        session.party_icons_retry_at = None;
        return Ok(());
    };
    let mut icons = match authority
        .observe_party_icons_for_push(session.admitted)
        .await
    {
        Ok(icons) => {
            session.party_icons_retry_at = None;
            icons
        }
        Err(error) => {
            // Only the dedicated turn schedules retries. An unrelated periodic
            // refresh that found a busy holder must not postpone that turn.
            if retry_busy {
                session.party_icons_retry_at =
                    matches!(error, crate::durability::DurabilityError::RootUnavailable)
                        .then(|| tokio::time::Instant::now() + VISIBILITY_REFRESH);
            }
            return Ok(());
        }
    };
    icons.retain(|icon| view.shows(icon.entity));
    icons.sort_by_key(|icon| icon.entity);
    if icons == session.party_icons {
        return Ok(());
    }
    // The header of the last view sent, where the client holds the actor (see `sidecar`).
    let (content_generation, actor_position) = view.published().ok_or(ServeEnd::Disconnected)?;
    let mut payload =
        oteryn_protocol_oteryn::world_spatial_entities::encode_world_spatial_entities_delta(
            &oteryn_protocol_oteryn::world_spatial_entities::WorldSpatialEntitiesDelta {
                content_generation,
                actor_position,
                enter: Vec::new(),
                update: Vec::new(),
                leave: Vec::new(),
            },
        )
        .map_err(|_| ServeEnd::Disconnected)?;
    let encoded = encode_party_icons(&icons).map_err(|_| ServeEnd::Disconnected)?;
    // Optional field 6 of the existing type-2 visibility delta; no new state domain/type.
    payload.push(0x32);
    let mut len = encoded.len();
    while len >= 128 {
        payload.push((len as u8 & 127) | 128);
        len >>= 7;
    }
    payload.push(len as u8);
    payload.extend_from_slice(&encoded);
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
    session.party_icons = icons;
    Ok(())
}
