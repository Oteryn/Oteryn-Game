// QUEST-XP-1 cases (migration 0069, QUEST-GATE-0 §5.5) on the QUEST-STATE-1 fixtures: Character
// 41 at revision one (level 50, 1000 experience, policy `policy-1`, reward `reward-1`) live on
// session 50. The writer cases run the quest transition and its XP award through the revision
// slot or directly; the guard cases issue the exact SQL a writer must, so every 0069 guard branch
// is reached independently of the writers.

use std::sync::Arc;

use super::{
    QUEST, STAGE, advance, award, command, debug, expect_committed, fence, hex, id, run, uuid,
};
use crate::bestiary_postgres_harness::{CHARACTER, Harness, TestResult, context};
use crate::domain::CharacterId;
use crate::domain::progression::{FiniteProgressionPolicy, LevelThreshold};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_progression::{
    CharacterProgressionError, ExperienceCommitOutcome, ExperienceRewardOccurrence,
};
use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
use crate::durability::quest_state::quest::{
    QUESTGATE0_RL_10, QuestComparison, QuestEffect, QuestEffectKind, QuestRefusal,
    QuestStateCatalogue, QuestTrack, QuestTransition,
};
use crate::durability::quest_state::{
    CommittedQuestTransition, QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
    QuestXpObligation, request_pending_quest_experience,
};
use oteryn_simulation_determinism::{ExactI64, RoundingMode};

const XP_ROW: &str = "quest XP obligation transition is not allowed";
const XP_PROVEN: &str = "quest XP obligation must commit with its quest receipt";
const XP_AWARDED: &str = "quest XP obligation occurrence is already awarded";
const XP_CONSUME: &str = "quest XP award must consume its obligation";
const XP_CAPACITY: &str = "quest XP obligations exceed QUESTGATE0-RL-10";

fn transition_key(name: &str) -> String {
    format!("oteryn:quest-transition/fixture.xp.{name}")
}

/// The rats quest with XP-bearing transitions: `begin` (50), `again` (10, from any stage),
/// `huge` (past the policy's terminal experience) and `plain` (none).
fn catalogue() -> TestResult<Arc<QuestStateCatalogue>> {
    let transition = |name: &str, from, experience| QuestTransition {
        key: transition_key(name),
        quest: QUEST.into(),
        effects: vec![QuestEffect {
            track: STAGE.into(),
            from,
            effect: QuestEffectKind::Set(1),
        }],
        completes: false,
        experience,
    };
    Ok(Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![QuestTrack {
                key: STAGE.into(),
                quest: QUEST.into(),
                initial: -1,
                min: -1,
                max: 10,
            }],
            vec![
                transition("begin", QuestComparison::Eq(-1), Some(50)),
                transition("again", QuestComparison::Any, Some(10)),
                transition("huge", QuestComparison::Any, Some(300)),
                transition("plain", QuestComparison::Any, None),
            ],
        )
        .map_err(debug)?,
    ))
}

/// The active progression policy (`reward_revision` `reward`), as the Bestiary cases.
fn policy(reward: &str) -> FiniteProgressionPolicy<String, 2> {
    FiniteProgressionPolicy {
        context: context(),
        policy_revision: "policy-1".into(),
        reward_revision: reward.into(),
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
    }
}

fn request(name: &str, command_id: u64) -> TestResult<QuestTransitionRequest> {
    Ok(QuestTransitionRequest {
        transition_key: transition_key(name),
        cause: QuestCause::Command(command(command_id)?),
    })
}

fn character() -> TestResult<CharacterId> {
    CharacterId::from_bytes(id(CHARACTER)).map_err(|error| debug(error).into())
}

async fn transition(
    harness: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    revision: u64,
    request: QuestTransitionRequest,
) -> TestResult<QuestTransitionOutcome> {
    harness
        .root
        .commit_character_quest_transition(
            authority,
            &harness.node,
            fence(revision)?,
            request,
            catalogue()?,
        )
        .await
        .map_err(|error| debug(error).into())
}

fn committed(outcome: QuestTransitionOutcome) -> TestResult<CommittedQuestTransition> {
    match outcome {
        QuestTransitionOutcome::Committed(receipt) => Ok(receipt),
        other => Err(format!("expected a commit, got {other:?}").into()),
    }
}

/// The Character's pending quest XP obligations, as (occurrence hex, amount, pin).
async fn obligations(pool: &sqlx::PgPool) -> TestResult<Vec<(String, i64, String)>> {
    Ok(sqlx::query_as(
        "SELECT encode(uuid_send(reward_occurrence_id), 'hex'), amount, pinned_content_revision \
           FROM game_character_quest_xp_obligations ORDER BY created_at, reward_occurrence_id",
    )
    .fetch_all(pool)
    .await?)
}

/// The revision chain, the experience and every quest XP obligation, as one comparable value.
async fn snapshot(pool: &sqlx::PgPool) -> TestResult<String> {
    Ok(sqlx::query_scalar(
        "SELECT concat_ws('|', \
           (SELECT string_agg(concat_ws(':', character_revision, total_experience), ',') \
              FROM game_character_progression_state), \
           (SELECT count(*) FROM game_character_xp_receipts), \
           (SELECT count(*) FROM game_character_quest_receipts), \
           (SELECT string_agg(concat_ws(':', reward_occurrence_id, amount), ',' ORDER BY 1) \
              FROM game_character_quest_xp_obligations))",
    )
    .fetch_one(pool)
    .await?)
}

fn obligation_of(receipt: &CommittedQuestTransition) -> TestResult<QuestXpObligation> {
    receipt
        .experience
        .ok_or_else(|| "the XP-bearing transition wrote no obligation".into())
}

fn occurrence_hex(obligation: &QuestXpObligation) -> String {
    hex(obligation.occurrence.as_bytes())
}

#[test]
fn a_quest_transition_and_its_xp_award_commit_in_one_slot() -> TestResult {
    run("quest_xp_slot", async |harness| {
        let pool = &harness.pool;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let sequencer = CharacterRevisionSequencer::new();
        let mut slot = sequencer.acquire(character()?).await;
        let begin = request("begin", 1)?;
        let award = slot
            .commit_quest_transition_with_experience(
                &harness.root,
                &authority,
                &harness.node,
                fence(1)?,
                begin.clone(),
                catalogue()?,
                &policy("reward-1"),
            )
            .await
            .map_err(debug)?;
        let receipt = committed(award.transition)?;
        let obligation = obligation_of(&receipt)?;
        assert_eq!(receipt.committed_character_revision.get(), 2);
        assert_eq!(obligation.amount, 50);
        assert_eq!(obligation.occurrence.as_bytes()[6] >> 4, 7, "UUIDv7");
        let Some(Ok(ExperienceCommitOutcome::Committed(xp))) = award.experience else {
            return Err(format!("expected an XP commit, got {:?}", award.experience).into());
        };
        assert_eq!(xp.occurrence, obligation.occurrence);
        assert_eq!(xp.original_character_revision.get(), 2);
        assert_eq!(xp.committed_character_revision.get(), 3);
        assert_eq!(xp.experience_after.get(), 1050);
        assert!(obligations(pool).await?.is_empty(), "the award consumed it");
        // The award carries the active policy's reward revision, never the quest content
        // revision; the content revision stays the obligation's provenance only.
        let (reward, policy_revision): (String, String) = sqlx::query_as(
            "SELECT reward_revision, policy_revision FROM game_character_xp_receipts \
              WHERE reward_occurrence_id = $1::uuid",
        )
        .bind(occurrence_hex(&obligation))
        .fetch_one(pool)
        .await?;
        assert_eq!(
            (reward.as_str(), policy_revision.as_str()),
            ("reward-1", "policy-1")
        );

        // Replaying the transition returns its receipt without a pending obligation, and no
        // second award is submitted.
        let before = snapshot(pool).await?;
        let replay = slot
            .commit_quest_transition_with_experience(
                &harness.root,
                &authority,
                &harness.node,
                fence(1)?,
                begin,
                catalogue()?,
                &policy("reward-1"),
            )
            .await
            .map_err(debug)?;
        let QuestTransitionOutcome::AlreadyCommitted(replayed) = replay.transition else {
            return Err(format!("expected a replay, got {:?}", replay.transition).into());
        };
        assert_eq!(replayed.experience, None);
        assert!(replay.experience.is_none());
        assert_eq!(snapshot(pool).await?, before);

        // A transition without experience writes no obligation and submits no award.
        let plain = slot
            .commit_quest_transition_with_experience(
                &harness.root,
                &authority,
                &harness.node,
                fence(1)?,
                request("plain", 2)?,
                catalogue()?,
                &policy("reward-1"),
            )
            .await
            .map_err(debug)?;
        assert_eq!(
            committed(plain.transition)?
                .committed_character_revision
                .get(),
            4
        );
        assert!(plain.experience.is_none());
        assert!(obligations(pool).await?.is_empty());
        drop(slot);
        drop(authority);
        drop(seal);
        Ok(())
    })
}

#[test]
fn a_pending_quest_xp_obligation_is_requested_again_at_admission() -> TestResult {
    run("quest_xp_replay", async |harness| {
        let pool = &harness.pool;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        // The transition commits, then the node stops before the award.
        let begin = request("begin", 1)?;
        let receipt = committed(transition(harness, &authority, 1, begin.clone()).await?)?;
        let obligation = obligation_of(&receipt)?;
        assert_eq!(
            obligations(pool).await?,
            [(occurrence_hex(&obligation), 50, "content-1".to_owned())]
        );
        // A replay of the transition names the still-pending obligation.
        let QuestTransitionOutcome::AlreadyCommitted(replayed) =
            transition(harness, &authority, 1, begin).await?
        else {
            return Err("expected a replay".into());
        };
        assert_eq!(replayed.experience, Some(obligation));

        // Admission loads it and requests it again in the revision slot.
        let mut copy = harness
            .root
            .read_character_quest_state(&authority, character()?)
            .await
            .map_err(debug)?;
        assert_eq!(copy.xp_obligations(), [obligation]);
        let sequencer = CharacterRevisionSequencer::new();
        let retry = request_pending_quest_experience(
            &sequencer,
            &harness.root,
            &authority,
            &harness.node,
            fence(1)?,
            &policy("reward-1"),
            &mut copy,
        )
        .await;
        assert!(!retry);
        assert!(copy.xp_obligations().is_empty());
        assert!(obligations(pool).await?.is_empty());
        assert_eq!(harness.root_revision().await?, "3");
        let experience: i64 =
            sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state")
                .fetch_one(pool)
                .await?;
        assert_eq!(experience, 1050);

        // Idempotent: the exact award replays its receipt and writes nothing; a reload finds
        // nothing pending, so nothing is awarded twice.
        let before = snapshot(pool).await?;
        let replay = harness
            .root
            .commit_character_quest_experience(
                &authority,
                &harness.node,
                fence(2)?,
                obligation.award(&policy("reward-1")),
            )
            .await
            .map_err(debug)?;
        assert!(matches!(
            replay,
            ExperienceCommitOutcome::AlreadyCommitted(_)
        ));
        let mut reloaded = harness
            .root
            .read_character_quest_state(&authority, character()?)
            .await
            .map_err(debug)?;
        assert!(reloaded.xp_obligations().is_empty());
        assert!(
            !request_pending_quest_experience(
                &sequencer,
                &harness.root,
                &authority,
                &harness.node,
                fence(3)?,
                &policy("reward-1"),
                &mut reloaded,
            )
            .await
        );
        assert_eq!(snapshot(pool).await?, before);
        drop(authority);
        drop(seal);
        Ok(())
    })
}

#[test]
fn a_quest_xp_award_is_fenced_and_a_refusal_keeps_its_obligation() -> TestResult {
    run("quest_xp_refuse", async |harness| {
        let pool = &harness.pool;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let receipt = committed(transition(harness, &authority, 1, request("begin", 1)?).await?)?;
        let obligation = obligation_of(&receipt)?;
        let at_two = fence(2)?;
        let award = |request| {
            harness.root.commit_character_quest_experience(
                &authority,
                &harness.node,
                at_two,
                request,
            )
        };
        let before = snapshot(pool).await?;
        // Another amount, another reward revision, an occurrence without obligation: refused,
        // nothing written, the obligation kept.
        let mut short = obligation.award(&policy("reward-1"));
        short.amount = ExactI64::new(49);
        assert!(matches!(
            award(short).await,
            Err(CharacterProgressionError::InvalidInput)
        ));
        assert!(matches!(
            award(obligation.award(&policy("reward-2"))).await,
            Err(CharacterProgressionError::ProgressionContextMismatch)
        ));
        let stranger = QuestXpObligation {
            occurrence: ExperienceRewardOccurrence::from_bytes(id(77)).map_err(debug)?,
            amount: 50,
        };
        assert!(matches!(
            award(stranger.award(&policy("reward-1"))).await,
            Err(CharacterProgressionError::InvalidInput)
        ));
        // The session-generation fence holds: a stale connection generation is refused.
        let stale = crate::durability::character_progression::CurrentCharacterGameplayFence {
            connection_generation: crate::foundation::ConnectionGeneration::new(2)
                .map_err(debug)?,
            ..fence(2)?
        };
        assert!(matches!(
            harness
                .root
                .commit_character_quest_experience(
                    &authority,
                    &harness.node,
                    stale,
                    obligation.award(&policy("reward-1")),
                )
                .await,
            Err(CharacterProgressionError::AuthorityRejected)
        ));
        // The plain XP writer cannot award a quest occurrence without consuming it.
        assert!(matches!(
            harness
                .root
                .commit_character_experience(
                    &authority,
                    &harness.node,
                    fence(2)?,
                    obligation.award(&policy("reward-1")),
                )
                .await,
            Err(CharacterProgressionError::Unavailable(_))
        ));
        assert_eq!(snapshot(pool).await?, before);

        // A calculation refusal (past the terminal experience) keeps its obligation too, and
        // the admission replay reports it without retrying in this session.
        let sequencer = CharacterRevisionSequencer::new();
        let mut slot = sequencer.acquire(character()?).await;
        let huge = slot
            .commit_quest_transition_with_experience(
                &harness.root,
                &authority,
                &harness.node,
                fence(2)?,
                request("huge", 2)?,
                catalogue()?,
                &policy("reward-1"),
            )
            .await
            .map_err(debug)?;
        let huge_obligation = obligation_of(&committed(huge.transition)?)?;
        assert!(matches!(
            huge.experience,
            Some(Err(CharacterProgressionError::Calculation(_)))
        ));
        drop(slot);
        let mut copy = harness
            .root
            .read_character_quest_state(&authority, character()?)
            .await
            .map_err(debug)?;
        assert_eq!(copy.xp_obligations().len(), 2);
        assert!(copy.xp_obligations().contains(&obligation));
        assert!(copy.xp_obligations().contains(&huge_obligation));
        let retry = request_pending_quest_experience(
            &sequencer,
            &harness.root,
            &authority,
            &harness.node,
            fence(3)?,
            &policy("reward-1"),
            &mut copy,
        )
        .await;
        assert!(!retry, "a refusal is not an unknown outcome");
        assert_eq!(copy.xp_obligations(), [huge_obligation]);
        assert_eq!(obligations(pool).await?.len(), 1);
        assert_eq!(harness.root_revision().await?, "4");
        drop(authority);
        drop(seal);
        Ok(())
    })
}

#[test]
fn quest_xp_obligations_are_bounded_at_rl_10() -> TestResult {
    run("quest_xp_capacity", async |harness| {
        let pool = &harness.pool;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut revision = 1;
        for command_id in 1..=u64::try_from(QUESTGATE0_RL_10)? {
            committed(
                transition(harness, &authority, revision, request("again", command_id)?).await?,
            )?;
            revision += 1;
        }
        assert_eq!(obligations(pool).await?.len(), QUESTGATE0_RL_10);
        // The seventeenth XP-bearing transition is refused whole, before any write.
        let before = snapshot(pool).await?;
        assert_eq!(
            transition(harness, &authority, revision, request("again", 100)?).await?,
            QuestTransitionOutcome::Refused(QuestRefusal::OutOfRange)
        );
        assert_eq!(snapshot(pool).await?, before);
        // A transition without experience still commits.
        committed(transition(harness, &authority, revision, request("plain", 101)?).await?)?;
        revision += 1;
        // One award frees a slot.
        let mut copy = harness
            .root
            .read_character_quest_state(&authority, character()?)
            .await
            .map_err(debug)?;
        let first = copy.xp_obligations()[0];
        let awarded = harness
            .root
            .commit_character_quest_experience(
                &authority,
                &harness.node,
                fence(revision)?,
                first.award(&policy("reward-1")),
            )
            .await
            .map_err(debug)?;
        copy.settle_experience(first.occurrence, &Ok(awarded));
        assert_eq!(copy.xp_obligations().len(), QUESTGATE0_RL_10 - 1);
        revision += 1;
        committed(transition(harness, &authority, revision, request("again", 102)?).await?)?;
        assert_eq!(obligations(pool).await?.len(), QUESTGATE0_RL_10);
        drop(authority);
        drop(seal);
        Ok(())
    })
}

/// Runs `script` as one transaction; the rejection must be the named 0069 guard and change
/// nothing.
async fn expect_rejected(pool: &sqlx::PgPool, case: &str, script: &str, rule: &str) -> TestResult {
    let before = snapshot(pool).await?;
    let mut tx = pool.begin().await?;
    let result = match sqlx::raw_sql(sqlx::AssertSqlSafe(script.to_owned()))
        .execute(&mut *tx)
        .await
    {
        Ok(_) => tx.commit().await,
        Err(error) => Err(error),
    };
    let error = result
        .err()
        .ok_or_else(|| format!("{case}: expected rejection"))?;
    let database = error
        .as_database_error()
        .ok_or_else(|| format!("{case}: not a database error: {error}"))?;
    assert_eq!(database.code().as_deref(), Some("23514"), "{case}: {error}");
    assert_eq!(database.message(), rule, "{case}");
    assert_eq!(
        snapshot(pool).await?,
        before,
        "{case}: rejected write changed state"
    );
    Ok(())
}

#[test]
fn quest_xp_guards_reject_rows_without_their_receipts() -> TestResult {
    run("quest_xp_guards", async |harness| {
        let pool = &harness.pool;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        // r2: `begin` with its pending obligation; r3: `plain`, no obligation.
        let receipt = committed(transition(harness, &authority, 1, request("begin", 1)?).await?)?;
        let pending = occurrence_hex(&obligation_of(&receipt)?);
        committed(transition(harness, &authority, 2, request("plain", 2)?).await?)?;
        let character = uuid(CHARACTER);
        let cause = uuid(super::SESSION);
        let insert = |occurrence: &str, ordinal: u64, key: &str, pin: &str| {
            format!(
                "INSERT INTO game_character_quest_xp_obligations(reward_occurrence_id, \
                   character_id, cause_id, cause_ordinal, transition_key, amount, \
                   pinned_content_revision, created_at) \
                 VALUES ('{occurrence}'::uuid, {character}, {cause}, {ordinal}, '{key}', 50, \
                   '{pin}', 1);"
            )
        };
        // An obligation for the receipt of an earlier transaction.
        expect_rejected(
            pool,
            "obligation of an earlier receipt",
            &insert(&hex(&id(80)), 2, &transition_key("plain"), "content-1"),
            XP_PROVEN,
        )
        .await?;
        // Never updated; deleted only with its XP receipt.
        expect_rejected(
            pool,
            "update",
            &format!(
                "UPDATE game_character_quest_xp_obligations SET amount = 51 \
                  WHERE reward_occurrence_id = '{pending}'::uuid;"
            ),
            XP_ROW,
        )
        .await?;
        let delete = format!(
            "DELETE FROM game_character_quest_xp_obligations \
              WHERE reward_occurrence_id = '{pending}'::uuid;"
        );
        expect_rejected(pool, "delete without its XP receipt", &delete, XP_ROW).await?;
        // An XP receipt (r3 -> r4) of the pending occurrence without the delete, or a delete
        // with an XP receipt of another amount.
        let xp = |awarded: i64| {
            format!(
                "{advance}INSERT INTO game_character_xp_receipts(\
                   reward_occurrence_id, command_binding, policy_digest, character_id, \
                   original_character_revision, committed_character_revision, level_before, \
                   level_after, experience_before, experience_after, experience_awarded, \
                   profile_revision, ruleset_revision, content_revision, simulation_revision, \
                   evidence_revision, declaration_revision, policy_revision, reward_revision, \
                   committed_at) \
                 VALUES ('{pending}'::uuid, '\\x{binding}'::bytea, '\\x{digest}'::bytea, \
                   {character}, 3, 4, 50, 50, 1000, {after}, {awarded}, 'profile-1', \
                   'ruleset-1', 'content-1', 'simulation-1', 'evidence-1', 'declaration-1', \
                   'policy-1', 'reward-1', 1);",
                advance = advance(3, 1000 + awarded),
                binding = hex(&[9; 8]),
                digest = hex(&[9; 32]),
                after = 1000 + awarded,
            )
        };
        expect_rejected(pool, "award without consuming", &xp(50), XP_CONSUME).await?;
        expect_rejected(
            pool,
            "delete with an award of another amount",
            &format!("{}{delete}", xp(40)),
            XP_ROW,
        )
        .await?;
        expect_committed(pool, "award consuming", &format!("{}{delete}", xp(50))).await?;
        // r5: an XP receipt of an unrelated occurrence. An obligation cannot be born for an
        // occurrence already awarded, with another pin, nor past RL-10.
        expect_committed(pool, "unrelated XP r5", &award(81, 4, 1050, 1060)).await?;
        let key = transition_key("begin");
        let pin = ("content-1", receipt.definition_hash);
        let later = |original: u64, ordinal: u64| {
            let mut later = super::Receipt::new(original, ordinal, &[(STAGE, 1, 1)]);
            later.experience = 1060;
            later.transition = key.clone();
            later.pin = pin;
            later.continue_quest()
        };
        expect_rejected(
            pool,
            "obligation for an awarded occurrence",
            &format!(
                "{}{}",
                later(5, 9),
                insert(&hex(&id(81)), 9, &key, "content-1")
            ),
            XP_AWARDED,
        )
        .await?;
        expect_rejected(
            pool,
            "obligation with another pin",
            &format!(
                "{}{}",
                later(5, 10),
                insert(&hex(&id(90)), 10, &key, "content-2")
            ),
            XP_PROVEN,
        )
        .await?;
        // RL-10 pending through the writer (r5 -> r21), then a seventeenth by SQL.
        for (index, command_id) in (30..30 + u64::try_from(QUESTGATE0_RL_10)?).enumerate() {
            let revision = 5 + u64::try_from(index)?;
            committed(
                transition(harness, &authority, revision, request("again", command_id)?).await?,
            )?;
        }
        let script = format!(
            "{}{}",
            later(21, 50),
            insert(&hex(&id(91)), 50, &key, "content-1")
        );
        expect_rejected(pool, "seventeenth obligation", &script, XP_CAPACITY).await?;
        drop(authority);
        drop(seal);
        Ok(())
    })
}
