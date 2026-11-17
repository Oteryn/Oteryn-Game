//! Bulk purchase transport; the session continues to own capability and command sequencing.
pub use oteryn_session::npc_purchase;
use oteryn_session::{CommandStatus, Session, SessionError, SessionStream};
pub async fn send<S: SessionStream>(
    session: &mut Session<S>,
    intent: &npc_purchase::NpcPurchaseIntent,
) -> Result<(CommandStatus, String), SessionError> {
    let payload =
        npc_purchase::encode_npc_purchase(intent).map_err(|_| SessionError::SessionUnusable)?;
    let result = session
        .gated_command(
            npc_purchase::CAPABILITY_NPC_PURCHASE_V1,
            npc_purchase::COMMAND_TYPE_NPC_PURCHASE,
            &payload,
        )
        .await?;
    if result.payload.len() > 1024 {
        return Err(SessionError::SessionUnusable);
    }
    let text =
        String::from_utf8(result.payload.to_vec()).map_err(|_| SessionError::SessionUnusable)?;
    Ok((result.status, text))
}

/// SET/CLEAR/OPEN use the sibling Obtain capability and never mutate loot preferences.
pub async fn obtain<S: SessionStream>(
    session: &mut Session<S>,
    intent: &npc_purchase::ObtainContainerIntent,
) -> Result<oteryn_session::quick_loot::QuickLootResult, SessionError> {
    let payload =
        npc_purchase::encode_obtain_container(intent).map_err(|_| SessionError::SessionUnusable)?;
    let result = session
        .gated_command(
            npc_purchase::CAPABILITY_OBTAIN_CONTAINERS_V1,
            npc_purchase::COMMAND_TYPE_OBTAIN_CONTAINER,
            &payload,
        )
        .await?;
    oteryn_session::quick_loot::decode_quick_loot_result(&result.payload)
        .map_err(|_| SessionError::SessionUnusable)
}
