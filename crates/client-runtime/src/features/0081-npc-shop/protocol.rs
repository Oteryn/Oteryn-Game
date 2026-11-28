//! Capability-81 NPC shop commands sent through the admitted session.
use crate::raw_feature::FeatureCommand;
pub use oteryn_session::atelier::resources;
pub use oteryn_session::container_tree;
pub use oteryn_session::npc_service;
pub use oteryn_session::npc_shop;
pub use oteryn_session::{CharacterInventory, CommandStatus, EntityRef, ItemEntry};
use oteryn_session::{GatedCommandResult, Session, SessionError, SessionStream};
pub use oteryn_session::{NpcIntentDisposition, NpcTradeSide};

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
    let reply = crate::raw_feature::send(session, &shop_command(command, false)?).await?;
    shop_reply(reply)
}

pub fn shop_command(
    command: &NpcShopCommand,
    include_equipped: bool,
) -> Result<FeatureCommand, SessionError> {
    Ok(FeatureCommand {
        capability: npc_shop::CAPABILITY_NPC_SHOP_V1,
        kind: npc_shop::COMMAND_TYPE_NPC_SHOP,
        payload: npc_shop::encode_npc_shop_intent_for(&command.0, include_equipped)
            .map_err(|_| SessionError::SessionUnusable)?,
    })
}

pub fn shop_reply(reply: GatedCommandResult) -> Result<NpcShopResponse, SessionError> {
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
    if !matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        return Ok(NpcIntentDisposition::Rejected);
    }
    npc_service::decode_npc_intent_result(&result.payload)
        .map_err(|_| SessionError::SessionUnusable)
}

pub async fn trade<S: SessionStream>(
    session: &mut Session<S>,
    intent: &npc_service::NpcTradeIntent,
) -> Result<NpcIntentDisposition, SessionError> {
    let reply = crate::raw_feature::send(session, &trade_command(intent, false)?).await?;
    trade_reply(reply)
}

pub fn trade_command(
    intent: &npc_service::NpcTradeIntent,
    pack_in_backpacks: bool,
) -> Result<FeatureCommand, SessionError> {
    Ok(FeatureCommand {
        capability: npc_service::CAPABILITY_NPC_SERVICE_V1,
        kind: npc_service::COMMAND_TYPE_NPC_TRADE_INTENT,
        payload: npc_service::encode_npc_trade_intent_for(intent, pack_in_backpacks)
            .map_err(|_| SessionError::SessionUnusable)?,
    })
}

pub fn trade_reply(result: GatedCommandResult) -> Result<NpcIntentDisposition, SessionError> {
    if !matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        return Ok(NpcIntentDisposition::Rejected);
    }
    npc_service::decode_npc_intent_result(&result.payload)
        .map_err(|_| SessionError::SessionUnusable)
}

/// Compatibility name for the existing NPC prerequisite adapter.
pub use trade as buy;

/// Live balances are queried independently of trade receipts.
pub async fn balances<S: SessionStream>(
    session: &mut Session<S>,
) -> Result<Option<resources::ResourceBalances>, SessionError> {
    if !session.is_selected(resources::CAPABILITY_ATELIER_RESOURCES_V1) {
        return Ok(None);
    }
    let reply = session
        .gated_command(
            resources::CAPABILITY_ATELIER_RESOURCES_V1,
            resources::COMMAND_TYPE_ATELIER_RESOURCES_QUERY,
            &resources::encode_query(),
        )
        .await?;
    if !matches!(
        reply.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        return Ok(None);
    }
    resources::decode_reply(&reply.payload)
        .map(Some)
        .map_err(|_| SessionError::SessionUnusable)
}
