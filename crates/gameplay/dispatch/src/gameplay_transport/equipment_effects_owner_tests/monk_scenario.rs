//! Test-only access to the existing monk owner fixture; no second fixture registration.

use super::fixture::{Harness, debug, id, run};
use super::{
    ComposedFreshAdmission, Dependencies, TestResult, fresh_fixture, prepare_current_session,
};

/// Reuse the inventoried owner and isolated database for equipment dispatch scenarios.
pub(in crate::gameplay_transport) fn run_equipment_scenario(
    scenario: impl for<'a, 'f, 's> AsyncFnOnce(
        ComposedFreshAdmission<'a, 'f, 's>,
        &'a sqlx::PgPool,
        crate::foundation::fresh_admission_durability::FreshAdmissionCommitRequestV1,
    ) -> TestResult,
) -> TestResult {
    run(async |admin| {
        // Harness database names are unique, but its retained recovery directory uses this
        // tag verbatim. Concurrent equipment scenarios must not remove each other's store.
        static NEXT_FIXTURE: std::sync::atomic::AtomicUsize =
            std::sync::atomic::AtomicUsize::new(0);
        let tag = format!(
            "equip_effects_{}",
            NEXT_FIXTURE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        );
        let harness = Harness::create_with_admission_seed(admin, &tag, true, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        // Recreate the exact request from its original verification time, which can precede
        // the durable decision's second. A fresh timestamp would change the replay binding.
        let encoded: String = sqlx::query_scalar(
            "SELECT CASE WHEN octet_length(operation_json) <= 65536 THEN operation_json END \
             FROM game_durability_fresh_admission_receipts \
             WHERE game_session_id=encode($1,'hex')::uuid",
        )
        .bind(id(50).as_slice())
        .fetch_one(&harness.pool)
        .await?;
        let operation =
            crate::durability::fresh_admission::decode_operation(&encoded, 65536).map_err(debug)?;
        let request =
            fresh_fixture::fresh::Source::new(operation.authorization.verified_at)?.request()?;
        assert_eq!(request.operation(), &operation);
        scenario(
            dependencies.owner(&harness, &authority),
            &harness.pool,
            request,
        )
        .await?;
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
