//! Detached BANK-NPC-1 dialogue policy (BANK-0 §4.3, §6; accepted NPC-0 §4).
//!
//! No bank writer, occurrence or authority is created here. The caller must resolve current
//! session/dialogue/range, Bank service, Character/Account/World and junior facts, resolve a
//! transfer recipient in the sender's World, and perform prompt checks before storing a prompt.
//! A matching confirmation is only dialogue data. The future adapter must issue the occurrence
//! bound to the confirming CommandRef and call the existing fenced bank writer. It must keep
//! unknown commit outcomes unsequenced; a storage error is never a BankResult refusal.

use oteryn_protocol_oteryn::GameSessionId;
use oteryn_protocol_oteryn::npc_service::{
    MAX_TALK_TEXT_BYTES, TRAVEL_CONFIRMATION_TIMEOUT_SECONDS,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

use crate::domain::character_name::CharacterName;
use crate::domain::{CharacterId, WorldId};
use crate::durability::bank::{
    BankDepositRequest, BankResult, BankTransferRequest, BankWithdrawRequest,
};
use crate::durability::bank_audit::{BANK_DEPOSIT_MAX, BANK_TRANSFER_MAX, BANK_WITHDRAW_MAX};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BankNpcError {
    InvalidInput,
    AmountOutOfRange,
    UnknownRecipient,
    InvalidContext,
    TimeOverflow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BankNpcIntent {
    Balance,
    Deposit(u64),
    DepositAll,
    Withdraw(u64),
    Transfer { amount: u64, name: CharacterName },
}

fn amount(value: &str, cap: u64) -> Result<u64, BankNpcError> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(BankNpcError::InvalidInput);
    }
    value
        .parse::<u64>()
        .ok()
        .filter(|amount| (1..=cap).contains(amount))
        .ok_or(BankNpcError::AmountOutOfRange)
}

/// Exact bounded bank commands; no substring matches, coin exchange or promotion command.
pub(crate) fn parse(text: &str) -> Result<BankNpcIntent, BankNpcError> {
    if text.len() > MAX_TALK_TEXT_BYTES || text.chars().any(char::is_control) {
        return Err(BankNpcError::InvalidInput);
    }
    let words = text.trim_matches(' ').splitn(5, ' ').collect::<Vec<_>>();
    let is = |word: &str, expected: &str| word.eq_ignore_ascii_case(expected);
    match words.as_slice() {
        [word] if is(word, "balance") => Ok(BankNpcIntent::Balance),
        [verb, all] if is(verb, "deposit") && is(all, "all") => Ok(BankNpcIntent::DepositAll),
        [verb, value, gold] if is(verb, "deposit") && is(gold, "gold") => {
            amount(value, BANK_DEPOSIT_MAX).map(BankNpcIntent::Deposit)
        }
        [verb, value, gold] if is(verb, "withdraw") && is(gold, "gold") => {
            amount(value, BANK_WITHDRAW_MAX).map(BankNpcIntent::Withdraw)
        }
        [verb, value, gold, to, name]
            if is(verb, "transfer") && is(gold, "gold") && is(to, "to") =>
        {
            Ok(BankNpcIntent::Transfer {
                amount: amount(value, BANK_TRANSFER_MAX)?,
                name: CharacterName::parse(name).map_err(|_| BankNpcError::UnknownRecipient)?,
            })
        }
        _ => Err(BankNpcError::InvalidInput),
    }
}

/// Prompt data after caller-owned resolution. `deposit all` is already an exact amount here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BankNpcOperation {
    Deposit(u64),
    Withdraw(u64),
    Transfer {
        amount: u64,
        recipient: CharacterId,
        name: CharacterName,
    },
}

impl BankNpcOperation {
    pub(crate) fn prompt(&self) -> String {
        match self {
            Self::Deposit(amount) => format!("Do you want to deposit {amount} gold?"),
            Self::Withdraw(amount) => format!("Do you want to withdraw {amount} gold?"),
            Self::Transfer { amount, name, .. } => format!(
                "Do you want to transfer {amount} gold to {}?",
                name.as_str()
            ),
        }
    }

    fn valid_amount(&self) -> bool {
        let (amount, cap) = match self {
            Self::Deposit(amount) => (amount, BANK_DEPOSIT_MAX),
            Self::Withdraw(amount) => (amount, BANK_WITHDRAW_MAX),
            Self::Transfer { amount, .. } => (amount, BANK_TRANSFER_MAX),
        };
        (1..=cap).contains(amount)
    }

    /// Consistency checks only; native identities, coin facts and fences remain writer-owned.
    pub(crate) fn matches_deposit(&self, request: &BankDepositRequest) -> bool {
        matches!(self, Self::Deposit(amount) if *amount == request.amount)
    }

    pub(crate) fn matches_withdraw(&self, request: &BankWithdrawRequest) -> bool {
        matches!(self, Self::Withdraw(amount) if *amount == request.amount)
    }

    pub(crate) fn matches_transfer(&self, request: &BankTransferRequest) -> bool {
        matches!(self, Self::Transfer { amount, recipient, name }
            if *amount == request.amount && *recipient == request.recipient_character_id
                && name.comparison_key() == request.recipient_name_key)
    }

    /// Only a terminal BankResult of this operation, returned by the native writer, belongs
    /// here. The caller must check the request/record and occurrence binding before replying.
    pub(crate) fn reply(&self, result: BankResult) -> String {
        match result {
            BankResult::Ok => match self {
                Self::Deposit(amount) => format!("You have deposited {amount} gold."),
                Self::Withdraw(amount) => format!("You have withdrawn {amount} gold."),
                Self::Transfer { amount, name, .. } => format!(
                    "Very well. You have transferred {amount} gold to {}.",
                    name.as_str()
                ),
            },
            BankResult::UnknownRecipient => "This player does not exist.".into(),
            BankResult::RecipientCannotReceiveTransfers => {
                "You cannot transfer money to this account.".into()
            }
            BankResult::SameAccount => "This character already shares your bank balance.".into(),
            BankResult::InsufficientBalance => "There is not enough gold on your account.".into(),
            BankResult::JuniorAccount => {
                "You can use the bank once you have left the island.".into()
            }
            BankResult::BalanceLimit if matches!(self, Self::Transfer { .. }) => {
                "You cannot transfer money to this account.".into()
            }
            BankResult::BalanceLimit => "Your bank balance cannot receive that amount.".into(),
            BankResult::InsufficientCoins => "You do not have enough gold in your backpack.".into(),
            BankResult::NoRoom => "There is not enough room in your backpack.".into(),
        }
    }
}

/// Unqualified binding data, not proof of current authority. The owner must independently
/// resolve an open, in-range Bank conversation and retire pending data on reconnect/transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BankDialogueContext {
    pub(crate) session: GameSessionId,
    pub(crate) character: CharacterId,
    pub(crate) world: WorldId,
    pub(crate) npc: EntityRef,
    pub(crate) conversation_revision: u64,
}

/// One pending invocation. Not Clone: a response consumes it, including cancellation.
#[derive(Debug)]
pub(crate) struct BankConfirmation {
    operation: BankNpcOperation,
    context: BankDialogueContext,
    opened_at_ms: u64,
    expires_at_ms: u64,
}

impl BankConfirmation {
    pub(crate) fn new(
        operation: BankNpcOperation,
        context: BankDialogueContext,
        now_ms: u64,
    ) -> Result<Self, BankNpcError> {
        if !operation.valid_amount() {
            return Err(BankNpcError::AmountOutOfRange);
        }
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
    pub(crate) fn respond(
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

/// Privacy-safe refusals before a transfer prompt; no unresolved recipient identity is made up.
pub(crate) fn refusal_reply(result: BankResult) -> &'static str {
    match result {
        BankResult::UnknownRecipient => "This player does not exist.",
        BankResult::SameAccount => "This character already shares your bank balance.",
        BankResult::JuniorAccount => "You can use the bank once you have left the island.",
        _ => "You cannot transfer money to this account.",
    }
}
