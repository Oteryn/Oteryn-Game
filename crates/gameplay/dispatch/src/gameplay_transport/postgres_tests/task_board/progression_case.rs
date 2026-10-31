//! The durable planner and existing XP owner agree after retries and level-ups.
use super::*;
use crate::durability::character_progression::{
    ExperienceAwardRequest, ExperienceCommitOutcome, ExperienceRewardOccurrence,
};
use crate::durability::quest_state::QuestCause;
use crate::durability::task_board::TaskBoardCommand;
use oteryn_gameplay_quests::quest::task_board::full::TaskBoardAction;
use oteryn_simulation_determinism::ExactI64;
use std::sync::Arc;

pub(super) async fn policy(
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

/// Advance an already receipted character through the existing XP owner. Never
/// replace its current level/experience or rewind the shared revision chain.
pub(super) async fn award_total(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    admitted: &AdmittedSession,
    total: i64,
    expected_level: u32,
    tag: u16,
) -> TestResult {
    let policy = policy(owner, harness).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "XP fixture fence")?
        .ok_or("XP fixture fence")?;
    let before = owner
        .root
        .read_character_progression(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("initialized progression")?;
    let amount = total
        .checked_sub(before.total_experience.get())
        .filter(|amount| *amount > 0)
        .ok_or("XP fixture must advance, never rewind, the current total")?;
    let occurrence = ExperienceRewardOccurrence::from_bytes(death_occurrence(tag)).map_err(debug)?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let outcome = slot
        .commit_experience(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            ExperienceAwardRequest {
                occurrence,
                amount: ExactI64::new(amount),
                context: policy.context.clone(),
                policy_revision: policy.policy_revision.clone(),
                reward_revision: policy.reward_revision.clone(),
                policy,
            },
            None,
        )
        .await
        .map_err(debug)?;
    let ExperienceCommitOutcome::Committed(committed) = outcome else {
        return Err(format!("XP fixture requires a new accepted award: {outcome:?}").into());
    };
    assert_eq!(committed.experience_before, before.total_experience);
    assert_eq!(committed.experience_awarded.get(), amount);
    assert_eq!(committed.experience_after.get(), total);
    assert_eq!(committed.level_after, expected_level);
    drop(slot);
    assert_eq!(experience(harness).await?, total);
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_xp_receipts WHERE character_id=encode($1,'hex')::uuid AND reward_occurrence_id=encode($2,'hex')::uuid")
        .bind(id(41).as_slice()).bind(occurrence.as_bytes().as_slice()).fetch_one(&harness.pool).await?,1);
    owner.refresh_quest_session(admitted.game_session_id, true).await;
    Ok(())
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

/// Use the historical seed for the no-vocation refusal, then choose through the
/// existing build owner so the next quest mutation sees a valid receipt-chain tip.
pub(super) async fn fixture_vocation(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    vocation: &str,
) -> TestResult {
    use crate::durability::character_build::{
        BuildCause, BuildChangeRequest, BuildOccurrence, NO_VOCATION, convert_vocation,
    };
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "vocation fixture fence")?
        .ok_or("vocation fixture fence")?;
    let before = owner
        .root
        .read_character_build_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(
        before.vocation(),
        NO_VOCATION,
        "the existing no-vocation build must remain intact until its accepted choice"
    );
    if vocation == NO_VOCATION {
        return Ok(());
    }
    // The same qualified profile/pin used by the existing PARTY-XP fixture.
    let formula = crate::spell::mana_training::CompiledTrainingFormula::from_profile(
        include_bytes!("../../../../../../../tools/content-schema/native-gameplay/build-training.json"),
        "content-1",
    )
    .map_err(debug)?;
    let after = convert_vocation(&formula, &before, vocation).map_err(debug)?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    slot.commit_build(
        owner.root,
        owner.character,
        owner.holder,
        fence,
        BuildChangeRequest {
            occurrence: BuildOccurrence::from_bytes(death_occurrence(64_999)).map_err(debug)?,
            cause: BuildCause::VocationChoice,
            before,
            after: after.clone(),
            pruned_stance: None,
        },
        &formula,
        None,
    )
    .await
    .map_err(debug)?;
    assert_eq!(
        owner
            .root
            .read_character_build_state(owner.character, fence.character_id)
            .await
            .map_err(debug)?,
        after
    );
    Ok(())
}

pub(super) async fn pending_experience_case(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    admitted: &AdmittedSession,
) -> TestResult {
    // Crystal ioweeklytasks.cpp:989-1029: 25*L^2 - 75*L + 100.
    // These independent goldens for levels 60 and 61 remain below the
    // Beginner cap of 200_000; expected values do not call weekly_experience.
    const FIRST_REWARD: i64 = 85_600;
    const NEXT_REWARD: i64 = 88_550;
    let session = admitted.game_session_id;
    owner.refresh_quest_session(session, true).await;
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_quest_xp_obligations WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&harness.pool).await?,0);
    let policy = policy(owner, harness).await?;
    let next = policy
        .thresholds
        .iter()
        .find(|row| row.level == 61)
        .ok_or("level61 threshold")?
        .minimum_experience
        .get();
    // The earlier Wheel case legitimately reached level 51. Advance to a known later
    // threshold instead of rewriting the level/XP tip underneath its receipts.
    let initial = next.checked_sub(FIRST_REWARD).ok_or("level60 starting XP")?;
    award_total(owner, harness, admitted, initial, 60, 64_997).await?;
    let before_paid = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_xp_receipts")
        .fetch_one(&harness.pool).await?;
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
    assert_eq!(first.view.level, 61);
    assert_eq!(
        first
            .transition
            .as_ref()
            .and_then(|t| t.experience.as_ref())
            .map(|xp| xp.amount),
        Some(FIRST_REWARD + NEXT_REWARD)
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
    assert_eq!(second.view.level, 62);
    assert_eq!(
        second
            .transition
            .as_ref()
            .and_then(|t| t.experience.as_ref())
            .map(|xp| xp.amount),
        Some(NEXT_REWARD)
    );
    let obligations = [
        first.transition.as_ref().and_then(|transition| transition.experience).ok_or("first XP obligation")?,
        second.transition.as_ref().and_then(|transition| transition.experience).ok_or("second XP obligation")?,
    ];
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
    assert_eq!(experience(harness).await?, initial + FIRST_REWARD + 2 * NEXT_REWARD);
    owner.refresh_quest_session(session, true).await;
    assert_eq!(experience(harness).await?, initial + FIRST_REWARD + 2 * NEXT_REWARD);
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_xp_receipts")
        .fetch_one(&harness.pool).await?,before_paid+2);
    for obligation in obligations {
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_xp_receipts WHERE character_id=encode($1,'hex')::uuid AND reward_occurrence_id=encode($2,'hex')::uuid")
            .bind(id(41).as_slice()).bind(obligation.occurrence.as_bytes().as_slice()).fetch_one(&harness.pool).await?,1);
    }
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_quest_xp_obligations WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&harness.pool).await?,0);
    println!(
        "<TASK_BOARD> POSTGRES_CASE_COMPLETED: sequential_completion_and_pending_xp_reload_once"
    );
    Ok(())
}
