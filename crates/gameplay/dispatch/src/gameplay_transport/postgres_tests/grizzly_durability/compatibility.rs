//! An old active quest pin is preserved until an explicit migration is accepted.
//! The predecessor is committed by the actual legacy writer, not injected with SQL.

use super::*;
use crate::durability::quest_state::QuestTransitionRequest;

const LEGACY_TRANSITION: &str =
    "oteryn:quest-transition/quest/u8_5/killing_in_the_name_of/alt_kill_count/minotaur_archer_count/npc_1";

#[test]
fn grizzly_legacy_active_quest_pin_refuses_without_reinterpreting_value() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("GRIZZLY_LEGACY_PIN POSTGRES_CASE_SKIPPED no_admin_url");
        return Ok(());
    }
    super::super::kill_reward_e2e::scenario_thread::run("grizzly-legacy-pin", scenario)
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create(admin, "grizzly_legacy_pin", true).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let sequencer = CharacterRevisionSequencer::new();
        let legacy = oteryn_gameplay_quests::quest::loader::parse_quest_state(
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"),
                "/../../../content/quests/missions/quest-state.json")),
            "content-1",
        ).map_err(debug)?;
        let legacy = Arc::new(legacy.catalogue().clone());
        let current = catalogue()?;
        assert_ne!(legacy.definition_hash(quest::QUEST), current.definition_hash(quest::QUEST));

        // Budrik's old exact SET(0) fragment shares the same quest state as Grizzly.
        let predecessor = QuestTransitionRequest {
            transition_key: LEGACY_TRANSITION.into(), cause: command(1)?,
        };
        let receipt = committed(sequencer.acquire(fence(1)?.character_id).await
            .commit_quest_transition(&h.root, &authority, &h.node, fence(1)?,
                predecessor.clone(), Arc::clone(&legacy)).await.map_err(debug)?)?;
        let before = state(&h, &authority).await?;
        let pinned = before.states().get(quest::QUEST).ok_or("legacy quest state")?;
        assert!(!pinned.completed);
        assert_eq!(Some(pinned.definition_hash), legacy.definition_hash(quest::QUEST));
        assert_eq!(before.tracks().len(), 1);
        assert_eq!(before.tracks().values().next(), Some(&0));
        let gate = PremiumProbe::new(false);
        for _ in 0..2 {
            assert_eq!(apply(&h, &authority, &sequencer, &current, command(2)?,
                Action::Join, &gate).await?,
                QuestTransitionOutcome::Refused(QuestRefusal::RevisionMismatch));
            assert_eq!(state(&h, &authority).await?, before);
        }
        assert_eq!(h.root.reconcile_character_grizzly_transition(&authority,
            fence(1)?.character_id, command(2)?, Action::Join).await.map_err(debug)?,
            Some(QuestTransitionOutcome::Refused(QuestRefusal::RevisionMismatch)));

        // An unchanged sibling NPC transition is also blocked by the old quest pin.
        assert_eq!(sequencer.acquire(fence(1)?.character_id).await
            .commit_quest_transition(&h.root, &authority, &h.node, fence(1)?,
                QuestTransitionRequest { transition_key: LEGACY_TRANSITION.into(),
                    cause: command(3)? }, Arc::clone(&current)).await.map_err(debug)?,
            QuestTransitionOutcome::Refused(QuestRefusal::RevisionMismatch));
        // Historical receipt replay remains legal; it cannot replace the stored pin.
        assert_eq!(sequencer.acquire(fence(1)?.character_id).await
            .commit_quest_transition(&h.root, &authority, &h.node, fence(1)?,
                predecessor, Arc::clone(&current)).await.map_err(debug)?,
            QuestTransitionOutcome::AlreadyCommitted(receipt));
        assert_eq!(state(&h, &authority).await?, before);
        let counts: (i64, i64, i64, i64) = sqlx::query_as(
            "SELECT (SELECT count(*) FROM game_character_quest_receipts), \
                    (SELECT count(*) FROM game_character_grizzly_refusals \
                      WHERE result_code='REVISION_MISMATCH'), \
                    (SELECT count(*) FROM game_character_quest_xp_obligations), \
                    (SELECT count(*) FROM game_character_grizzly_reward_entitlements)")
            .fetch_one(&h.pool).await?;
        assert_eq!(counts, (1, 1, 0, 0));
        eprintln!("GRIZZLY_LEGACY_PIN POSTGRES_CASE_COMPLETED");
        drop(authority); drop(seal); h.cleanup().await
    })
}
