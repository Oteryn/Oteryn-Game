#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformPurchaseResult {
    Accepted,
    StaleQuote,
    OfferUnavailable,
    PurchaseDisabled,
    InsufficientBalance,
    AccountRestricted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PurchaseOutcome {
    ShowSuccess,
    RefreshOffer,
    ShowOfferUnavailable,
    ShowPurchaseDisabled,
    ShowInsufficientBalance,
    ShowAccountRestricted,
}

#[must_use]
pub const fn classify_answer(answer: PlatformPurchaseResult) -> PurchaseOutcome {
    match answer {
        PlatformPurchaseResult::Accepted => PurchaseOutcome::ShowSuccess,
        PlatformPurchaseResult::StaleQuote => PurchaseOutcome::RefreshOffer,
        PlatformPurchaseResult::OfferUnavailable => PurchaseOutcome::ShowOfferUnavailable,
        PlatformPurchaseResult::PurchaseDisabled => PurchaseOutcome::ShowPurchaseDisabled,
        PlatformPurchaseResult::InsufficientBalance => PurchaseOutcome::ShowInsufficientBalance,
        PlatformPurchaseResult::AccountRestricted => PurchaseOutcome::ShowAccountRestricted,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryRule {
    ReuseSameRequestId,
}

#[must_use]
pub const fn transport_loss_rule() -> RetryRule {
    RetryRule::ReuseSameRequestId
}

#[cfg(test)]
#[path = "tests/outcome_tests.rs"]
mod tests;
