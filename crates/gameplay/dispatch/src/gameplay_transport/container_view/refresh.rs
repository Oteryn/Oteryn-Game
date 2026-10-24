//! The shared post-commit refresh of this connection's open container views.

use super::super::connection::FreshAdmissionAuthority;
use super::super::item_view::{ItemViewDelta, ItemViewError, SessionItemView};
use super::ContainerViewState;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::ExactActorRef;

pub(in super::super) async fn refresh<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    fence: Option<CurrentCharacterItemFence>,
    views: &mut ContainerViewState,
    view: &mut SessionItemView,
) -> Result<Option<ItemViewDelta>, ItemViewError> {
    let mut observed = Vec::new();
    for key in views.containers() {
        let observation = authority.observe_container(actor, fence, key).await;
        observed.push((key, observation));
    }
    views.refresh(view, &observed)
}
