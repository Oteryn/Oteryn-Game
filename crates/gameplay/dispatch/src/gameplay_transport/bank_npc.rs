//! Detached BANK-NPC-1 dialogue policy (BANK-0 §4.3, §6; accepted NPC-0 §4).
//!
//! No bank writer, occurrence or authority is created here. The caller must resolve current
//! session/dialogue/range, Bank service, Character/Account/World and junior facts, resolve a
//! transfer recipient in the sender's World, and perform prompt checks before storing a prompt.
//! A matching confirmation is only dialogue data. The future adapter must issue the occurrence
//! bound to the confirming CommandRef and call the existing fenced bank writer. It must keep
//! unknown commit outcomes unsequenced; a storage error is never a BankResult refusal.

use crate::durability::bank::{
    BankDepositRequest, BankResult, BankTransferRequest, BankWithdrawRequest,
};
use crate::durability::bank_audit::{BANK_DEPOSIT_MAX, BANK_TRANSFER_MAX, BANK_WITHDRAW_MAX};

pub use oteryn_gameplay_npc::bank_npc::{
    BankDialogueContext, BankNpcError, BankNpcIntent, BankNpcOperation,
};

/// Exact bounded bank commands; no substring matches, coin exchange or promotion command.
pub fn parse(text: &str) -> Result<BankNpcIntent, BankNpcError> {
    oteryn_gameplay_npc::bank_npc::parse(
        text,
        BANK_DEPOSIT_MAX,
        BANK_WITHDRAW_MAX,
        BANK_TRANSFER_MAX,
    )
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

    fn reply(&self, result: BankResult) -> String {
        oteryn_gameplay_npc::bank_npc::reply(self, result)
    }
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
        if !oteryn_gameplay_npc::bank_npc::valid_amount(
            &operation,
            BANK_DEPOSIT_MAX,
            BANK_WITHDRAW_MAX,
            BANK_TRANSFER_MAX,
        ) {
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

pub use oteryn_gameplay_npc::bank_npc::refusal_reply;
