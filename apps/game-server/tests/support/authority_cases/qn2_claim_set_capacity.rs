//! Reading other required claims must not reserve their unwritten tracks.
use crate::bestiary_postgres_harness::{
    Harness, TestResult, configured_admin, debug, fence, id, runtime,
};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_quests::quest::{
    self, QuestComparison, QuestEffect, QuestEffectKind, QuestTrack, QuestTransition,
};
use std::sync::Arc;

#[test]
fn qn2_claim_set_reads_do_not_consume_the_last_track_slot() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            runtime().map_err(debug)?.block_on(async move {
                let harness = Harness::create(admin, "qn2_claim_capacity", true)
                    .await
                    .map_err(debug)?;
                eprintln!("QN2_M35_CAPACITY_POSTGRES_READY");
                let seal = harness.recovery.seal_current().map_err(debug)?;
                let authority = harness
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(debug)?;
                let base = quest::loader::load_embedded_quest_state("content-1").map_err(debug)?;
                let owner = "oteryn:quest.qn2_capacity";
                let tracks: Vec<_> = (0..quest::QUESTSTATE0_RL_08_TRACKS - 1)
                    .map(|i| QuestTrack {
                        key: format!("oteryn:quest-progress/qn2-capacity/{i:04}"),
                        quest: owner.into(),
                        initial: 0,
                        min: 0,
                        max: 1,
                    })
                    .collect();
                let transitions: Vec<_> = tracks
                    .chunks(quest::QUESTSTATE0_RL_02)
                    .enumerate()
                    .map(|(i, chunk)| QuestTransition {
                        key: format!("oteryn:quest-transition/qn2-capacity/seed-{i}"),
                        quest: owner.into(),
                        effects: chunk
                            .iter()
                            .map(|track| QuestEffect {
                                track: track.key.clone(),
                                from: QuestComparison::Eq(0),
                                effect: QuestEffectKind::Set(1),
                            })
                            .collect(),
                        completes: false,
                        experience: None,
                        world_change: None,
                    })
                    .collect();
                let seed_keys: Vec<_> = transitions.iter().map(|t| t.key.clone()).collect();
                let catalogue = Arc::new(
                    base.catalogue()
                        .with_quest(tracks, transitions)
                        .map_err(debug)?,
                );
                let mut revision = 1;
                for (i, key) in seed_keys.into_iter().enumerate() {
                    let request = QuestTransitionRequest {
                        transition_key: key,
                        cause: QuestCause::Command(CommandRef::new(
                            GameSessionId::decode(&id(50)).map_err(debug)?,
                            CommandId::new(1000 + i as u64).map_err(debug)?,
                        )),
                    };
                    let outcome = harness
                        .root
                        .commit_character_quest_transition(
                            &authority,
                            &harness.node,
                            fence(revision).map_err(debug)?,
                            request,
                            Arc::clone(&catalogue),
                        )
                        .await
                        .map_err(debug)?;
                    let QuestTransitionOutcome::Committed(receipt) = outcome else {
                        return Err(format!("seed: {outcome:?}"));
                    };
                    revision = receipt.committed_character_revision.get();
                }
                let request = QuestTransitionRequest {
                    transition_key: "oteryn:quest-transition/kv/bear_chest2/chest".into(),
                    cause: QuestCause::Command(CommandRef::new(
                        GameSessionId::decode(&id(50)).map_err(debug)?,
                        CommandId::new(9000).map_err(debug)?,
                    )),
                };
                let outcome = harness
                    .root
                    .commit_character_quest_transition(
                        &authority,
                        &harness.node,
                        fence(revision).map_err(debug)?,
                        request,
                        Arc::clone(&catalogue),
                    )
                    .await
                    .map_err(debug)?;
                let QuestTransitionOutcome::Committed(receipt) = outcome else {
                    return Err(format!(
                        "one writable slot must admit one claim: {outcome:?}"
                    ));
                };
                assert!(!receipt.completes);
                assert_eq!(receipt.changes.len(), 1);
                let character = crate::domain::CharacterId::from_bytes(id(41)).map_err(debug)?;
                let copy = harness
                    .root
                    .read_character_quest_state(&authority, character)
                    .await
                    .map_err(debug)?;
                assert_eq!(copy.tracks().len(), quest::QUESTSTATE0_RL_08_TRACKS);
                drop(authority);
                drop(seal);
                harness.cleanup().await.map_err(debug)?;
                Ok(())
            })
        })?
        .join()
        .map_err(|_| "claim capacity test panicked")?
        .map_err(Into::into)
}
