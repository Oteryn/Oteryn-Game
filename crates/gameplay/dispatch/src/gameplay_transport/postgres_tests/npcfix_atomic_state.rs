//! Father’s Burden acceptance is one eleven-track transaction, never partial starts.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestRefusal, QuestStateCatalogue, QuestTrack,
    QuestTransition,
};
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.a_father_s_burden_quest";
const KEY: &str = "oteryn:quest-transition/npcfix/fathers-burden/accept";
const NPC_ACCEPT: &str = "oteryn:quest-npc/a_father_s_burden_quest/tereban_functions/mission-accept";

fn acceptance(failure: Option<QuestRefusal>) -> TestResult<Arc<QuestStateCatalogue>> {
    // Existing normalized source, Crystal 201189f, tereban_functions.lua:173-184.
    let source: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../content/dialogues/quest-rules/quest-batch3-a-fathers_burden.json"
    ))?;
    let rule = source["rules"]
        .as_array()
        .ok_or("rules")?
        .iter()
        .find(|rule| {
            rule["key"]
                .as_str()
                .is_some_and(|key| key.ends_with("/mission-accept"))
        })
        .ok_or("acceptance")?;
    let mut tracks = Vec::new();
    let mut effects = Vec::new();
    for effect in rule["effects"].as_array().ok_or("effects")? {
        if effect["kind"] != "set" {
            continue;
        }
        let track = effect["track"].as_str().ok_or("track")?.to_owned();
        tracks.push(QuestTrack {
            key: track.clone(),
            quest: QUEST.into(),
            initial: -1,
            min: -1,
            max: 10,
        });
        effects.push(QuestEffect {
            track,
            from: QuestComparison::Eq(-1),
            effect: QuestEffectKind::Set(effect["value"].as_i64().ok_or("value")?),
        });
    }
    assert_eq!(effects.len(), 11);
    let last = effects.last_mut().ok_or("last effect")?;
    match failure {
        Some(QuestRefusal::StageMismatch) => last.from = QuestComparison::Eq(9),
        Some(QuestRefusal::OutOfRange) => last.effect = QuestEffectKind::Add(12),
        None => {}
        _ => return Err("unexpected fixture refusal".into()),
    }
    Ok(Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            tracks,
            vec![QuestTransition { world_change: None,
                key: KEY.into(),
                quest: QUEST.into(),
                effects,
                completes: false,
                experience: None,
            }],
        )
        .map_err(debug)?,
    ))
}

#[test]
fn npcfix_eleven_track_postgres_atomic_receipt_replay_and_rollback() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "atomic NPC state thread panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        for failure in [
            None,
            Some(QuestRefusal::StageMismatch),
            Some(QuestRefusal::OutOfRange),
        ] {
            let h = Harness::create_with_admission_seed(
                admin.clone(),
                "npcfix_atomic_state",
                true,
                false,
            )
            .await?;
            let result: TestResult = async {
                eprintln!("POSTGRES_STARTED NPCFIX eleven-track acceptance {failure:?}");
                prepare_current_session(&h).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
                let deps = Dependencies::new()?;
                let owner = deps.owner(&h, &authority);
                deps.initialize(&owner).await;
                let fence = owner.current_quest_fence(deps.admitted.game_session_id).await
                    .map_err(|()| "fence")?.ok_or("live fence")?;
                let catalogue = if failure.is_none() {
                    let loaded = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1").map_err(debug)?;
                    let source = oteryn_gameplay_quests::quest::npc_rules::NpcRuleCatalog::runtime().map_err(debug)?;
                    assert!(source.admitted_for_npc("oteryn:npc.tereban", loaded.catalogue()).map_err(debug)?.is_some(), "production acceptance programme");
                    Arc::new(loaded.catalogue().clone())
                } else { acceptance(failure)? };
                let request = QuestTransitionRequest { transition_key: KEY.into(), cause: QuestCause::Command(
                    CommandRef::new(deps.admitted.game_session_id, CommandId::new(1).map_err(debug)?)) };
                let npc_request = crate::durability::quest_state::npc_program::NpcProgramRequest {
                    command: CommandRef::new(deps.admitted.game_session_id, CommandId::new(1).map_err(debug)?),
                    npc: "oteryn:npc.tereban".into(),
                    trigger: oteryn_gameplay_quests::quest::npc_rules::NpcRuleTrigger::Say,
                    message: "yes".into(), topic: 1, transition_key: NPC_ACCEPT.into(),
                };
                let outcome = if failure.is_none() {
                    // Main1314 admits precisely this immutable source acceptance.
                    let world = crate::foundation::WorldId::decode(&super::fixture::id(42)).map_err(debug)?;
                    let (controller, _) = crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
                    let active = controller.active().ok_or("active acceptance generation")?;
                    let native = active.native_gameplay().ok_or("native acceptance content")?;
                    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
                    slot.commit_npc_program(&h.root, &authority, &h.node, fence, npc_request.clone(), Arc::clone(&catalogue),
                        &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native))
                        .await.map_err(debug)?.transition
                } else {
                    h.root.commit_character_quest_transition(&authority, &h.node, fence,
                        request.clone(), Arc::clone(&catalogue)).await.map_err(debug)?
                };
                if let Some(refusal) = failure {
                    assert!(matches!(outcome, QuestTransitionOutcome::Refused(actual) if actual == refusal));
                    let copy = h.root.read_character_quest_state(&authority, fence.character_id).await.map_err(debug)?;
                    assert!(copy.tracks().is_empty());
                    assert!(copy.states().is_empty());
                    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
                        .fetch_one(&h.pool).await?;
                    assert_eq!(receipts, 0);
                    let current = owner.current_quest_fence(deps.admitted.game_session_id).await
                        .map_err(|()| "fence")?.ok_or("live fence")?;
                    assert_eq!(current, fence, "refusal leaves the revision unchanged");
                } else {
                    let QuestTransitionOutcome::Committed(receipt) = outcome else { return Err("one committed receipt".into()); };
                    assert_eq!(receipt.changes.len(), 11);
                    assert_eq!(receipt.committed_character_revision.get(), receipt.original_character_revision.get() + 1);
                    let replay = h.root.reconcile_npc_program(&authority, fence.character_id, npc_request)
                        .await.map_err(debug)?.ok_or("canonical acceptance replay")?.transition;
                    assert!(matches!(replay, QuestTransitionOutcome::AlreadyCommitted(replayed) if replayed == receipt));
                    let copy = h.root.read_character_quest_state(&authority, fence.character_id).await.map_err(debug)?;
                    assert_eq!(copy.tracks().len(), 11);
                    for change in &receipt.changes { assert_eq!(copy.tracks().get(&change.track), Some(&change.after)); }
                    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
                        .fetch_one(&h.pool).await?;
                    assert_eq!(receipts, 1);
                }
                Ok(())
            }.await;
            h.cleanup().await?;
            result?;
        }
        Ok(())
    })
}
