//! SEAM-1: the production loss caller must publish reconnectability for a closed socket.
use super::*;
use crate::gameplay_transport::connection::{ControlLossResult, FreshAdmissionAuthority};

#[test]
fn seam_closed_session_becomes_reconnectable() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            run(async |admin| {
                let h = Harness::create_with_admission_seed(
                    admin, "seam_closed_loss", false, false,
                ).await?;
                prepare_current_session(&h).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
                let deps = Dependencies::new()?;
                let owner = deps.owner(&h, &authority);
                deps.initialize(&owner).await;
                owner.runtime.lock().await
                    .initialize_first_entry_position(deps.actor).map_err(debug)?;
                // A transient statement failure rolls the loss transaction back. The sequence
                // survives rollback so the production retry can commit the same epoch once.
                sqlx::raw_sql(
                    "CREATE SEQUENCE seam_loss_attempt;
                     CREATE FUNCTION seam_loss_fail_once() RETURNS trigger
                     LANGUAGE plpgsql SECURITY DEFINER AS $$
                     BEGIN
                         IF NEW.session_state = 1 AND nextval('seam_loss_attempt') = 1 THEN
                             RAISE EXCEPTION 'SEAM-1 transient loss failure' USING ERRCODE = '40001';
                         END IF;
                         RETURN NEW;
                     END $$;
                     CREATE TRIGGER seam_loss_failure
                     BEFORE UPDATE ON game_durability_reconnect_sessions
                     FOR EACH ROW EXECUTE FUNCTION seam_loss_fail_once();",
                ).execute(&h.pool).await?;
                assert_eq!(
                    owner.lose_control(deps.admitted, Duration::ZERO, None).await,
                    ControlLossResult::Unknown,
                    "a rolled-back loss is retryable, not a final refusal",
                );
                assert!(h.root.maintain_ready_once().await?);
                assert_eq!(
                    owner.lose_control(deps.admitted, Duration::ZERO, None).await,
                    ControlLossResult::Recorded,
                );
                let current = FreshAdmissionStore::from_root(h.root.clone())
                    .current_session(deps.admitted.game_session_id).await?;
                assert_eq!(current.session_state(), crate::foundation::GameSessionState::Reconnectable);
                assert_eq!(current.current_control_loss_epoch().map(|epoch| epoch.get()), Some(1));
                assert!(current.current_transport().is_none());
                let receipts: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM game_durability_admission_lifecycle_receipts
                     WHERE operation_key = 'owning-loss-v1'::bytea || $1::bytea || int8send(1)",
                ).bind(deps.admitted.game_session_id.as_bytes().as_slice())
                    .fetch_one(&h.pool).await?;
                assert_eq!(receipts, 1);
                drop(owner);
                drop(seal);
                h.cleanup().await
            }).map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "control loss scenario panicked")??;
    Ok(())
}
