//! The shared post-commit refresh of this connection's open container views.

use super::super::connection::FreshAdmissionAuthority;
use super::super::item_view::{ItemViewDelta, ItemViewError, SessionItemView};
use super::ContainerViewState;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::ExactActorRef;
use std::collections::BTreeSet;

pub(in super::super) async fn refresh<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    fence: Option<CurrentCharacterItemFence>,
    views: &mut ContainerViewState,
    view: &mut SessionItemView,
) -> Result<Option<ItemViewDelta>, ItemViewError> {
    let mut observed = Vec::new();
    let mut seen = BTreeSet::new();
    for key in views.containers() {
        // Several IDs may browse one tile at independent page indexes. Read it once so all
        // aliases receive one authoritative observation, without repeated durable reads.
        if !seen.insert(key) {
            continue;
        }
        let Some(observation) = authority
            .observe_container_with_house_capacity(actor, fence, key, views.house_capacity_v2())
            .await
        else {
            // A busy/yielded background read is not evidence that the container
            // is gone. Keep this projection intact and retry on the next tick.
            view.inventory_refresh_pending = true;
            return Ok(None);
        };
        observed.push((key, Some(observation)));
    }
    views.refresh(view, &observed)
}
