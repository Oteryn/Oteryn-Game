//! Server-owned scheduled announcement; the player can observe, never activate it.
use super::ComposedFreshAdmission;
use super::capabilities::OfferedCapability;
use super::connection::{FreshAdmissionAuthority, UseCommand};
use crate::durability::character_progression::CharacterProgressionError;
use oteryn_protocol_oteryn::world_event::{CAPABILITY_WORLD_EVENT_V1, decode_world_event_query};

pub(super) const OFFERED: OfferedCapability = OfferedCapability {
    id: CAPABILITY_WORLD_EVENT_V1,
    requires: &[],
};

/// None is an unknown/unavailable commit: disconnect without a terminal result.
pub(super) type Reply = Option<Result<Vec<u8>, ()>>;

pub(super) async fn dispatch(
    authority: &impl FreshAdmissionAuthority,
    command: UseCommand,
    payload: &[u8],
) -> Reply {
    if decode_world_event_query(payload).is_err() {
        return Some(Err(()));
    }
    authority.world_event(command).await
}

pub(super) async fn query(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    command: UseCommand,
) -> Reply {
    let Some(fence) = command.item_fence else {
        return Some(Err(()));
    };
    if fence.game_session_id != command.game_session_id
        || fence.runtime_scope
            != crate::foundation::RuntimeScopeRefV1::channel(owner.world_id, owner.channel_id)
    {
        return Some(Err(()));
    }
    match owner
        .root
        .query_world_event(
            owner.character,
            owner.holder,
            fence,
            command.command_id,
            owner.reset_epoch,
        )
        .await
    {
        Ok(bytes) => Some(Ok(bytes)),
        Err(CharacterProgressionError::Unavailable(_)) => None,
        Err(_) => Some(Err(())),
    }
}
