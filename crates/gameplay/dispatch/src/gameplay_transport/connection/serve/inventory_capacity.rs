//! INVCAP-1: send the own character's capacity when it differs from the last one sent.
use super::*;
use oteryn_protocol_oteryn::inventory_capacity::CAPABILITY_INVENTORY_CAPACITY_V1;
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
        .contains(CAPABILITY_INVENTORY_CAPACITY_V1)
        || session.visibility.is_none()
    {
        return Ok(());
    }
    // Visibility ticks every 250 ms; capacity is advisory and also sees out-of-connection
    // changes through this bounded refresh. Failed reads share the same bound.
    let now = tokio::time::Instant::now();
    if session
        .inventory_capacity_refresh_at
        .is_some_and(|next| now < next)
    {
        return Ok(());
    }
    session.inventory_capacity_refresh_at = Some(now + std::time::Duration::from_secs(5));
    let Some(capacity) = authority.observe_inventory_capacity(session.admitted).await else {
        return Ok(());
    };
    if session.inventory_capacity == Some(capacity) {
        return Ok(());
    }
    super::sidecar::write(
        stream,
        authority,
        session,
        &DeltaSidecar::InventoryCapacity(capacity),
    )
    .await?;
    session.inventory_capacity = Some(capacity);
    Ok(())
}
