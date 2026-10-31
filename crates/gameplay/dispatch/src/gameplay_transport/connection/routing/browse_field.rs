//! Browse and seek reuse the connection's container views and their admission window.

use super::super::{AdmittedSession, ContainerPlan, ContainerViewState, ExactActorRef,
    FreshAdmissionAuthority, SessionItemView};
use super::Dispatch;
use oteryn_protocol_oteryn::ClientCommandView;
use oteryn_protocol_oteryn::browse_field::decode_browse_field_intent;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &mut AdmittedSession,
    command: ClientCommandView<'_>,
    container_views: &mut Option<ContainerViewState>,
    item_view: &mut Option<SessionItemView>,
) -> Option<Dispatch> {
    let (Some(views), Some(view)) = (container_views.as_mut(), item_view.as_mut()) else {
        return Some(Dispatch::Unregistered);
    };
    let accepted = view.admit_view_command(tokio::time::Instant::now());
    admitted.continuity.item_view = view.continuity();
    if !accepted { return Some(Dispatch::Unregistered); }
    let Ok(intent) = decode_browse_field_intent(command.payload) else {
        return Some(Dispatch::Unregistered);
    };
    let decided = match views.plan_browse(intent) {
        ContainerPlan::Decided(outcome, delta) => Ok((outcome, delta)),
        ContainerPlan::Observe(pending) => {
            let observation = authority.observe_container(actor, admitted.item_fence, pending.key()).await;
            views.apply(view, pending, observation)
        }
    };
    admitted.continuity.item_view = view.continuity();
    let (outcome, delta) = decided.ok()?;
    Some(Dispatch::ContainerView(outcome, delta))
}
