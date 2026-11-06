//! The quest writer must retain all eleven effects in one fenced receipt.
//! Source: Crystal 9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d,
//! data-global/npc/tereban_functions.lua:175-180 (three tracks plus eight materials),
//! sha256:0269af24eef41644b6b03356d182a79f4b02b768d9500147213d0a17bb5b0b6d.
use super::*;
use oteryn_gameplay_quests::quest::{QuestRefusal, QuestTrack};

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult {
    let quest = "oteryn:quest.l2_atomic_tracks";
    let tracks: Vec<_> = (0..11)
        .map(|i| QuestTrack {
            key: format!("oteryn:quest-progress/l2-atomic/{i}"),
            quest: quest.into(),
            initial: 0,
            min: 0,
            max: 1,
        })
        .collect();
    let good = QuestTransition {
        key: "oteryn:quest-transition/l2-atomic/complete".into(),
        quest: quest.into(),
        effects: tracks
            .iter()
            .map(|t| QuestEffect {
                track: t.key.clone(),
                from: QuestComparison::Eq(0),
                effect: QuestEffectKind::Set(1),
            })
            .collect(),
        completes: true,
        experience: None,
    };
    let mut bad = good.clone();
    bad.key = "oteryn:quest-transition/l2-atomic/mismatch".into();
    bad.effects.last_mut().ok_or("eleventh effect")?.from = QuestComparison::Eq(1);
    let catalogue = Arc::new(
        QuestStateCatalogue::new("content-1", tracks, vec![good.clone(), bad.clone()])
            .map_err(debug)?,
    );
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let before: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_character_quest_receipts), (SELECT count(*) FROM game_character_quest_tracks)",
    )
    .fetch_one(&h.pool)
    .await?;
    let request = |key: &str, ordinal| -> TestResult<QuestTransitionRequest> {
        Ok(QuestTransitionRequest {
            transition_key: key.into(),
            cause: QuestCause::Command(CommandRef::new(
                admitted.game_session_id,
                CommandId::new(ordinal).map_err(debug)?,
            )),
        })
    };
    let refused = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            request(&bad.key, 9000)?,
            Arc::clone(&catalogue),
        )
        .await
        .map_err(debug)?;
    assert_eq!(
        refused,
        QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
    );
    let after: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_character_quest_receipts), (SELECT count(*) FROM game_character_quest_tracks)",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(before, after, "eleventh-track mismatch writes nothing");
    let command = request(&good.key, 9001)?;
    let committed = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            command.clone(),
            Arc::clone(&catalogue),
        )
        .await
        .map_err(debug)?;
    let QuestTransitionOutcome::Committed(receipt) = committed else {
        return Err(format!("eleven-track commit: {committed:?}").into());
    };
    assert_eq!(receipt.changes.len(), 11);
    assert!(
        receipt
            .changes
            .iter()
            .all(|c| c.before == 0 && c.after == 1)
    );
    let persisted: (i64, i32, i64) = sqlx::query_as(
        "SELECT count(*), min(cardinality(track_keys)), min(committed_character_revision - original_character_revision)::bigint FROM game_character_quest_receipts WHERE quest_key=$1",
    )
    .bind(quest)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        persisted,
        (1, 11, 1),
        "one receipt, eleven effects, one revision"
    );
    let stored: (i64, i64, i64) = sqlx::query_as(
        "SELECT count(*), min(value), max(value) FROM game_character_quest_tracks WHERE quest_key=$1",
    )
    .bind(quest)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(stored, (11, 1, 1), "all eleven stored values advance");
    let replay = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            command,
            catalogue,
        )
        .await
        .map_err(debug)?;
    assert_eq!(replay, QuestTransitionOutcome::AlreadyCommitted(receipt));
    Ok(())
}
