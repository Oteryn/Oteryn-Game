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

/// Start Premium with a current lease and advance only the Premium clock past entitlement end.
pub(in crate::gameplay_transport) async fn expiring_travel(
    account: [u8; 16],
    root: &crate::durability::DurabilityRoot,
) -> Result<(SpellPremiumCoordinator, impl Fn() + use<>), DurabilityError> {
    let (_, _, activation, mut proof) = current(account);
    proof.effective_until_us = T0 + 2 * MINUTE;
    let consumer = Arc::new(PremiumConsumer::default());
    consumer.prove_for_test(proof);
    let clock = clock_at(T0 + MINUTE);
    root.latch_premium_activation(
        &activation.activation_id,
        activation.switch_over_us,
        activation.switch_over_us + MINUTE,
    )
    .await?;
    let coordinator = SpellPremiumCoordinator::new(
        consumer,
        Some(clock.clone()),
        Some(activation),
        Some(root.clone()),
    );
    Ok((coordinator, move || {
        *clock.0.lock().unwrap() = TrustedNow::new(T0 + 3 * MINUTE, 1_000_000);
    }))
}
