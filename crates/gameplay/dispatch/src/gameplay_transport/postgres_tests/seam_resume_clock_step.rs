//! NBW0: a wall clock that steps back after the durable loss decision cannot reject a resume.
use super::*;
use crate::foundation::AuthenticatedTransportRefV1;
use crate::gameplay_transport::connection::{
    ControlLossResult, FreshAdmissionAuthority, ResumeAttempt,
};

/// How far the clock steps back after the loss decision. Within the resume's retry window.
const CLOCK_STEP_SECONDS: i64 = 2;

#[test]
fn seam_resume_survives_a_clock_step_back_after_the_loss_decision() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run(async |admin| {
                let h = Harness::create_with_admission_seed(
                    admin,
                    "seam_resume_clock_step",
                    false,
                    false,
                )
                .await?;
                let recovery = super::house_transfer::RecoverySource::start(&h).await?;
                super::house_transfer::prepare_owned_session(&h).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(debug)?;
                let deps = Dependencies::new()?;
                let mut owner = deps.owner(&h, &authority);
                owner.evidence = &recovery.evidence;
                deps.initialize(&owner).await;
                owner
                    .runtime
                    .lock()
                    .await
                    .initialize_first_entry_position(deps.actor)
                    .map_err(debug)?;
                let mut admitted = deps.admitted;
                admitted.first_entry = FirstEntryOutcome::Positioned;
                admitted.item_fence = crate::gameplay_transport::item_fence_of(
                    FreshAdmissionStore::from_root(h.root.clone())
                        .current_session(admitted.game_session_id)
                        .await?,
                );
                owner.states.command_ingress.admit(admitted.game_session_id);
                assert_eq!(
                    owner.lose_control(admitted, Duration::ZERO, None).await,
                    ControlLossResult::Recorded
                );
                // The loss was decided in the second before the wall clock stepped back: every
                // later durable sample is earlier than its stored decision time.
                let mut key = b"owning-loss-v1".to_vec();
                key.extend_from_slice(admitted.game_session_id.as_bytes());
                key.extend_from_slice(&1_u64.to_be_bytes());
                let mut tx = h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role = replica")
                    .execute(&mut *tx)
                    .await?;
                let shifted = sqlx::query(
                    "UPDATE game_durability_admission_lifecycle_receipts \
                     SET decided_at = decided_at + $2 WHERE operation_key = $1",
                )
                .bind(&key)
                .bind(CLOCK_STEP_SECONDS)
                .execute(&mut *tx)
                .await?
                .rows_affected();
                tx.commit().await?;
                assert_eq!(shifted, 1);
                let token = super::house_transfer::RecoverySource::token(2)?;
                let resumed = owner
                    .resume(ResumeAttempt {
                        game_session_id: admitted.game_session_id,
                        recovery_material: token.as_bytes(),
                        transport: AuthenticatedTransportRefV1::decode(&[2; 16])
                            .map_err(debug)?,
                        last_applied_server_sequence: admitted.continuity.server_sequence,
                        supported_capabilities: &[],
                    })
                    .await
                    .map_err(|refusal| format!("resume refused: {refusal:?}"))?;
                assert_eq!(resumed.game_session_id, admitted.game_session_id);
                assert_eq!(resumed.continuity.connection_generation, 2);
                drop(owner);
                drop(seal);
                h.cleanup().await
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "clock step resume thread panicked")??;
    Ok(())
}
