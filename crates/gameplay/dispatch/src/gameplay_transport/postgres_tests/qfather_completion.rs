//! Tereban pays the wiki-packet completion reward through NPC dispatch.
use super::fixture::{Harness, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, classic_quests};
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn qfather_tereban_grants_eight_thousand_gold_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("qfather-completion", || {
        super::quest_world_callbacks::scenario(
            "qfather-completion",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED qfather_tereban_grants_eight_thousand_gold_once duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let [tereban] = classic_quests::install(owner, admitted, ["oteryn:npc.tereban"]).await?;
    classic_quests::talk(owner, admitted, tereban, 2501, "hi").await?;
    classic_quests::talk(owner, admitted, tereban, 2502, "mission").await?;
    classic_quests::talk(owner, admitted, tereban, 2503, "yes").await?;
    // Start the completion case with all eight material hand-ins recorded.
    let changed = sqlx::query(
        "UPDATE game_character_quest_tracks SET value=CASE
             WHEN track_key='oteryn:quest-progress/quest/u8_6/afathers_burden/progress' THEN 8 ELSE 2 END
         WHERE character_id=encode($1,'hex')::uuid
           AND quest_key='oteryn:quest.a_father_s_burden_quest'
           AND track_key NOT LIKE '%/status' AND track_key NOT LIKE '%/quest_log'",
    )
    .bind(id(41).as_slice())
    .execute(&h.pool)
    .await?;
    assert_eq!(changed.rows_affected(), 9);
    owner.admit_quest_session(admitted).await;
    let (initial_xp, initial_capes, initial_gold) = rewards(&h.pool).await?;
    for command in [2800, 2801] {
        classic_quests::talk(owner, admitted, tereban, command, "mission").await?;
        assert_eq!(
            rewards(&h.pool).await?,
            (initial_xp + 8000, initial_capes + 1, initial_gold + 8000)
        );
    }
    Ok(())
}

async fn rewards(pool: &sqlx::PgPool) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as(
        "SELECT s.total_experience,
                (SELECT count(*) FROM game_item_instances WHERE lifecycle=1
                   AND definition_production_key='oteryn:item.tibia.i11701'),
                (SELECT coalesce(sum(i.quantity * 100), 0)::bigint
                 FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id)
                 WHERE e.character_id=s.character_id AND i.lifecycle=1
                   AND i.definition_production_key='oteryn:item.tibia.i3035')
         FROM game_character_progression_state s WHERE s.character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .fetch_one(pool)
    .await?)
}
