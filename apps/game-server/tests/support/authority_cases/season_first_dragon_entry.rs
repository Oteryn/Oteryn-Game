//! M18: committed personal trial progress gates the existing timed Boss Activity owner.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::activity_handoff::fixture_first_dragon_trials_permit;
use crate::durability::character_progression::{
    CharacterProgressionError, ExperienceAwardRequest, ExperienceCommitOutcome,
    ExperienceRewardOccurrence,
};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
};
use std::sync::Arc;
const QUEST: &str = "oteryn:quest.the_first_dragon_quest";
const TRIALS: &str = "oteryn:quest-progress/chosen-source/the_first_dragon_quest/s4";
const DEFEAT: &str = "oteryn:quest-progress/chosen-source/the_first_dragon_quest/s5";

#[test]
fn m18_first_dragon_trial_entry_guard_postgres_committed_progress_and_replay() -> fixture::TestResult
{
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h =
                        fixture::Harness::create(admin, "season_first_dragon_entry", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "M18_PG_EXECUTED first_dragon_entry_guard duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "M18 PostgreSQL thread failed")?
        .map_err(Into::into)
}
async fn permitted(h: &fixture::Harness) -> fixture::TestResult<bool> {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SELECT character_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(fixture::id(fixture::CHARACTER).as_slice()).fetch_one(&mut *tx).await?;
    let answer =
        fixture_first_dragon_trials_permit(&mut tx, fixture::fence(1)?.character_id).await?;
    tx.rollback().await?;
    Ok(answer)
}
async fn exercise(h: &fixture::Harness) -> fixture::TestResult {
    let canonical = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(fixture::debug)?;
    let trials = canonical
        .catalogue()
        .track(TRIALS)
        .cloned()
        .ok_or("canonical trial track")?;
    let defeat = canonical
        .catalogue()
        .track(DEFEAT)
        .cloned()
        .ok_or("canonical defeat track")?;
    assert_eq!(trials.quest, QUEST);
    assert_eq!(defeat.quest, QUEST);
    assert!((trials.min..=trials.max).contains(&1) && (trials.min..=trials.max).contains(&2));
    assert!((defeat.min..=defeat.max).contains(&1));
    let effect = |track: &str, from, kind| QuestEffect {
        track: track.into(),
        from,
        effect: kind,
    };
    let transition = |name: &str, effects| QuestTransition {
        key: format!("oteryn:quest-transition/m18-proof/{name}"),
        quest: QUEST.into(),
        effects,
        completes: false,
        experience: None,
        world_change: None,
    };
    let catalogue = Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![trials, defeat],
            vec![
                transition(
                    "one",
                    vec![effect(
                        TRIALS,
                        QuestComparison::Eq(0),
                        QuestEffectKind::Set(1),
                    )],
                ),
                transition(
                    "two",
                    vec![effect(
                        TRIALS,
                        QuestComparison::Eq(1),
                        QuestEffectKind::Set(2),
                    )],
                ),
                transition(
                    "defeat",
                    vec![
                        effect(TRIALS, QuestComparison::Eq(2), QuestEffectKind::Add(0)),
                        effect(DEFEAT, QuestComparison::Eq(0), QuestEffectKind::Set(1)),
                    ],
                ),
            ],
        )
        .map_err(fixture::debug)?,
    );
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let session = GameSessionId::decode(&fixture::id(fixture::SESSION)).map_err(fixture::debug)?;
    let request = |name: &str, id| -> fixture::TestResult<QuestTransitionRequest> {
        Ok(QuestTransitionRequest {
            transition_key: format!("oteryn:quest-transition/m18-proof/{name}"),
            cause: QuestCause::Command(CommandRef::new(
                session,
                CommandId::new(id).map_err(fixture::debug)?,
            )),
        })
    };
    assert!(!permitted(h).await?, "missing progress must refuse entry");
    for (name, revision, id, expected) in [("one", 1, 22101, false), ("two", 2, 22102, true)] {
        let outcome = h
            .root
            .commit_character_quest_transition(
                &authority,
                &h.node,
                fixture::fence(revision)?,
                request(name, id)?,
                catalogue.clone(),
            )
            .await?;
        assert!(
            matches!(outcome, QuestTransitionOutcome::Committed(_)),
            "{outcome:?}"
        );
        assert_eq!(permitted(h).await?, expected, "{name}");
        let replay = h
            .root
            .commit_character_quest_transition(
                &authority,
                &h.node,
                fixture::fence(revision)?,
                request(name, id)?,
                catalogue.clone(),
            )
            .await?;
        assert!(matches!(
            replay,
            QuestTransitionOutcome::AlreadyCommitted(_)
        ));
        assert_eq!(permitted(h).await?, expected, "replay of {name}");
    }
    let before = h
        .root
        .read_character_quest_state(&authority, fixture::fence(3)?.character_id)
        .await?;
    let receipts = h.count("game_character_quest_receipts").await?;
    assert!(
        matches!(
            h.root
                .commit_character_quest_transition(
                    &authority,
                    &h.node,
                    fixture::fence(3)?,
                    request("defeat", 22103)?,
                    catalogue.clone(),
                )
                .await,
            Err(CharacterProgressionError::InvalidInput)
        ),
        "a generic command cannot stand in for the selected boss death"
    );
    assert_eq!(h.root_revision().await?, "3");
    assert_eq!(h.count("game_character_quest_receipts").await?, receipts);
    assert_eq!(
        h.root
            .read_character_quest_state(&authority, fixture::fence(3)?.character_id)
            .await?
            .tracks(),
        before.tracks()
    );
    assert!(permitted(h).await?);

    // This case qualifies the persisted SQL permit, not the native death writer.
    // The actual carrier death, Instance handoff and duplicate drain are covered by
    // postgres_tests/season_first_dragon.rs::m18_selected_first_dragon_native_victory_completes_once_postgres.
    // Arrange that historical prerequisite at a real revision earned by the XP owner.
    use crate::domain::progression::{FiniteProgressionPolicy, LevelThreshold};
    use oteryn_simulation_determinism::{ExactI64, RoundingMode};
    let progression = h
        .root
        .read_character_progression(&authority, fixture::fence(3)?.character_id)
        .await?
        .ok_or("fixture progression")?;
    assert_eq!(progression.context, fixture::context());
    assert_eq!(progression.level, 50);
    let xp = ExperienceAwardRequest {
        occurrence: ExperienceRewardOccurrence::from_bytes(fixture::id(221))
            .map_err(fixture::debug)?,
        amount: ExactI64::new(1),
        context: progression.context.clone(),
        policy_revision: progression.policy_revision.clone(),
        reward_revision: progression.reward_revision.clone(),
        policy: FiniteProgressionPolicy {
            context: progression.context.clone(),
            policy_revision: progression.policy_revision.clone(),
            reward_revision: progression.reward_revision.clone(),
            death_policy_revision: "death-1".into(),
            declared_difference_revision: "declaration-1".into(),
            thresholds: [
                LevelThreshold {
                    level: 50,
                    minimum_experience: ExactI64::new(1000),
                },
                LevelThreshold {
                    level: 51,
                    minimum_experience: ExactI64::new(1100),
                },
            ],
            terminal_exclusive_experience: ExactI64::new(1200),
            death_loss_numerator: 1,
            death_loss_denominator: 10,
            death_loss_rounding: RoundingMode::Floor,
        },
    };
    let outcome = h
        .root
        .commit_character_experience(&authority, &h.node, fixture::fence(3)?, xp.clone())
        .await?;
    let ExperienceCommitOutcome::Committed(receipt) = outcome else {
        return Err("historical prerequisite revision".into());
    };
    assert_eq!(receipt.original_character_revision.get(), 3);
    assert_eq!(receipt.committed_character_revision.get(), 4);
    assert_eq!(receipt.level_after, receipt.level_before);
    assert_eq!(receipt.experience_awarded, ExactI64::new(1));
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let inserted = sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,$3,1,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid AND character_revision=$4::text::numeric")
        .bind(fixture::id(fixture::CHARACTER).as_slice()).bind(DEFEAT).bind(QUEST)
        .bind(receipt.committed_character_revision.get().to_string()).execute(&mut *tx).await?;
    assert_eq!(inserted.rows_affected(), 1);
    tx.commit().await?;
    assert!(
        !permitted(h).await?,
        "the stored defeated state must refuse re-entry"
    );
    let replay = h
        .root
        .commit_character_experience(&authority, &h.node, fixture::fence(3)?, xp)
        .await?;
    assert!(
        matches!(replay, ExperienceCommitOutcome::AlreadyCommitted(ref old) if old == &receipt)
    );
    assert!(
        !permitted(h).await?,
        "XP receipt replay must preserve the persisted refusal"
    );
    assert_eq!(h.root_revision().await?, "4");
    Ok(())
}
