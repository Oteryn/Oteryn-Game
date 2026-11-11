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
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, barbarian_mead as mead, world_callbacks};
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

/// Historical Sven state outside the barrel operation under test. Only the four
/// source counters are arranged; the durable barrel owner alone writes DRAW.
pub(super) async fn seed_sven_state(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    values: [Option<i64>; 4],
) -> TestResult {
    use crate::durability::character_progression::{
        ExperienceAwardRequest, ExperienceCommitOutcome, ExperienceRewardOccurrence,
    };
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "Sven fixture fence")?
        .ok_or("Sven fixture fence")?;
    if fence.expected_character_revision.get() == 1 {
        // Quest history starts at revision 2. Obtain it from the real sequencer
        // and XP ledger rather than inventing a generic quest receipt or root.
        let binding = owner
            .progression_binding(admitted.game_session_id)
            .ok_or("Sven fixture progression")?;
        let outcome = owner
            .revision_sequencer
            .acquire(fence.character_id)
            .await
            .commit_experience(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                ExperienceAwardRequest {
                    occurrence: ExperienceRewardOccurrence::from_bytes(super::fixture::id(175))
                        .map_err(debug)?,
                    amount: oteryn_simulation_determinism::ExactI64::new(1),
                    context: binding.context.clone(),
                    policy_revision: binding.policy_revision.clone(),
                    reward_revision: binding.reward_revision.clone(),
                    policy: binding.policy.clone(),
                },
                None,
            )
            .await
            .map_err(debug)?;
        let ExperienceCommitOutcome::Committed(receipt) = outcome else {
            return Err("Sven fixture revision did not commit".into());
        };
        assert_eq!(receipt.original_character_revision.get(), 1);
        assert_eq!(receipt.committed_character_revision.get(), 2);
        assert_eq!(receipt.level_before, receipt.level_after);
    }
    let catalogue = owner.quest_catalogue.as_ref().ok_or("Sven catalogue")?;
    let mut tx = h.pool.begin().await?;
    let revision: String = sqlx::query_scalar(
        "SELECT character_revision::text FROM game_character_roots \
         WHERE character_id=encode($1,'hex')::uuid FOR UPDATE",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_one(&mut *tx)
    .await?;
    assert!(revision.parse::<u64>()? >= 2);
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (key, value) in mead::TRACKS.into_iter().zip(values) {
        let track = catalogue.track(key).ok_or("Sven source track")?;
        assert_eq!(track.quest, mead::QUEST);
        let Some(value) = value else {
            let stored: Option<i64> = sqlx::query_scalar(
                "SELECT value FROM game_character_quest_tracks \
                 WHERE character_id=encode($1,'hex')::uuid AND track_key=$2",
            )
            .bind(fence.character_id.as_bytes().as_slice())
            .bind(key)
            .fetch_optional(&mut *tx)
            .await?;
            assert_eq!(stored, None, "first Sven entry must leave {key} absent");
            continue;
        };
        assert!((track.min..=track.max).contains(&value));
        let changed = sqlx::query(
            "INSERT INTO game_character_quest_tracks \
             (character_id,track_key,quest_key,value,committed_character_revision) \
             SELECT character_id,$2,$3,$4,character_revision FROM game_character_roots \
             WHERE character_id=encode($1,'hex')::uuid AND character_revision=$5::text::numeric \
             ON CONFLICT(character_id,track_key) DO UPDATE \
             SET value=EXCLUDED.value,committed_character_revision=EXCLUDED.committed_character_revision \
             WHERE game_character_quest_tracks.quest_key=EXCLUDED.quest_key",
        )
        .bind(fence.character_id.as_bytes().as_slice())
        .bind(key)
        .bind(&track.quest)
        .bind(value)
        .bind(&revision)
        .execute(&mut *tx)
        .await?;
        assert_eq!(changed.rows_affected(), 1);
    }
    tx.commit().await?;
    // Historical counters share the current root revision. Reload the session
    // explicitly before its next real barrel command can consult the quest copy.
    owner
        .refresh_quest_session(admitted.game_session_id, true)
        .await;
    let sessions = owner
        .states
        .quest_sessions
        .lock()
        .map_err(|_| "Sven quest sessions")?;
    let copy = sessions
        .get(&admitted.game_session_id)
        .and_then(|session| session.copy.as_ref())
        .ok_or("Sven refreshed quest copy")?;
    for (key, value) in mead::TRACKS.into_iter().zip(values) {
        assert_eq!(copy.tracks().get(key).copied(), value);
    }
    Ok(())
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
    let catalogue = Arc::new(
        QuestStateCatalogue::new(
            base.content_revision(),
            tracks,
            vec![world_callbacks::primary_transition(callback)?],
        )
        .map_err(debug)?,
    );
    // Sven accepts the round without creating the streak on the first entry.
    seed_sven_state(h, owner, admitted, [Some(2), Some(2), None, Some(0)]).await?;
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
    drop(slot);
    for (values, n, line, message) in [
        (
            [3, 3, 9, 17],
            7100,
            3,
            "You already passed the test, no need to torture yourself anymore.",
        ),
        ([2, 2, 4, 21], 7200, 1, "Ask Sven for another round."),
    ] {
        seed_sven_state(h, owner, admitted, values.map(Some)).await?;
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
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
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
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
    let success = catalogue
        .evaluate_with_roll(
            catalogue.transition(mead::KEY).ok_or("transition")?,
            &failed_values,
            0,
            Some(5),
        )
        .map_err(debug)?;
    assert!(!owner.project_mead_blackout(&runtime, &mut states, actor, command, &success, map));
    assert_eq!(
        states
            .get(&runtime, actor, admitted.game_session_id)
            .ok_or("player")?
            .owned_conditions(),
        &before
    );
    let active = owner.active_generation.take();
    assert!(!owner.project_mead_blackout(
        &runtime,
        &mut states,
        actor,
        command,
        &failed,
        owner.bundle_map().ok_or("map")?
    ));
    assert_eq!(
        states
            .get(&runtime, actor, admitted.game_session_id)
            .ok_or("player")?
            .owned_conditions(),
        &before
    );
    owner.active_generation = active;
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
