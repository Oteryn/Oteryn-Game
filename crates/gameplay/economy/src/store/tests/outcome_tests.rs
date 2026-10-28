use super::*;

#[test]
fn all_closed_platform_results_have_game_outcomes() {
    let cases = [
        (
            PlatformPurchaseResult::Accepted,
            PurchaseOutcome::ShowSuccess,
        ),
        (
            PlatformPurchaseResult::StaleQuote,
            PurchaseOutcome::RefreshOffer,
        ),
        (
            PlatformPurchaseResult::OfferUnavailable,
            PurchaseOutcome::ShowOfferUnavailable,
        ),
        (
            PlatformPurchaseResult::PurchaseDisabled,
            PurchaseOutcome::ShowPurchaseDisabled,
        ),
        (
            PlatformPurchaseResult::InsufficientBalance,
            PurchaseOutcome::ShowInsufficientBalance,
        ),
        (
            PlatformPurchaseResult::AccountRestricted,
            PurchaseOutcome::ShowAccountRestricted,
        ),
    ];
    for (answer, expected) in cases {
        assert_eq!(classify_answer(answer), expected);
    }
    assert_eq!(transport_loss_rule(), RetryRule::ReuseSameRequestId);
}
