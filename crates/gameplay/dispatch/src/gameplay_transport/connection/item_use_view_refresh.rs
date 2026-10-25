//! Send the inventory and open-container changes of one committed USE in the same turn.

use super::*;

pub(super) async fn refresh<S: AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
    stream: &mut S,
    authority: &A,
    actor: ExactActorRef,
    admitted: &mut AdmittedSession,
    view: &mut SessionItemView,
    views: Option<&mut ContainerViewState>,
    sequence: &mut u64,
) -> Option<()> {
    let inventory =
        item_move::observe_inventory(authority, actor, admitted.game_session_id, view).await?;
    let changed = view.inventory_committed(inventory);
    admitted.continuity.item_view = view.continuity();
    let mut deltas: Vec<_> = changed.ok()?.into_iter().collect();
    if let Some(views) = views {
        let refreshed = super::super::container_view::refresh::refresh(
            authority,
            actor,
            admitted.item_fence,
            views,
            view,
        )
        .await;
        admitted.continuity.item_view = view.continuity();
        deltas.extend(refreshed.ok()?);
    }
    for delta in deltas {
        let (delta_sequence, frame) =
            item_view_delta(admitted.continuity.connection_generation, *sequence, &delta)?;
        *sequence = delta_sequence;
        admitted.continuity.server_sequence = *sequence;
        write_frame(stream, &frame).await.ok()?;
    }
    Some(())
}
