//! Genuine independently admitted Party member and forced Player movement through Ahau's exit.
use super::TestResult;
use super::fixture::{Harness, id};

#[test]
fn m25_push_peer_uses_genuine_fresh_admission_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("m25-push-peer-fresh", || {
        super::fixture::run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "m25_push_peer", false, false)
                .await?;
            let source = super::house_transfer_fixtures::RecoverySource::start(&h).await?;
            super::house_transfer_fixtures::prepare_owned_session(&h)
                .await
                .map_err(|error| format!("M25 first Fresh admission: {error}"))?;
            let now = sqlx::query_scalar::<_, i64>(
                "SELECT floor(extract(epoch FROM clock_timestamp()))::bigint",
            )
            .fetch_one(&h.pool)
            .await?;
            crate::world_party_runtime_postgres_cases::committed_fresh_player_from_source_owned(
                &crate::world_party_runtime_postgres_cases::FreshPlayerOwners {
                    root: h.root.clone(),
                    node: h.node.clone(),
                    recovery: &h.recovery,
                },
                60,
                63,
                171,
                now,
                Some("platform-test"),
            )
            .await
            .map_err(|error| format!("M25 second Fresh admission: {error}"))?;
            crate::gameplay_transport::native_activity::refresh_activity_accounts(
                &source.evidence,
                &h.root,
                &h.node,
                &[id(40), id(63)],
            )
            .await
            .map_err(|error| format!("M25 independent group account evidence: {error:?}"))?;
            assert!(h.root.pending_native_source_publications(&h.node).await?.is_empty());
            let refreshed: i64 = sqlx::query_scalar(
                "SELECT count(DISTINCT convert_from(semantic_facts,'UTF8')::jsonb->>'account_id') \
                 FROM game_durability_native_source_floors \
                 WHERE source_authority='platform-test' AND operation='ReadAccountSecurityV1' \
                   AND source_revision>1 AND convert_from(semantic_facts,'UTF8')::jsonb->>'allowed'='true' \
                   AND convert_from(semantic_facts,'UTF8')::jsonb->>'account_id'=ANY($1::text[])",
            )
            .bind(vec![
                crate::gameplay_transport::canonical_uuid(&id(40)),
                crate::gameplay_transport::canonical_uuid(&id(63)),
            ])
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(refreshed, 2, "both independent account decisions are current");
            h.cleanup().await
        })
    })?;
    eprintln!("M25_PUSH_PEER_FRESH_COMPLETED duration={:?}", started.elapsed());
    Ok(())
}
