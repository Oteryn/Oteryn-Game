use super::ComposedFreshAdmission;
use super::connection::{FreshAdmissionAuthority, UseCommand};
use oteryn_protocol_oteryn::raid_schedule::{
    decode_raid_schedule_query, encode_raid_schedule_state,
};
pub(super) type Reply = Option<Result<Vec<u8>, ()>>;
pub(super) async fn dispatch(
    authority: &impl FreshAdmissionAuthority,
    command: UseCommand,
    payload: &[u8],
) -> Reply {
    if decode_raid_schedule_query(payload).is_err() {
        return Some(Err(()));
    }
    authority.raid_schedule(command).await
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
    match owner.root.query_raid_schedule(owner.channel_id).await {
        Ok(state) => encode_raid_schedule_state(&state).ok().map(Ok),
        Err(_) => None,
    }
}
