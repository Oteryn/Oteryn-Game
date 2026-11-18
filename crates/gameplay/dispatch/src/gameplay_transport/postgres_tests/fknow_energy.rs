//! Crystal 201189f, creaturescripts_replica_servants.lua:1-33.
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::world_callbacks::Position;

pub(super) const NAME: &str = "fknow-energy-replica-credit";

#[test]
fn fknow_energy_replica_deaths_credit_the_native_killer_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(
            NAME,
            Position {
                x: 32327,
                y: 32093,
                floor: -7,
            },
            false,
        )
    })
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    harness: &Harness,
) -> TestResult {
    super::questpc7_native_deaths::prepare_native_hunter(owner, admitted).await?;
    for (kind, first_command) in [("diamond", 200), ("golden", 210)] {
        let creature = format!("oteryn:creature.{kind}_servant_replica");
        let track = format!(
            "oteryn:quest-progress/quest/u11_02/forgotten_knowledge/{kind}_servant_counter"
        );
        for expected in 1..=5 {
            // The fixture sends each lethal notification twice through the real death queue.
            super::qengine_kill_stories::death(
                owner,
                admitted,
                &creature,
                first_command + u64::try_from(expected)?,
            )
            .await?;
            let count: i64 = sqlx::query_scalar(
                "SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),0)",
            )
            .bind(&track)
            .fetch_one(&harness.pool)
            .await?;
            assert_eq!(count, expected, "{kind} replica credit");
        }
    }
    Ok(())
}
