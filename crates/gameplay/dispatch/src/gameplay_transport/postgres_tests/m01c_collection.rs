//! M01c: imported COLLECT operations retain custody and use the fenced quest owner.
use super::m01c_collection_fixtures::{BINDING};

use super::TestResult;
use super::fixture::{Harness, debug, run};
use oteryn_gameplay_quests::quest::QuestStateCatalogue;
use oteryn_gameplay_quests::quest::authored_bindings::Bindings;

#[test]
fn m01c_collection_requires_committed_item_move_and_exact_carried_inputs_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("M01C_COLLECTION POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("m01c-collection", || {
        run(async |admin| {
            let h = Harness::create(admin, "m01c_collection", true).await?;
            let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
                .fetch_one(&h.pool)
                .await?;
            eprintln!("M01C_COLLECTION POSTGRES_CONNECTED version={version}");
            let bindings = Bindings::parse(BINDING.as_bytes()).map_err(debug)?;
            let base = QuestStateCatalogue::empty("content-1").ok_or("content revision")?;
            let catalogue = bindings.attach(&base).map_err(debug)?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let fence = super::fixture::fence(1)?;
            let request = crate::durability::quest_state::QuestTransitionRequest {
                transition_key: "oteryn:quest-collection/m01c/black-knight-key".into(),
                cause: crate::durability::quest_state::QuestCause::Command(
                    crate::foundation::CommandRef::new(
                        fence.game_session_id,
                        crate::foundation::CommandId::new(80001).map_err(debug)?,
                    ),
                ),
            };
            let result = h
                .root
                .commit_character_quest_transition(
                    &authority,
                    &h.node,
                    fence,
                    request,
                    std::sync::Arc::new(catalogue),
                )
                .await
                .map_err(debug)?;
            assert!(
                matches!(
                    result,
                    crate::durability::quest_state::QuestTransitionOutcome::Refused(
                        oteryn_gameplay_quests::quest::QuestRefusal::StageMismatch
                    )
                ),
                "collection without a committed move and held key must not advance: {result:?}"
            );
            let receipts: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(receipts, 0, "refusal cannot create a completion receipt");
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    });
    eprintln!(
        "M01C_COLLECTION POSTGRES_CASE_COMPLETED duration_ms={} success={}",
        started.elapsed().as_millis(),
        result.is_ok()
    );
    result
}

#[test]
fn m01c_collection_counts_owned_stacks_without_consumption_and_replays_postgres() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-m01c-collection")
}
