//! A fresh admission waiting for the busy holder keeps the scheduled monster passes behind it.
use super::fixture::{Harness, debug};
use super::kill_reward_e2e::scenario_thread::run_async as run;
use super::{Dependencies, TestResult};
use std::time::Duration;

#[test]
fn fresh_admission_waiting_for_the_holder_refuses_background_passes_postgres() -> TestResult {
    use crate::foundation::{AuthenticatedTransportRefV1, CharacterId, GameSessionId};
    use crate::gameplay_transport::FreshAdmissionAttempt;
    use crate::gameplay_transport::connection::FreshAdmissionAuthority;

    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "fresh_holder_preference", true, false)
            .await?;
        let result: TestResult = async {
            let source = super::house_transfer::RecoverySource::start(&h).await?;
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs();
            let mut runtime = super::fresh_fixture::fresh::Source::new(i64::try_from(now)?)?;
            runtime.rows.retain(|row| {
                matches!(
                    row.key,
                    crate::foundation::admission_authority_publication::AdmissionAuthorityGuardKeyV1::Runtime(_)
                )
            });
            let publication =
                crate::foundation::admission_authority_publication::AdmissionAuthorityPublicationV1::prepare(
                    &runtime,
                    i64::try_from(now)?,
                )
                .map_err(debug)?;
            h.root
                .publish_runtime_readiness(&h.node, &publication)
                .await
                .map_err(debug)?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let deps = Dependencies::new()?;
            let mut owner = deps.owner(&h, &authority);
            owner.evidence = &source.evidence;
            let token = super::house_transfer::RecoverySource::fresh_token(91)?;
            let session = GameSessionId::decode(&super::fixture::id(91)).map_err(debug)?;
            let attempt = FreshAdmissionAttempt {
                character_id: CharacterId::decode(&super::fixture::id(41)).map_err(debug)?,
                admission_material: token.as_bytes(),
                game_session_id: session,
                transport: AuthenticatedTransportRefV1::decode(&[91; 16]).map_err(debug)?,
                supported_capabilities: &[],
            };

            let held = h.root.try_issue_semantic_pass()?;
            let mut admitting = std::pin::pin!(owner.admit(attempt));
            assert!(
                tokio::time::timeout(Duration::from_millis(100), &mut admitting)
                    .await
                    .is_err(),
                "the admission waits for the busy holder"
            );
            held.run(|_, _| Box::pin(async { Ok(()) })).await?;
            // Without yielding to the admission: a monster pass must not retake the holder.
            let background = h.root.try_issue_background_semantic_pass();
            let preferred = matches!(
                background,
                Err(crate::durability::DurabilityError::RootUnavailable)
            );
            if let Ok(pass) = background {
                pass.run(|_, _| Box::pin(async { Ok(()) })).await?;
            }
            assert!(preferred, "a waiting admission yields to a background pass");
            let admitted = admitting.await.map_err(debug)?;
            assert_eq!(admitted.game_session_id, session);
            h.root
                .try_issue_background_semantic_pass()?
                .run(|_, _| Box::pin(async { Ok(()) }))
                .await?;
            eprintln!("PG_EXECUTED fresh_holder_preference background_refused=1 admitted=1");
            Ok(())
        }
        .await;
        let cleanup = h.cleanup().await;
        result?;
        cleanup
    })
}
