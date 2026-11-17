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
    // changes through this bounded refresh. Failed reads share the same bound. This
    // connection's own item commits and maximum-vitals changes (a level) open the gate
    // at once, so the next tick rereads (`reread_after_command`, `reread_after_vitals`).
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
    super::sidecar::write(stream, session, &DeltaSidecar::InventoryCapacity(capacity)).await?;
    session.inventory_capacity = Some(capacity);
    Ok(())
}

/// INVCAP-1: a command of this connection that committed an item change (a move, a pickup or
/// drop, a use, an NPC trade) makes the next visibility tick reread the capacity.
pub(super) fn reread_after_command(session: &mut ServeState, dispatch: &Dispatch) {
    let committed = match dispatch {
        Dispatch::ItemMove(outcome, _, _) => {
            *outcome == oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved
        }
        Dispatch::Use(outcome) => {
            outcome.disposition == oteryn_protocol_oteryn::world_object::UseDisposition::Committed
        }
        Dispatch::NpcTrade(_, _, deltas) | Dispatch::NpcShop(_, _, _, deltas) => !deltas.is_empty(),
        _ => false,
    };
    if committed {
        session.inventory_capacity_refresh_at = None;
    }
}

/// INVCAP-1: a changed maximum health or mana (a level gained or lost) changes the capacity
/// too; the next visibility tick rereads it.
pub(super) fn reread_after_vitals(
    session: &mut ServeState,
    vitals: &oteryn_protocol_oteryn::actor_spell::ActorVitals,
) {
    let maxima = (vitals.max_health, vitals.max_mana);
    if session
        .inventory_capacity_vitals_max
        .replace(maxima)
        .is_some_and(|previous| previous != maxima)
    {
        session.inventory_capacity_refresh_at = None;
    }
}
