//! Durable qualification of the existing choice adapter, not native NPC admission.
//! The fixture has no stance; no unknown stance-fit, island or departure rule is inferred.
//! Training is committed through the real sequencer before current build/stance are read.

use crate::{domain, durability};

#[allow(dead_code)]
#[path = "../support/bestiary_postgres_harness.rs"]
mod bestiary_postgres_harness;

use crate::gameplay_transport::vocation_choice::{
    RetainedNpcVocation, commit_retained_npc_vocation, vocation_choice_request,
};

use bestiary_postgres_harness::{
    CHARACTER, Harness, TestResult, configured_admin, debug, fence, id, runtime,
};
use domain::CharacterId;
use durability::DurabilityRoot;
use durability::character_build::{
    BuildCause, BuildChangeRequest, BuildCommitOutcome, BuildFormula, BuildOccurrence,
    DurableBuildState, MAX_BUILD_LEVEL, MIN_SKILL_LEVEL, NO_VOCATION, cumulative_progress,
};
use durability::character_progression::CharacterProgressionError;
use durability::character_revision_sequencer::CharacterRevisionSequencer;

/// Synthetic formula data exercises re-levelling of all eight families. It is not a
/// qualified Reference formula or evidence for native Dawnport availability.
struct Table;

impl BuildFormula for Table {
    fn required(&self, vocation: &str, family: usize, level: u16) -> Option<u64> {
        let floor = if family == 0 { 0 } else { MIN_SKILL_LEVEL };
        if family > 7 || !(floor + 1..=MAX_BUILD_LEVEL).contains(&level) {
            return None;
        }
        let base = match vocation {
            NO_VOCATION => 100,
            "knight" => 50,
            _ => return None,
        };
        Some(base * (family as u64 + 1))
    }

    fn digest(&self) -> [u8; 32] {
        [7; 32]
    }

    fn content_revision(&self) -> &str {
        "content-1"
    }
}

fn assert_progress_preserved(before: &DurableBuildState, after: &DurableBuildState) {
    let old = std::iter::once(before.magic()).chain(before.skills());
    let new = std::iter::once(after.magic()).chain(after.skills());
    for (family, ((old_level, old_progress), (new_level, new_progress))) in old.zip(new).enumerate()
    {
        let floor = if family == 0 { 0 } else { MIN_SKILL_LEVEL };
        assert_eq!(
            cumulative_progress(old_level, old_progress, floor, |level| {
                Table.required(before.vocation(), family, level)
            }),
            cumulative_progress(new_level, new_progress, floor, |level| {
                Table.required(after.vocation(), family, level)
            }),
            "family {family}"
        );
    }
}

#[test]
fn native_npc_receipt_core_preserves_training_and_replays_after_reopen() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "dawnport_choice", true).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let character = CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?;
        let sequencer = CharacterRevisionSequencer::new();
        let mut slot = sequencer.acquire(character).await;
        let seed = harness
            .root
            .read_character_build_state(&authority, character)
            .await
            .map_err(debug)?;
        let trained = DurableBuildState::new(NO_VOCATION, (3, 17), [(12, 3); 7]).map_err(debug)?;
        let training = BuildChangeRequest {
            occurrence: BuildOccurrence::from_bytes(id(80)).map_err(debug)?,
            cause: BuildCause::Training,
            before: seed.clone(),
            after: trained.clone(),
            pruned_stance: None,
        };
        let outcome = slot
            .commit_build(
                &harness.root,
                &authority,
                &harness.node,
                fence(1)?,
                training,
                &Table,
                None,
            )
            .await
            .map_err(debug)?;
        assert!(matches!(outcome, BuildCommitOutcome::Committed(_)));

        // A pre-flush build cannot discard the newly committed training, even with the
        // current revision. This negative changes only the build predecessor.
        let occurrence = BuildOccurrence::from_bytes(id(81)).map_err(debug)?;
        let stale =
            vocation_choice_request(occurrence, seed, "knight", &Table, None).map_err(debug)?;
        let outcome = slot
            .commit_build(
                &harness.root,
                &authority,
                &harness.node,
                fence(2)?,
                stale,
                &Table,
                None,
            )
            .await;
        assert!(matches!(
            outcome,
            Err(CharacterProgressionError::BuildStateMismatch)
        ));
        assert_eq!(harness.root_revision().await?, "2");
        assert_eq!(harness.count("game_character_build_receipts").await?, 1);

        let before = harness
            .root
            .read_character_build_state(&authority, character)
            .await
            .map_err(debug)?;
        let stance = harness
            .root
            .read_character_stance(&authority, character)
            .await
            .map_err(debug)?;
        assert_eq!(before, trained);
        assert_eq!(stance, Default::default());
        let choice =
            vocation_choice_request(occurrence, before, "knight", &Table, None).map_err(debug)?;
        let retained = RetainedNpcVocation {
            fence: fence(2)?,
            request: choice.clone(),
            formula_digest: Table.digest(),
        };
        let receipt = commit_retained_npc_vocation(
            &mut slot,
            &harness.root,
            &authority,
            &harness.node,
            &retained,
            &Table,
        )
        .await
        .map_err(debug)?;
        assert_eq!(receipt.original_character_revision.get(), 2);
        assert_eq!(receipt.committed_character_revision.get(), 3);
        assert_eq!(receipt.before, trained);
        assert_eq!(receipt.after, choice.after);
        assert_ne!(receipt.before.magic(), receipt.after.magic());
        assert_progress_preserved(&receipt.before, &receipt.after);
        drop(slot);
        drop(authority);

        // A new root and sequencer have no cached cursor or choice state. Admission verifies
        // the persisted chain; reconcile recovers a response lost after the original commit.
        let restarted = DurabilityRoot::connect_test_runtime(&harness.database.url)?;
        assert!(restarted.maintain_ready_once().await?);
        let reopened = restarted
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        assert_eq!(
            restarted
                .reconcile_character_build(&reopened, occurrence)
                .await
                .map_err(debug)?,
            Some(receipt.clone())
        );
        let sequencer = CharacterRevisionSequencer::new();
        let mut slot = sequencer.acquire(character).await;
        let replay = commit_retained_npc_vocation(
            &mut slot,
            &restarted,
            &reopened,
            &harness.node,
            &retained,
            &Table,
        )
        .await
        .map_err(debug)?;
        assert_eq!(replay, receipt);
        assert_eq!(
            restarted
                .read_character_build_state(&reopened, character)
                .await
                .map_err(debug)?,
            receipt.after
        );
        // The same occurrence with a changed intent conflicts, including after reopen.
        let mut conflicting = choice;
        conflicting.pruned_stance = Some("guard".into());
        let conflicting = RetainedNpcVocation {
            request: conflicting,
            ..retained
        };
        let outcome = commit_retained_npc_vocation(
            &mut slot,
            &restarted,
            &reopened,
            &harness.node,
            &conflicting,
            &Table,
        )
        .await;
        assert!(matches!(
            outcome,
            Err(CharacterProgressionError::ConflictingOccurrence)
        ));
        assert_eq!(harness.root_revision().await?, "3");
        assert_eq!(harness.count("game_character_build_receipts").await?, 2);
        let choices: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_build_receipts WHERE cause = 'vocation_choice'",
        )
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(choices, 1);
        assert_eq!(harness.count("game_character_stance_receipts").await?, 0);
        assert_eq!(
            restarted
                .read_character_stance(&reopened, character)
                .await
                .map_err(debug)?,
            stance
        );
        drop(slot);
        drop(reopened);
        drop(seal);
        harness.cleanup().await
    })
}
