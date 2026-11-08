//! M47 drives the fenced world writer, preserving the command receipt on replay.
use super::TestResult;
use super::fixture::{Harness, debug};
use super::quest_world_callbacks::scenario;
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence,
};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::quest_world_callback::QuestWorldContext;
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::connection::AdmittedSession;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
    barbarian_mead as mead, world_callbacks,
};
use std::sync::Arc;

#[test]
fn m47_postgres_mead_roll_is_fenced_and_exactly_replayed() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("m47-mead", || {
        scenario(
            "barbarian-test-mead",
            Position {
                x: 32201,
                y: 31154,
                floor: -7,
            },
            false,
        )
    });
    println!(
        "M47_POSTGRES_EXECUTED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    assert!(started.elapsed().as_secs_f64() > 0.01);
    result
}

pub(super) async fn exercise(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    fence: CurrentCharacterGameplayFence,
    callback: &Callback,
) -> TestResult {
    let base = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
    let tracks = mead::NATIVE_TRACKS
        .into_iter()
        .map(|k| base.track(k).cloned().ok_or("mead track"))
        .collect::<Result<Vec<_>, _>>()?;
    let setup = QuestTransition {
        key: "oteryn:quest-transition/m47-fixture-round".into(),
        quest: mead::QUEST.into(),
        // Sven writes questline, mission and total; the streak is absent on first entry.
        effects: [
            (mead::TRACKS[0], 2),
            (mead::TRACKS[1], 2),
            (mead::TRACKS[3], 0),
        ]
        .into_iter()
        .map(|(track, value)| QuestEffect {
            track: track.into(),
            from: QuestComparison::Between(-1, i64::MAX),
            effect: QuestEffectKind::Set(value),
        })
        .collect(),
        completes: false,
        experience: None,
        world_change: None,
    };
    let seed = |key: &str, values: [i64; 4]| QuestTransition {
        key: key.into(),
        effects: mead::TRACKS
            .into_iter()
            .zip(values)
            .map(|(track, value)| QuestEffect {
                track: track.into(),
                from: QuestComparison::Between(-1, i64::MAX),
                effect: QuestEffectKind::Set(value),
            })
            .collect(),
        ..setup.clone()
    };
    let passed = seed("oteryn:quest-transition/m47-fixture-passed", [3, 3, 9, 17]);
    let exhausted = seed(
        "oteryn:quest-transition/m47-fixture-exhausted",
        [2, 2, 4, 21],
    );
    let catalogue = Arc::new(
        QuestStateCatalogue::new(
            base.content_revision(),
            tracks,
            vec![
                setup.clone(),
                passed.clone(),
                exhausted.clone(),
                world_callbacks::primary_transition(callback)?,
            ],
        )
        .map_err(debug)?,
    );
    let request = |key: &str, n| -> TestResult<QuestTransitionRequest> {
        Ok(QuestTransitionRequest {
            transition_key: key.into(),
            cause: QuestCause::Use(CommandRef::new(
                admitted.game_session_id,
                CommandId::new(n).map_err(debug)?,
            )),
        })
    };
    let (context, permit) = {
        let runtime = owner.runtime.lock().await;
        let actor = admitted.runtime_actor.ok_or("actor")?;
        let context = QuestWorldContext {
            map_revision: crate::map::overlay::map_revision(
                owner.bundle_map().ok_or("map")?.overlay.base(),
            ),
            content_revision: format!(
                "sha256:{}",
                runtime
                    .content_pin()
                    .server_artifact_digest()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            ),
            placement_context: runtime.content_pin().frame_binding_digest().to_vec(),
            reset_epoch: owner.reset_epoch,
            actor_placement: actor.placement_identity(),
            movement_revision: runtime
                .read_actor_position(actor)
                .map_err(debug)?
                .revision(),
        };
        drop(runtime);
        (context, owner.spell_lane_permit().await.ok_or("permit")?)
    };
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let mut start = request(&setup.key, 7000)?;
    start.cause = QuestCause::Command(CommandRef::new(
        admitted.game_session_id,
        CommandId::new(7000).map_err(debug)?,
    ));
    assert!(matches!(
        slot.commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            start,
            catalogue.clone()
        )
        .await?,
        QuestTransitionOutcome::Committed(_)
    ));
    let command = request(mead::KEY, 7001)?;
    let generic = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            command.clone(),
            catalogue.clone(),
        )
        .await;
    assert!(
        matches!(generic, Err(CharacterProgressionError::InvalidInput)),
        "{generic:?}"
    );
    let mut stale = fence;
    stale.character_lease_generation += 1;
    let rejected = slot
        .commit_world_callback(
            owner.root,
            owner.character,
            owner.holder,
            stale,
            command.clone(),
            catalogue.clone(),
            &permit,
            context.clone(),
            None,
        )
        .await;
    assert!(
        matches!(rejected, Err(CharacterProgressionError::AuthorityRejected)),
        "{rejected:?}"
    );
    let mut wrong_world = context.clone();
    wrong_world.reset_epoch += 1;
    let rejected = slot
        .commit_world_callback(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            command.clone(),
            catalogue.clone(),
            &permit,
            wrong_world,
            None,
        )
        .await?;
    assert!(
        matches!(rejected, QuestTransitionOutcome::Refused(_)),
        "{rejected:?}"
    );
    let first = slot
        .commit_world_callback(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            command.clone(),
            catalogue.clone(),
            &permit,
            context.clone(),
            None,
        )
        .await?;
    let QuestTransitionOutcome::Committed(receipt) = first else {
        return Err(format!("M47 did not commit: {first:?}").into());
    };
    assert_eq!(receipt.changes.len(), 5);
    let streak = receipt
        .changes
        .iter()
        .find(|c| c.track == mead::TRACKS[2])
        .ok_or("streak receipt")?;
    assert_eq!((streak.before, streak.after), (-1, 0));
    assert!(
        (1..=5).contains(
            &receipt
                .changes
                .iter()
                .find(|c| c.track == mead::DRAW)
                .ok_or("receipt draw")?
                .after
        )
    );
    assert_eq!(
        receipt
            .changes
            .iter()
            .find(|c| c.track == mead::TRACKS[3])
            .ok_or("total")?
            .after,
        1
    );
    for _ in 0..3 {
        let replay = slot
            .commit_world_callback(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                command.clone(),
                catalogue.clone(),
                &permit,
                context.clone(),
                None,
            )
            .await?;
        assert_eq!(
            replay,
            QuestTransitionOutcome::AlreadyCommitted(receipt.clone())
        );
    }
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(mead::KEY)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(count, 1);
    let stored: Vec<(String,i64)> = sqlx::query_as("SELECT track_key,value FROM game_character_quest_tracks WHERE track_key=ANY($1::text[]) ORDER BY track_key").bind(mead::NATIVE_TRACKS.as_slice()).fetch_all(&h.pool).await?;
    assert_eq!(stored.len(), 5);
    for (track, value) in stored {
        assert_eq!(
            receipt
                .changes
                .iter()
                .find(|c| c.track == track)
                .ok_or("stored receipt track")?
                .after,
            value
        );
    }
    for (seed, n, line, message) in [
        (
            &passed,
            7100,
            3,
            "You already passed the test, no need to torture yourself anymore.",
        ),
        (&exhausted, 7200, 1, "Ask Sven for another round."),
    ] {
        let setup_request = QuestTransitionRequest {
            transition_key: seed.key.clone(),
            cause: QuestCause::Command(CommandRef::new(
                admitted.game_session_id,
                CommandId::new(n).map_err(debug)?,
            )),
        };
        assert!(matches!(
            slot.commit_quest_transition(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                setup_request,
                catalogue.clone()
            )
            .await?,
            QuestTransitionOutcome::Committed(_)
        ));
        let attempt = request(mead::KEY, n + 1)?;
        let outcome = slot
            .commit_world_callback(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                attempt.clone(),
                catalogue.clone(),
                &permit,
                context.clone(),
                None,
            )
            .await?;
        let QuestTransitionOutcome::Committed(saved) = outcome else {
            return Err(format!("M47 nonrandom outcome refused: {outcome:?}").into());
        };
        assert_eq!(
            saved
                .changes
                .iter()
                .find(|c| c.track == mead::DRAW)
                .map(|c| c.after),
            Some(0)
        );
        assert_eq!(
            saved
                .changes
                .iter()
                .find(|c| c.track == mead::TRACKS[0])
                .map(|c| c.after),
            Some(line)
        );
        assert_eq!(callback.committed_message(&saved.changes), Some(message));
        if line == 3 {
            assert!(
                saved
                    .changes
                    .iter()
                    .filter(|c| c.track != mead::DRAW)
                    .all(|c| c.before == c.after)
            );
        } else {
            assert_eq!(
                saved
                    .changes
                    .iter()
                    .find(|c| c.track == mead::TRACKS[1])
                    .map(|c| c.after),
                Some(1)
            );
            assert_eq!(
                saved
                    .changes
                    .iter()
                    .find(|c| c.track == mead::TRACKS[2])
                    .map(|c| c.after),
                Some(4)
            );
            assert_eq!(
                saved
                    .changes
                    .iter()
                    .find(|c| c.track == mead::TRACKS[3])
                    .map(|c| c.after),
                Some(21)
            );
        }
        let replay = slot
            .commit_world_callback(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                attempt,
                catalogue.clone(),
                &permit,
                context.clone(),
                None,
            )
            .await?;
        assert_eq!(replay, QuestTransitionOutcome::AlreadyCommitted(saved));
    }
    // A late replay must return its original roll without undoing the round reset.
    let before: Vec<(String,i64)> = sqlx::query_as("SELECT track_key,value FROM game_character_quest_tracks WHERE track_key=ANY($1::text[]) ORDER BY track_key").bind(mead::NATIVE_TRACKS.as_slice()).fetch_all(&h.pool).await?;
    let late = slot
        .commit_world_callback(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            command,
            catalogue.clone(),
            &permit,
            context,
            None,
        )
        .await?;
    assert_eq!(late, QuestTransitionOutcome::AlreadyCommitted(receipt));
    let after: Vec<(String,i64)> = sqlx::query_as("SELECT track_key,value FROM game_character_quest_tracks WHERE track_key=ANY($1::text[]) ORDER BY track_key").bind(mead::NATIVE_TRACKS.as_slice()).fetch_all(&h.pool).await?;
    assert_eq!(before, after);
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(mead::KEY)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(count, 3);
    drop(slot);
    // The condition owner receives the deterministic failed outcome from the real rule.
    let failed_values = mead::TRACKS
        .into_iter()
        .zip([2, 2, -1, 0])
        .map(|(key, value)| (key.to_owned(), value))
        .collect();
    let failed = catalogue
        .evaluate_with_roll(
            catalogue.transition(mead::KEY).ok_or("transition")?,
            &failed_values,
            0,
            Some(1),
        )
        .map_err(debug)?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let runtime = owner.runtime.lock().await;
    let mut states = owner.states.spell_states.lock().await;
    let map = owner.bundle_map().ok_or("map")?;
    let wrong_session =
        crate::foundation::GameSessionId::decode(&super::fixture::id(99)).map_err(debug)?;
    let wrong = CommandRef::new(wrong_session, CommandId::new(7300).map_err(debug)?);
    let before = states
        .get(&runtime, actor, admitted.game_session_id)
        .ok_or("player")?
        .owned_conditions()
        .clone();
    assert!(!owner.project_mead_blackout(&runtime, &mut states, actor, wrong, &failed, map));
    assert_eq!(
        states
            .get(&runtime, actor, admitted.game_session_id)
            .ok_or("player")?
            .owned_conditions(),
        &before
    );
    let command = CommandRef::new(
        admitted.game_session_id,
        CommandId::new(7300).map_err(debug)?,
    );

    assert!(
        owner.project_mead_blackout(
            &runtime,
            &mut states,
            actor,
            command,
            &failed,
            owner.bundle_map().ok_or("map")?
        ),
        "qualified failure must project its blackout"
    );
    let conditions = states
        .get(&runtime, actor, admitted.game_session_id)
        .ok_or("player")?
        .owned_conditions();
    let blackout = conditions
        .get(oteryn_gameplay_domain::domain::condition::ConflictKey::Outfit)
        .ok_or("blackout condition")?;
    assert_eq!(
        blackout.remaining_duration_ms(blackout.started_at()),
        Some(1000)
    );
    let item = blackout
        .definition()
        .source_item_appearance()
        .ok_or("Item look")?;
    assert_eq!(item.definition_key(), "oteryn:item.tibia.i111");
    assert_eq!(
        item.artifact_digest(),
        runtime.content_pin().server_artifact_digest()
    );
    let expiry = blackout
        .started_at()
        .checked_add(1_000_000)
        .ok_or("expiry")?;
    assert!(
        conditions
            .displayed_temporary_appearance_at(expiry - 1)
            .is_some()
    );
    assert!(
        conditions
            .displayed_temporary_appearance_at(expiry)
            .is_none()
    );
    Ok(())
}
