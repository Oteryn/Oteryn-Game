use crate::bestiary_postgres_harness::{
    CHARACTER, Harness, TestResult, configured_admin, debug, fence, id, runtime,
};
use crate::domain::CharacterId;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use std::sync::Arc;

#[test]
fn qn2_unordered_claims_durably_complete_on_last_chest() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            runtime().map_err(debug)?.block_on(async move {
                let harness = Harness::create(admin, "qn2_claim_sets", true)
                    .await
                    .map_err(debug)?;
                eprintln!("QN2_POSTGRES_READY");
                let seal = harness.recovery.seal_current().map_err(debug)?;
                let authority = harness
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(debug)?;
                let catalogue = Arc::new(
                    oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                        .map_err(debug)?
                        .catalogue()
                        .clone(),
                );
                let character = CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?;
                for (index, marker) in ["bear_chest3", "bear_chest1", "bear_chest2"]
                    .iter()
                    .enumerate()
                {
                    let request = QuestTransitionRequest {
                        transition_key: format!("oteryn:quest-transition/kv/{marker}/chest"),
                        cause: QuestCause::Command(CommandRef::new(
                            GameSessionId::decode(&id(50)).map_err(debug)?,
                            CommandId::new(100 + index as u64).map_err(debug)?,
                        )),
                    };
                    let outcome = harness
                        .root
                        .commit_character_quest_transition(
                            &authority,
                            &harness.node,
                            fence(index as u64 + 1).map_err(debug)?,
                            request.clone(),
                            Arc::clone(&catalogue),
                        )
                        .await
                        .map_err(debug)?;
                    let QuestTransitionOutcome::Committed(receipt) = outcome else {
                        return Err(format!("{outcome:?}"));
                    };
                    assert_eq!(receipt.completes, index == 2);
                    let replay = harness
                        .root
                        .commit_character_quest_transition(
                            &authority,
                            &harness.node,
                            fence(index as u64 + 1).map_err(debug)?,
                            request,
                            Arc::clone(&catalogue),
                        )
                        .await
                        .map_err(debug)?;
                    assert!(matches!(
                        replay,
                        QuestTransitionOutcome::AlreadyCommitted(_)
                    ));
                    let copy = harness
                        .root
                        .read_character_quest_state(&authority, character)
                        .await
                        .map_err(debug)?;
                    assert_eq!(
                        copy.states()["oteryn:quest.bear_room_quest"].completed,
                        index == 2
                    );
                }
                drop(authority);
                drop(seal);
                harness.cleanup().await.map_err(debug)?;
                Ok(())
            })
        })?
        .join()
        .map_err(|_| "qn2 test panicked")?
        .map_err(Into::into)
}
