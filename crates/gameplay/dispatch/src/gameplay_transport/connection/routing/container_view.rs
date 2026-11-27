//! Container view command body; predicate and connection control flow stay in `connection.rs`.

use super::super::{
    AdmittedSession, ContainerPlan, ContainerViewState, ExactActorRef, FreshAdmissionAuthority,
    SessionItemView, decode_container_view_intent,
};
use super::Dispatch;
use oteryn_protocol_oteryn::{ClientCommandView, CommandStatus, container_tree as wire};

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
        if command.command_type == wire::COMMAND_TYPE_CONTAINER_VIEW_SEEK {
            let (Some(views), Some(items), Some(seek)) = (
                container_views.as_mut().filter(|_| admitted_rate),
                item_view.as_mut(),
                wire::ContainerSeek::decode(command.payload),
            ) else {
                return Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new()));
            };
            return seek_page(authority, actor, admitted, views, items, seek, command).await;
        }
        match (
            container_views.as_mut().filter(|_| admitted_rate),
            item_view.as_mut(),
            decode_container_view_intent(command.payload),
        ) {
            (Some(views), Some(view), Ok(intent)) => {
                let decided = match views.plan(view, intent) {
                    Ok(ContainerPlan::Decided(outcome, delta)) => Ok((outcome, delta)),
                    Ok(ContainerPlan::Observe(pending)) => {
                        let Ok(observation) = authority
                            .observe_container_for_use(actor, admitted.item_fence, pending.key(), admitted.continuity.selected_capabilities.contains(oteryn_protocol_oteryn::container_tree::CAPABILITY_HOUSE_CONTAINER_CAPACITY_V2))
                            .await else {
                                return Some(Dispatch::CommandRefused);
                            };
                        views.apply(view, pending, observation)
                    }
                    Err(error) => Err(error),
                };
                admitted.continuity.item_view = view.continuity();
                let (outcome, delta) = match decided {
                    Ok(result) => result,
                    Err(error) => {
                        log_refusal(command, "view_projection", error);
                        (super::super::ContainerViewOutcome::Stale, None)
                    }
                };
                Dispatch::ContainerView(outcome, delta)
            }
            _ => Dispatch::Unregistered,
        }
    })
}

async fn seek_page<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &mut AdmittedSession,
    views: &mut ContainerViewState,
    item_view: &mut SessionItemView,
    seek: wire::ContainerSeek,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    let rejected = || Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new()));
    let Some(key) = views.ordinary_page_container(seek.view_id) else {
        return rejected();
    };
    let Some(observation) = authority
        .observe_container_with_house_capacity(
            actor,
            admitted.item_fence,
            key,
            admitted
                .continuity
                .selected_capabilities
                .contains(wire::CAPABILITY_HOUSE_CONTAINER_CAPACITY_V2),
        )
        .await
    else {
        return rejected();
    };
    let result = match views.apply_page(item_view, seek, observation) {
        Ok(result) => result,
        Err(error) => {
            log_refusal(command, "page_projection", error);
            return rejected();
        }
    };
    admitted.continuity.item_view = item_view.continuity();
    match result {
        Some((page, delta)) => Some(Dispatch::ContainerPage(page, delta)),
        None => rejected(),
    }
}


fn log_refusal(
    command: ClientCommandView<'_>,
    reason: &'static str,
    error: super::super::super::item_view::ItemViewError,
) {
    super::super::super::operator_event(&format!(
        "event=container_view_refused reason={reason} error={error:?} command_id={}",
        command.command_id,
    ));
}
