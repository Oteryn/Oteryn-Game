//! Detached BANK-NPC-1 dialogue policy (BANK-0 §4.3, §6; accepted NPC-0 §4).
//!
//! No bank writer, occurrence or authority is created here. The caller must resolve current
//! session/dialogue/range, Bank service, Character/Account/World and junior facts, resolve a
//! transfer recipient in the sender's World, and perform prompt checks before storing a prompt.
//! A matching confirmation is only dialogue data. The future adapter must issue the occurrence
//! bound to the confirming CommandRef and call the existing fenced bank writer. It must keep
//! unknown commit outcomes unsequenced; a storage error is never a BankResult refusal.

use oteryn_protocol_oteryn::npc_service::MAX_TALK_TEXT_BYTES;

use crate::domain::character_name::CharacterName;
use crate::durability::bank::{
    BankDepositRequest, BankResult, BankTransferRequest, BankWithdrawRequest,
};
use crate::durability::bank_audit::{BANK_DEPOSIT_MAX, BANK_TRANSFER_MAX, BANK_WITHDRAW_MAX};

pub use oteryn_gameplay_npc::bank_npc::{
    BankDialogueContext, BankNpcError, BankNpcIntent, BankNpcOperation,
};

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
pub fn parse(text: &str) -> Result<BankNpcIntent, BankNpcError> {
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

/// Matching current durable request values and classifying native writer results stay in dispatch.
pub trait BankNpcOperationDurability {
    fn matches_deposit(&self, request: &BankDepositRequest) -> bool;
    fn matches_withdraw(&self, request: &BankWithdrawRequest) -> bool;
    fn matches_transfer(&self, request: &BankTransferRequest) -> bool;
    fn reply(&self, result: BankResult) -> String;
}

impl BankNpcOperationDurability for BankNpcOperation {
    /// Consistency checks only; native identities, coin facts and fences remain writer-owned.
    fn matches_deposit(&self, request: &BankDepositRequest) -> bool {
        matches!(self, Self::Deposit(amount) if *amount == request.amount)
    }

    fn matches_withdraw(&self, request: &BankWithdrawRequest) -> bool {
        matches!(self, Self::Withdraw(amount) if *amount == request.amount)
    }

    fn matches_transfer(&self, request: &BankTransferRequest) -> bool {
        matches!(self, Self::Transfer { amount, recipient, name }
            if *amount == request.amount && *recipient == request.recipient_character_id
                && name.comparison_key() == request.recipient_name_key)
    }

    /// Only a terminal BankResult of this operation, returned by the native writer, belongs
    /// here. The caller must check the request/record and occurrence binding before replying.
    fn reply(&self, result: BankResult) -> String {
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

fn valid_amount(operation: &BankNpcOperation) -> bool {
    let (amount, cap) = match operation {
        BankNpcOperation::Deposit(amount) => (amount, BANK_DEPOSIT_MAX),
        BankNpcOperation::Withdraw(amount) => (amount, BANK_WITHDRAW_MAX),
        BankNpcOperation::Transfer { amount, .. } => (amount, BANK_TRANSFER_MAX),
    };
    (1..=cap).contains(amount)
}

/// One pending invocation. The adapter qualifies the operation's amount before retaining it.
#[derive(Debug)]
pub struct BankConfirmation(oteryn_gameplay_npc::bank_npc::BankConfirmation);

impl BankConfirmation {
    pub fn new(
        operation: BankNpcOperation,
        context: BankDialogueContext,
        now_ms: u64,
    ) -> Result<Self, BankNpcError> {
        if !valid_amount(&operation) {
            return Err(BankNpcError::AmountOutOfRange);
        }
        oteryn_gameplay_npc::bank_npc::BankConfirmation::new(operation, context, now_ms).map(Self)
    }

    pub fn respond(
        self,
        current_context: Option<&BankDialogueContext>,
        text: &str,
        now_ms: u64,
    ) -> Option<BankNpcOperation> {
        self.0.respond(current_context, text, now_ms)
    }
}

/// Privacy-safe refusals before a transfer prompt; no unresolved recipient identity is made up.
pub fn refusal_reply(result: BankResult) -> &'static str {
    match result {
        BankResult::UnknownRecipient => "This player does not exist.",
        BankResult::SameAccount => "This character already shares your bank balance.",
        BankResult::JuniorAccount => "You can use the bank once you have left the island.",
        _ => "You cannot transfer money to this account.",
    }
}
