//! Reuse the native gate's private clock and evidence; no second fixture implementation.
use super::*;

pub(in crate::gameplay_transport) fn current(
    account: [u8; 16],
) -> (
    Arc<PremiumConsumer>,
    Arc<dyn RegisteredPremiumClock>,
    PremiumActivation,
    PremiumEvidence,
) {
    let mut proof = evidence(EntitlementState::Active, T0 + 24 * 60 * MINUTE);
    proof.account_id = account;
    let consumer = Arc::new(PremiumConsumer::default());
    consumer.prove_for_test(proof.clone());
    (
        consumer,
        clock_at(T0 + MINUTE),
        PremiumActivation {
            activation_id: "promotion-test".into(),
            switch_over_us: T0,
        },
        proof,
    )
}
