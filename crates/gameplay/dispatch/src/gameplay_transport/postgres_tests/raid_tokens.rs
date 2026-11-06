//! Jorge's actual NPC talk route, item custody and repeat exchange receipts.
use super::fixture::{Harness, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};

#[test]
fn raid_tokens_jorge_repeatable_exchange_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("hunting-for-tokens", || {
        super::quest_world_callbacks::scenario(
            "hunting-for-tokens",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    println!(
        "POSTGRES_CASE_COMPLETED raid_tokens_jorge_repeatable_exchange_and_replay duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let [jorge] = super::classic_quests::install(owner, admitted, ["oteryn:npc.jorge"]).await?;
    super::classic_quests::talk(owner, admitted, jorge, 10000, "hi").await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: Position {
                x: 33293,
                y: 31677,
                floor: -15,
            },
            item: "oteryn:item.tibia.i19083".into(),
            count: 20,
        },
        400,
        true,
    )
    .await?;
    let before_experience: i64 = sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    super::classic_quests::talk(owner, admitted, jorge, 10001, "souvenir").await?;
    super::classic_quests::talk(owner, admitted, jorge, 10002, "abacus").await?;
    let values: (i64, i64) = sqlx::query_as("SELECT \
        coalesce((SELECT sum(quantity)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i19083'),0), \
        (SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i19151')")
        .fetch_one(&h.pool).await?;
    assert_eq!(
        values,
        (0, 1),
        "one exact20token debit and one fresh replica"
    );
    let experience: i64 = sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        experience, before_experience,
        "an item exchange grants no quest XP"
    );
    Ok(())
}
