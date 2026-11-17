//! GUILD-EMBLEM-1: replace the emblems of the visible players when they differ from the last
//! ones sent.
use super::*;
use oteryn_protocol_oteryn::guild_emblem::CAPABILITY_GUILD_EMBLEM_V1;
use oteryn_protocol_oteryn::world_spatial_entities::DeltaSidecar;

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
        .contains(CAPABILITY_GUILD_EMBLEM_V1)
    {
        return Ok(());
    }
    let Some(view) = session.visibility.as_ref() else {
        return Ok(());
    };
    let Some(mut emblems) = authority.observe_guild_emblems(session.admitted).await else {
        return Ok(());
    };
    // Only the players this session currently sees; the session rejects any other.
    emblems.retain(|emblem| view.shows(emblem.entity));
    emblems.sort_by_key(|emblem| emblem.entity);
    if emblems == session.guild_emblems {
        return Ok(());
    }
    let sidecar = DeltaSidecar::GuildEmblems(emblems);
    super::sidecar::write(stream, session, &sidecar).await?;
    if let DeltaSidecar::GuildEmblems(emblems) = sidecar {
        session.guild_emblems = emblems;
    }
    Ok(())
}
