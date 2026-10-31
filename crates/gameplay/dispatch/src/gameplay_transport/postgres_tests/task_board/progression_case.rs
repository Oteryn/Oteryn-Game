//! The durable planner and existing XP owner agree after retries and level-ups.
use super::*;
use crate::durability::character_progression::ExperienceRewardOccurrence;
use crate::durability::quest_state::QuestCause;
use crate::durability::task_board::TaskBoardCommand;
use oteryn_gameplay_quests::quest::task_board::full::{TaskBoardAction, weekly_experience};
use std::sync::Arc;

async fn policy(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
) -> TestResult<
    crate::domain::progression::FiniteProgressionPolicy<
        String,
        { crate::content::character_progression_content::CHARACTER_EXPERIENCE_TABLE_LEVELS },
    >,
> {
    let (profile,ruleset,content): (String,String,String) = sqlx::query_as("SELECT profile_revision,ruleset_revision,content_revision FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&harness.pool).await?;
    owner
        .task_board_catalogue()
        .ok_or("task catalogue")?
        .progression
        .as_ref()
        .ok_or("admitted progression")?
        .policy_for(&profile, &ruleset, &content)
        .map_err(debug)
}

pub(super) async fn fixture_admitted_level(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    level: u32,
) -> TestResult {
    let policy = policy(owner, harness).await?;
    let threshold = policy
        .thresholds
        .iter()
        .find(|row| row.level == level)
        .ok_or("admitted level threshold")?;
    fixture_level(
        harness,
        i64::from(level),
        threshold.minimum_experience.get(),
    )
    .await
}

pub(super) async fn fixture_vocation(harness: &Harness, vocation: &str) -> TestResult {
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_build_state(character_id,vocation,magic_level,mana_spent,fist_level,fist_tries,club_level,club_tries,sword_level,sword_tries,axe_level,axe_tries,distance_level,distance_tries,shielding_level,shielding_tries,fishing_level,fishing_tries,committed_character_revision,last_build_occurrence_id) SELECT character_id,$2,0,0,10,0,10,0,10,0,10,0,10,0,10,0,10,0,character_revision,game_character_uuid_v7() FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid ON CONFLICT(character_id) DO UPDATE SET vocation=EXCLUDED.vocation")
        .bind(id(41).as_slice()).bind(vocation).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

pub(super) async fn pending_experience_case(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    admitted: &AdmittedSession,
) -> TestResult {
    let session = admitted.game_session_id;
    owner.refresh_quest_session(session, true).await;
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_quest_xp_obligations WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&harness.pool).await?,0);
    let policy = policy(owner, harness).await?;
    let next = policy
        .thresholds
        .iter()
        .find(|row| row.level == 11)
        .ok_or("level11 threshold")?
        .minimum_experience
        .get();
    let first_reward = weekly_experience(10, 0).map_err(debug)?;
    let initial = next - i64::try_from(first_reward)?;
    fixture_level(harness, 10, initial).await?;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fence read")?
        .ok_or("current fence")?;
    let mut data = fixture_board_data(harness).await?;
    let weekly = data.weekly.as_mut().ok_or("weekly board")?;
    weekly.period = sqlx::query_scalar("SELECT game_task_board_period(encode($1,'hex')::uuid)")
        .bind(id(42).as_slice())
        .fetch_one(&harness.pool)
        .await?;
    weekly.finished = false;
    weekly.difficulty = 0;
    weekly.any_kills = weekly.any_required - 1;
    for (index, task) in weekly.kills.iter_mut().enumerate() {
        task.current = task.required - u32::from(index < 2);
    }
    let first_race = weekly.kills[0].race.clone();
    let second_race = weekly.kills[1].race.clone();
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE game_character_task_boards SET body=$2 WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .bind(serde_json::to_value(&data)?)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    let catalogue = Arc::new(owner.task_board_catalogue().ok_or("task catalogue")?);
    let request = |tag, race| -> TestResult<TaskBoardCommand> {
        Ok(TaskBoardCommand {
            cause: QuestCause::CreatureDeath(
                ExperienceRewardOccurrence::from_bytes(death_occurrence(tag)).map_err(debug)?,
            ),
            action: TaskBoardAction::CreditKill { race },
            payload_binding: Vec::new(),
            fee_change: None,
        })
    };
    let first_request = request(65_001, first_race)?;
    let second_request = request(65_002, second_race)?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let first = slot
        .commit_task_board(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            first_request.clone(),
            Arc::clone(&catalogue),
            None,
        )
        .await
        .map_err(debug)?;
    assert_eq!(first.view.level, 11);
    assert_eq!(
        first
            .transition
            .as_ref()
            .and_then(|t| t.experience.as_ref())
            .map(|xp| xp.amount),
        Some(4_150)
    );
    assert_eq!(
        experience(harness).await?,
        initial,
        "the board creates obligations; the XP owner has not paid them yet"
    );
    let second = slot
        .commit_task_board(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            second_request.clone(),
            Arc::clone(&catalogue),
            None,
        )
        .await
        .map_err(debug)?;
    assert_eq!(second.view.level, 12);
    assert_eq!(
        second
            .transition
            .as_ref()
            .and_then(|t| t.experience.as_ref())
            .map(|xp| xp.amount),
        Some(2_300)
    );
    for (request, expected) in [(first_request, first.view), (second_request, second.view)] {
        let replay = slot
            .commit_task_board(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                request,
                Arc::clone(&catalogue),
                None,
            )
            .await
            .map_err(debug)?;
        assert!(replay.replayed);
        assert_eq!(replay.view, expected);
    }
    drop(slot);
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_quest_xp_obligations WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&harness.pool).await?,2);
    // A forced reload uses the same path as admission/reconnect and pays the two
    // original obligations. Reloading it again cannot create another award.
    owner.refresh_quest_session(session, true).await;
    assert_eq!(experience(harness).await?, initial + 4_150 + 2_300);
    owner.refresh_quest_session(session, true).await;
    assert_eq!(experience(harness).await?, initial + 4_150 + 2_300);
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_quest_xp_obligations WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&harness.pool).await?,0);
    println!(
        "<TASK_BOARD> POSTGRES_CASE_COMPLETED: sequential_completion_and_pending_xp_reload_once"
    );
    Ok(())
}
