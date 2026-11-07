//! P11: the production observation reads committed account facts with the current session gate.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::account_achievement::{
    AchievementGrantOutcome, AchievementGrantRequest, AchievementSourceEvent,
};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{AuthenticatedTransportRefV1, ConnectionGeneration};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;

#[test]
fn p11_account_notice_postgres_observes_commits_replay_and_session_fences() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "P11 notice scenario panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        super::reset();
        let h =
            Harness::create_with_admission_seed(admin, "p11_account_notices", true, false).await?;
        prepare_current_session(&h).await?;
        {
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let deps = Dependencies::new()?;
            let owner = deps.owner(&h, &authority);
            let current = FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
            let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
            let mut admitted = deps.admitted;
            admitted.item_fence = Some(fence);
            let empty = owner
                .observe_achievement_notice_facts(admitted)
                .await
                .ok_or("initial observation")?;
            assert_eq!(empty.watermark.fact_count, 0);
            assert_eq!(empty.facts, Some(Vec::new()));
            let record = h
                .root
                .read_current_character(&authority, fence.character_id)
                .await
                .map_err(debug)?;
            let gameplay = CurrentCharacterGameplayFence {
                character_id: fence.character_id,
                game_session_id: fence.game_session_id,
                connection_generation: fence.connection_generation,
                character_lease_generation: fence.character_lease_generation,
                runtime_scope: fence.runtime_scope,
                scope_ownership_generation: fence.scope_ownership_generation,
                expected_character_revision: record.revision,
            };
            let requests: Vec<_> = ["here_fishy_fishy", "skin_deep"]
                .into_iter()
                .enumerate()
                .map(|(ordinal, slug)| {
                    let key = format!("oteryn:achievement/{slug}");
                    AchievementGrantRequest {
                        catalogue: deps.achievements.lookup(&key),
                        achievement_key: key,
                        source: AchievementSourceEvent {
                            kind: "oteryn:p11-observation-test".into(),
                            event_id: vec![ordinal as u8 + 1],
                        },
                    }
                })
                .collect();
            let grants = h
                .root
                .commit_test_achievement_grants(&authority, &h.node, gameplay, requests.clone())
                .await
                .map_err(debug)?;
            assert!(
                grants
                    .iter()
                    .all(|grant| matches!(grant, AchievementGrantOutcome::Granted(_)))
            );
            let observed = owner
                .observe_achievement_notice_facts(admitted)
                .await
                .ok_or("committed observation")?;
            assert_eq!(observed.watermark.fact_count, 2);
            assert_eq!(observed.watermark.total_points, 5);
            let facts = observed.facts.as_ref().ok_or("complete facts")?;
            assert_eq!(
                facts
                    .iter()
                    .map(|fact| fact.name.as_str())
                    .collect::<Vec<_>>(),
                ["Here, Fishy Fishy!", "Skin-Deep"]
            );
            assert!(
                facts
                    .iter()
                    .all(|fact| fact.watermark == observed.watermark)
            );
            let replay = h
                .root
                .commit_test_achievement_grants(&authority, &h.node, gameplay, requests)
                .await
                .map_err(debug)?;
            assert!(
                replay
                    .iter()
                    .all(|grant| matches!(grant, AchievementGrantOutcome::AlreadyHeld(_)))
            );
            assert_eq!(
                owner.observe_achievement_notice_facts(admitted).await,
                Some(observed.clone())
            );
            let (facts_count, receipts_count): (i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_account_achievements),(SELECT count(*) FROM game_account_achievement_grant_requests)").fetch_one(&h.pool).await?;
            assert_eq!((facts_count, receipts_count), (2, 2));

            let mut wrong_account = admitted;
            wrong_account
                .controller
                .as_mut()
                .ok_or("controller")?
                .account_id = id(77);
            assert!(
                owner
                    .observe_achievement_notice_facts(wrong_account)
                    .await
                    .is_none()
            );
            let mut wrong_transport = admitted;
            wrong_transport
                .controller
                .as_mut()
                .ok_or("controller")?
                .transport = AuthenticatedTransportRefV1::decode(&[10; 16]).map_err(debug)?;
            assert!(
                owner
                    .observe_achievement_notice_facts(wrong_transport)
                    .await
                    .is_none()
            );
            let mut wrong_generation = admitted;
            wrong_generation.continuity.connection_generation += 1;
            wrong_generation
                .item_fence
                .as_mut()
                .ok_or("item fence")?
                .connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
            assert!(
                owner
                    .observe_achievement_notice_facts(wrong_generation)
                    .await
                    .is_none()
            );
            let mut wrong_lease = admitted;
            wrong_lease
                .item_fence
                .as_mut()
                .ok_or("item fence")?
                .character_lease_generation += 1;
            assert!(
                owner
                    .observe_achievement_notice_facts(wrong_lease)
                    .await
                    .is_none()
            );
            let mut missing_actor = admitted;
            missing_actor.runtime_actor = None;
            assert!(
                owner
                    .observe_achievement_notice_facts(missing_actor)
                    .await
                    .is_none()
            );
            assert_eq!(
                owner.observe_achievement_notice_facts(admitted).await,
                Some(observed)
            );
            deps.loss(&h, 60).await?;
            assert!(
                owner
                    .observe_achievement_notice_facts(admitted)
                    .await
                    .is_none(),
                "a lost controller cannot receive new account observations"
            );
            eprintln!("POSTGRES_ASSERTED P11 account_notices_commits_replay_current_session");
        }
        h.cleanup().await
    })
}
