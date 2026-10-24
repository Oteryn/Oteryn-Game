//! Timed results and the item-view deltas of a known committed command.
use super::connection::FreshAdmissionAuthority;
use super::item_view::{ItemViewDelta, SessionItemView};
use crate::foundation::{ExactActorRef, GameSessionId};
use oteryn_protocol_oteryn::timed_item::{TimedDisposition, decode_timed_item_result};

pub(super) fn accepted(payload: &[u8]) -> bool {
    decode_timed_item_result(payload).is_ok_and(|result| result.disposition == TimedDisposition::Ok)
}

pub(super) async fn refresh<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    session: GameSessionId,
    payload: &[u8],
    view: Option<&mut SessionItemView>,
) -> Option<Vec<ItemViewDelta>> {
    let mut deltas = Vec::new();
    if accepted(payload)
        && let Some(view) = view
    {
        let inventory =
            super::item_move::observe_inventory(authority, actor, session, view).await?;
        deltas.extend(view.inventory_committed(inventory).ok()?);
    }
    Some(deltas)
}

/// The bounded command boundary, kept outside the shared connection loop.
pub(super) async fn dispatch<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    command: super::connection::UseCommand,
    payload: &[u8],
    view: Option<&mut SessionItemView>,
) -> Option<(Vec<u8>, Vec<ItemViewDelta>)> {
    let target = oteryn_protocol_oteryn::timed_item::decode_timed_item_intent(payload)
        .ok()
        .and_then(|intent| view.as_ref().and_then(|view| view.resolve(intent.target)));
    let payload = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        authority.timed_item(actor, command, payload, target),
    )
    .await
    .ok()??;
    let deltas = refresh(authority, actor, command.game_session_id, &payload, view).await?;
    Some((payload, deltas))
}
