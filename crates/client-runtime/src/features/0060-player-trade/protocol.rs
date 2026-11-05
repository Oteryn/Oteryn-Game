//! Capability-60 commands and the authoritative state that follows each receipt.
pub use oteryn_session::{CommandStatus, EntityRef, player_trade};
use oteryn_session::{Session, SessionError, SessionStream};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerTradeCommand {
    pub intent: player_trade::Intent,
    pub payload: Vec<u8>,
}

impl PlayerTradeCommand {
    pub fn new(intent: player_trade::Intent) -> Option<Self> {
        let payload = player_trade::encode_intent(intent).ok()?;
        Some(Self { intent, payload })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerTradeResponse {
    pub status: CommandStatus,
    pub result: player_trade::ResultCode,
    pub state: player_trade::State,
}

pub async fn send<S: SessionStream>(
    session: &mut Session<S>,
    request: &PlayerTradeCommand,
) -> Result<PlayerTradeResponse, SessionError> {
    if player_trade::decode_intent(&request.payload).ok() != Some(request.intent) {
        return Err(SessionError::CapabilityNotSelected {
            capability: player_trade::CAPABILITY_PLAYER_TRADE_V1,
        });
    }
    let result = session
        .gated_command(
            player_trade::CAPABILITY_PLAYER_TRADE_V1,
            player_trade::COMMAND_TYPE_PLAYER_TRADE_INTENT,
            &request.payload,
        )
        .await?;
    let code =
        player_trade::decode_result(&result.payload).map_err(|_| SessionError::SessionUnusable)?;
    let current = session.player_trade_state().unwrap_or_default();
    let changed = match code {
        player_trade::ResultCode::Offered | player_trade::ResultCode::Completed => true,
        // Repeating an already recorded accept produces a receipt without a new revision.
        player_trade::ResultCode::Accepted => !current.accepted[0],
        player_trade::ResultCode::Cancelled => matches!(current.phase, 1..=3),
        _ => false,
    };
    let state = if changed {
        let delta = session
            .read_gated_delta(player_trade::STATE_DOMAIN_PLAYER_TRADE, 1)
            .await?;
        player_trade::decode_state(&delta.payload).map_err(|_| SessionError::SessionUnusable)?
    } else {
        current
    };
    Ok(PlayerTradeResponse {
        status: result.status,
        result: code,
        state,
    })
}

pub use oteryn_session::{
    CharacterInventory, ItemEntry, OpenContainer, container_tree::ContainerViews,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TradeItems {
    pub inventory: CharacterInventory,
    pub corpse: OpenContainer,
    pub containers: ContainerViews,
    pub can_open: bool,
    pub pending: bool,
}
