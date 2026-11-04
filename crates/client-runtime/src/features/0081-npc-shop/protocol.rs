//! Capability-81 NPC shop commands sent through the admitted session.
pub use oteryn_session::npc_service;
pub use oteryn_session::npc_shop;
pub use oteryn_session::{CommandStatus, EntityRef};
pub use oteryn_session::{NpcIntentDisposition, NpcTradeSide};
use oteryn_session::{Session, SessionError, SessionStream};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcShopCommand(pub npc_shop::NpcShopIntent);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcShopResponse {
    pub status: CommandStatus,
    pub result: Option<npc_shop::NpcShopResult>,
}

pub async fn send<S: SessionStream>(
    session: &mut Session<S>,
    command: &NpcShopCommand,
) -> Result<NpcShopResponse, SessionError> {
    let payload =
        npc_shop::encode_npc_shop_intent(&command.0).map_err(|_| SessionError::SessionUnusable)?;
    let reply = session
        .gated_command(
            npc_shop::CAPABILITY_NPC_SHOP_V1,
            npc_shop::COMMAND_TYPE_NPC_SHOP,
            &payload,
        )
        .await?;
    let result = matches!(
        reply.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    )
    .then(|| npc_shop::decode_npc_shop_result(&reply.payload))
    .transpose()
    .map_err(|_| SessionError::SessionUnusable)?;
    Ok(NpcShopResponse {
        status: reply.status,
        result,
    })
}

/// NPC service shares the same admitted command and domain revision owners as the shop.
pub async fn talk<S: SessionStream>(
    session: &mut Session<S>,
    intent: &npc_service::NpcTalkIntent,
) -> Result<NpcIntentDisposition, SessionError> {
    let payload =
        npc_service::encode_npc_talk_intent(intent).map_err(|_| SessionError::SessionUnusable)?;
    let result = session
        .gated_command(
            npc_service::CAPABILITY_NPC_SERVICE_V1,
            npc_service::COMMAND_TYPE_NPC_TALK_INTENT,
            &payload,
        )
        .await?;
    npc_service::decode_npc_intent_result(&result.payload)
        .map_err(|_| SessionError::SessionUnusable)
}

pub async fn buy<S: SessionStream>(
    session: &mut Session<S>,
    intent: &npc_service::NpcTradeIntent,
) -> Result<NpcIntentDisposition, SessionError> {
    let payload =
        npc_service::encode_npc_trade_intent(intent).map_err(|_| SessionError::SessionUnusable)?;
    let result = session
        .gated_command(
            npc_service::CAPABILITY_NPC_SERVICE_V1,
            npc_service::COMMAND_TYPE_NPC_TRADE_INTENT,
            &payload,
        )
        .await?;
    npc_service::decode_npc_intent_result(&result.payload)
        .map_err(|_| SessionError::SessionUnusable)
}
