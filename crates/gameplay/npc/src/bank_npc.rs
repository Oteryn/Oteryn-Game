//! Detached BANK-NPC-1 dialogue policy (BANK-0 §4.3, §6; accepted NPC-0 §4).
//!
//! No bank writer, occurrence or authority is created here. The caller must resolve current
//! session/dialogue/range, Bank service, Character/Account/World and junior facts, resolve a
//! transfer recipient in the sender's World, and perform prompt checks before storing a prompt.
//! A matching confirmation is only dialogue data. The future adapter must issue the occurrence
//! bound to the confirming CommandRef and call the existing fenced bank writer. It must keep
//! unknown commit outcomes unsequenced; a storage error is never a BankResult refusal.

use crate::domain::character_name::CharacterName;
use crate::domain::{CharacterId, WorldId};
use oteryn_protocol_oteryn::GameSessionId;
use oteryn_protocol_oteryn::npc_service::{
    MAX_TALK_TEXT_BYTES, TRAVEL_CONFIRMATION_TIMEOUT_SECONDS,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BankNpcError {
    InvalidInput,
    AmountOutOfRange,
    UnknownRecipient,
    InvalidContext,
    TimeOverflow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BankNpcIntent {
    Balance,
    Deposit(u64),
    DepositAll,
    Withdraw(u64),
    Transfer { amount: u64, name: CharacterName },
}

impl BankNpcIntent {
    /// Deposit and withdraw move coin custody and need current backpack coin facts; balance
    /// and transfer read only the Bank account and never resolve item custody.
    pub const fn uses_coin_custody(&self) -> bool {
        matches!(
            self,
            Self::Deposit(_) | Self::DepositAll | Self::Withdraw(_)
        )
    }
}

/// Prompt data after caller-owned resolution. `deposit all` is already an exact amount here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BankNpcOperation {
    Deposit(u64),
    Withdraw(u64),
    Transfer {
        amount: u64,
        recipient: CharacterId,
        name: CharacterName,
    },
}

impl BankNpcOperation {
    pub fn prompt(&self) -> String {
        match self {
            Self::Deposit(amount) => format!("Do you want to deposit {amount} gold?"),
            Self::Withdraw(amount) => format!("Do you want to withdraw {amount} gold?"),
            Self::Transfer { amount, name, .. } => format!(
                "Do you want to transfer {amount} gold to {}?",
                name.as_str()
            ),
        }
    }
}

/// Unqualified binding data, not proof of current authority. The owner must independently
/// resolve an open, in-range Bank conversation and retire pending data on reconnect/transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BankDialogueContext {
    pub session: GameSessionId,
    pub character: CharacterId,
    pub world: WorldId,
    pub npc: EntityRef,
    pub conversation_revision: u64,
}

/// One pending invocation. Not Clone: a response consumes it, including cancellation.
#[derive(Debug)]
pub struct BankConfirmation {
    operation: BankNpcOperation,
    context: BankDialogueContext,
    opened_at_ms: u64,
    expires_at_ms: u64,
}

impl BankConfirmation {
    /// The dispatch adapter checks the operation amount cap before retaining this prompt.
    pub fn new(
        operation: BankNpcOperation,
        context: BankDialogueContext,
        now_ms: u64,
    ) -> Result<Self, BankNpcError> {
        if context.npc.generation == 0 || context.conversation_revision == 0 {
            return Err(BankNpcError::InvalidContext);
        }
        let expires_at_ms = now_ms
            .checked_add(u64::from(TRAVEL_CONFIRMATION_TIMEOUT_SECONDS) * 1_000)
            .ok_or(BankNpcError::TimeOverflow)?;
        Ok(Self {
            operation,
            context,
            opened_at_ms: now_ms,
            expires_at_ms,
        })
    }

    /// Only an exact ASCII-folded `yes` before expiry matches. Every other response, missing
    /// or changed context, expired/non-monotonic time cancels. The returned value is data;
    /// it cannot qualify current authority or replace the native writer's rechecks.
    pub fn respond(
        self,
        current_context: Option<&BankDialogueContext>,
        text: &str,
        now_ms: u64,
    ) -> Option<BankNpcOperation> {
        (current_context == Some(&self.context)
            && now_ms >= self.opened_at_ms
            && now_ms < self.expires_at_ms
            && text.len() <= MAX_TALK_TEXT_BYTES
            && !text.chars().any(char::is_control)
            && text.trim_matches(' ').eq_ignore_ascii_case("yes"))
        .then_some(self.operation)
    }
}
