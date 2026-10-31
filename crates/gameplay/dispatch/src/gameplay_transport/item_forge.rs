//! Exaltation Forge composition: authenticated command, retained receipt, then native mutation.
use super::{
    ComposedFreshAdmission,
    connection::AdmittedSession,
    item_view::{ItemKey, SessionItemView},
};
use crate::durability::character_forge_dust::{ForgeDustLimitError, ForgeResolvedItems};
use crate::foundation::{CommandId, CommandRef, RuntimeScopeRefV1};
use oteryn_protocol_oteryn::item_forge::{CAPABILITY_ITEM_FORGE_V1, Operation, Request};

macro_rules! authority_methods {
    () => {
        fn item_forge_available(&self) -> bool {
            false
        }
        fn item_forge(
            &self,
            _admitted: AdmittedSession,
            _command_id: u64,
            _request: oteryn_protocol_oteryn::item_forge::Request,
            _item_view: Option<&crate::gameplay_transport::item_view::SessionItemView>,
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { Some(Vec::new()) }
        }
    };
}
pub(super) use authority_methods;

macro_rules! composed_methods {
    () => {
        fn item_forge_available(&self) -> bool {
            self.active_generation
                .and_then(|generation| generation.native_gameplay())
                .is_some()
        }
        async fn item_forge(
            &self,
            admitted: AdmittedSession,
            command_id: u64,
            request: oteryn_protocol_oteryn::item_forge::Request,
            item_view: Option<&crate::gameplay_transport::item_view::SessionItemView>,
        ) -> Option<Vec<u8>> {
            item_forge::execute(self, admitted, command_id, request, item_view).await
        }
    };
}
pub(super) use composed_methods;

pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    request: Request,
    item_view: Option<&SessionItemView>,
) -> Option<Vec<u8>> {
    let (Some(fence), Some(actor), Ok(id)) = (
        admitted.item_fence,
        admitted.runtime_actor,
        CommandId::new(command_id),
    ) else {
        return Some(Vec::new());
    };
    if !request.is_canonical()
        || !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_ITEM_FORGE_V1)
        || fence.game_session_id != admitted.game_session_id
        || fence.connection_generation.get() != admitted.continuity.connection_generation
    {
        return Some(Vec::new());
    }
    // The item mutation lane precedes Channel and database locks.
    let permit = owner.spell_lane_permit().await?;
    {
        let runtime = owner.runtime.lock().await;
        let binding = runtime.binding();
        if admitted.world_id != binding.world_id()
            || admitted.channel_id != binding.channel_id()
            || fence.runtime_scope
                != RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id())
            || fence.scope_ownership_generation != binding.scope_generation()
            || runtime.owner_fence().is_err()
        {
            return Some(Vec::new());
        }
    }
    let command = CommandRef::new(admitted.game_session_id, id);
    // A consumed item, changed content, new trade or cooldown must not change a retained reply.
    // The read independently proves current session, lease, scope and node authority.
    match owner
        .root
        .read_item_forge_reply(owner.character, owner.holder, fence, command, request)
        .await
    {
        Ok(Some(result)) => return serde_json::to_vec(&result).ok(),
        Ok(None) => {}
        Err(ForgeDustLimitError::Unavailable(_)) => return None,
        Err(_) => return Some(Vec::new()),
    }
    // Resolve only after receipt lookup. A reconnect has new handles, and a
    // consumed source no longer has a live handle, but both must still replay.
    let resolved_items = request
        .first_item
        .zip(request.second_item)
        .and_then(|(first, second)| {
            let view = item_view?;
            let (ItemKey::Instance(first), ItemKey::Instance(second)) =
                (view.resolve(first)?, view.resolve(second)?)
            else {
                return None;
            };
            Some(ForgeResolvedItems { first, second })
        });
    if request.operation != Operation::QueryHistory
        && owner.states.spell_states.lock().await.is_dead(actor)
    {
        return Some(Vec::new());
    }
    let native = {
        let runtime = owner.runtime.lock().await;
        if !runtime
            .player_control_facts(actor, admitted.game_session_id)
            .is_ok_and(|facts| facts.control_loss.is_none())
            || super::item_ref_admission::pinned_index(&runtime, owner.active_generation).is_none()
        {
            return Some(Vec::new());
        }
        owner.active_generation?.native_gameplay()?
    };
    // The service resolves coins and cores internally, so it cannot consume an offered trade item.
    if request.operation.changes_inventory()
        && !owner
            .states
            .player_trades
            .physical_service(admitted.game_session_id)
    {
        return Some(Vec::new());
    }
    match owner
        .root
        .execute_item_forge(
            &permit,
            owner.character,
            owner.holder,
            fence,
            command,
            request,
            resolved_items,
            native,
        )
        .await
    {
        Ok(result) => serde_json::to_vec(&result).ok(),
        Err(ForgeDustLimitError::Unavailable(_)) => None,
        Err(_) => Some(Vec::new()),
    }
}

pub(super) fn offered(
    base: &[super::capabilities::OfferedCapability],
    available: bool,
) -> Vec<super::capabilities::OfferedCapability> {
    // Capability 152 remains bundle-world only. Availability may remove an offer, never add
    // it to the production fallback or an Item-view offer that does not contain it.
    base.iter()
        .copied()
        .filter(|capability| available || capability.id != CAPABILITY_ITEM_FORGE_V1)
        .collect()
}

#[cfg(test)]
#[path = "item_forge_tests.rs"]
mod tests;
