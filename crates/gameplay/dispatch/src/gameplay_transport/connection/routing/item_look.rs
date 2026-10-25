//! Item look command body; predicate and connection control flow stay in `connection.rs`.

use super::super::{
    AdmittedSession, CAPABILITY_ITEM_LOOK_V1, ExactActorRef, FreshAdmissionAuthority,
    ItemLookRefusal, ItemLookResult, ItemLookSelection, ItemLookTarget, SelectedCapabilities,
    SessionItemView, SessionVisibility, decode_item_look_intent_for, known_creature_generation,
};
use super::Dispatch;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &mut AdmittedSession,
    command: ClientCommandView<'_>,
    selected_capabilities: SelectedCapabilities,
    visibility: &Option<SessionVisibility>,
    item_view: &Option<SessionItemView>,
) -> Dispatch {
    let selection = ItemLookSelection {
        extended: selected_capabilities.contains(CAPABILITY_ITEM_LOOK_V1),
    };
    if !admitted.continuity.look.admit(tokio::time::Instant::now()) {
        Dispatch::Look(
            ItemLookResult::Refused(ItemLookRefusal::RateLimited),
            selection,
        )
    } else {
        match decode_item_look_intent_for(command.payload, selection) {
            Err(_) => Dispatch::Unregistered,
            Ok(intent) => Dispatch::Look(
                match intent.target {
                    ItemLookTarget::WorldObject(target) => {
                        authority
                            .look_world_object(actor, admitted.game_session_id, &target)
                            .await
                    }
                    ItemLookTarget::Creature(entity) => match known_creature_generation(entity) {
                        Err(refusal) => ItemLookResult::Refused(refusal),
                        Ok(_) if !visibility.as_ref().is_some_and(|view| view.shows(entity)) => {
                            ItemLookResult::Refused(ItemLookRefusal::NotVisible)
                        }
                        Ok(_) => {
                            authority
                                .look_creature(actor, admitted.game_session_id, entity)
                                .await
                        }
                    },
                    ItemLookTarget::Item(handle) => match item_view.as_ref() {
                        None => ItemLookResult::Refused(ItemLookRefusal::NotAvailable),
                        Some(views) if views.resolve(handle).is_none() => {
                            ItemLookResult::Refused(ItemLookRefusal::NotAvailable)
                        }
                        Some(views) => {
                            authority
                                .look_item(
                                    actor,
                                    admitted.game_session_id,
                                    admitted.item_fence,
                                    views,
                                    handle,
                                )
                                .await
                        }
                    },
                },
                selection,
            ),
        }
    }
}
