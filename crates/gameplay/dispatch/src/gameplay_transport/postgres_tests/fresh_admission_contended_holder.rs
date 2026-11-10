//! A fresh admission completes while other owners keep using the single durability holder.
use super::fixture::{Harness, debug};
use super::kill_reward_e2e::scenario_thread::run_async as run;
use super::{Dependencies, TestResult};
use std::time::Duration;

#[test]
fn fresh_admission_completes_while_other_owners_share_the_holder_postgres() -> TestResult {
    use crate::foundation::{AuthenticatedTransportRefV1, CharacterId, GameSessionId};
    use crate::gameplay_transport::FreshAdmissionAttempt;
    use crate::gameplay_transport::connection::FreshAdmissionAuthority;

    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "fresh_holder_contended", true, false)
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

            // Other owners keep the single holder about half busy with short passes and retry it
            // through the ordinary command budget whenever they find it busy, as a served session
            // and the Channel cadences do on a node (holder_window: 35-45 % held).
            let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let mut competitors = Vec::new();
            for think in [29_u64, 31, 37, 41] {
                let root = h.root.clone();
                let stop = stop.clone();
                competitors.push(tokio::spawn(async move {
                    while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                        let _ = crate::gameplay_transport::AdmissionRetryBudget::for_command(&root)
                            .retry_durability("contended_holder", async || {
                                root.try_issue_semantic_pass()?
                                    .run(|_, _| {
                                        Box::pin(async {
                                            tokio::time::sleep(Duration::from_millis(5)).await;
                                            Ok(())
                                        })
                                    })
                                    .await
                            })
                            .await;
                        tokio::time::sleep(Duration::from_millis(think)).await;
                    }
                }));
            }
            let admitted = owner.admit(attempt).await;
            stop.store(true, std::sync::atomic::Ordering::Relaxed);
            for competitor in competitors {
                competitor.await?;
            }
            let admitted = admitted.map_err(|refusal| {
                format!("a contended holder refused the fresh admission: {refusal:?}")
            })?;
            assert_eq!(admitted.game_session_id, session);
            eprintln!("PG_EXECUTED fresh_holder_contended admitted=1");
            Ok(())
        }
        .await;
        let cleanup = h.cleanup().await;
        result?;
        cleanup
    })
}
