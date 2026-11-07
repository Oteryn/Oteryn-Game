//! Container view command body; predicate and connection control flow stay in `connection.rs`.

use super::super::{
    AdmittedSession, ContainerPlan, ContainerViewState, ExactActorRef, FreshAdmissionAuthority,
    SessionItemView, decode_container_view_intent,
};
use super::Dispatch;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &mut AdmittedSession,
    command: ClientCommandView<'_>,
    container_views: &mut Option<ContainerViewState>,
    item_view: &mut Option<SessionItemView>,
) -> Option<Dispatch> {
    Some({
        // BAGS-WIRE-1: non-durable view command. Over BAGS0-RL-04 (10 per second, sliding
        // window) it is REJECTED with an empty payload before decoding, as is a malformed one.
        // The window is per GameSession and travels in the continuity.
        let admitted_rate = match (&*container_views, item_view.as_mut()) {
            (Some(_), Some(view)) => {
                let admitted_rate = view.admit_view_command(tokio::time::Instant::now());
                admitted.continuity.item_view = view.continuity();
                admitted_rate
            }
            _ => false,
        };
        match (
            container_views.as_mut().filter(|_| admitted_rate),
            item_view.as_mut(),
            decode_container_view_intent(command.payload),
        ) {
            (Some(views), Some(view), Ok(intent)) => {
                let decided = match views.plan(view, intent) {
                    Ok(ContainerPlan::Decided(outcome, delta)) => Ok((outcome, delta)),
                    Ok(ContainerPlan::Observe(pending)) => {
                        let observation = authority
                            .observe_container(actor, admitted.item_fence, pending.key())
                            .await;
                        views.apply(view, pending, observation)
                    }
                    Err(error) => Err(error),
                };
                admitted.continuity.item_view = view.continuity();
                let Ok((outcome, delta)) = decided else {
                    return None;
                };
                Dispatch::ContainerView(outcome, delta)
            }
            _ => Dispatch::Unregistered,
        }
    })
}
