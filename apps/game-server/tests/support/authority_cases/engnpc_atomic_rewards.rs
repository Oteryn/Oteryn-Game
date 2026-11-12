//! Exercise the real reward transaction and its deferred primary/participant guards.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::reward_claim_mint::QuestRewardAdmission;
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_quests::quest::reward_callbacks::{self, RewardCallback};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTrack, QuestTransition};
use std::sync::Arc;

#[test]
fn engnpc_reward_sources_accept_sixteen_storage_operations_and_combined_tracks()
-> fixture::TestResult {
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
                    let h = fixture::Harness::create(admin, "engnpc_atomic_rewards", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "ENGNPC_PG_EXECUTED reward_sources duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "reward PG thread failed")?
        .map_err(Into::into)
}

struct NoItemWeights([u8; 32]);
impl crate::durability::item_transfer::ItemWeightLookup for NoItemWeights {
    fn generation_digest(&self) -> [u8; 32] {
        self.0
    }
    fn weight_of(
        &self,
        _: &crate::domain::item_definition::TypedDefinitionRef,
    ) -> Result<u32, crate::durability::item_transfer::ItemWeightRefused> {
        // These source fixtures only change quest state and never grant or consume items.
        Err(crate::durability::item_transfer::ItemWeightRefused::Unknown)
    }
}

async fn exercise(h: &fixture::Harness) -> fixture::TestResult {
    let (controller, _) =
        oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(
            fixture::scope()?.world_id(),
            None,
        );
    let native = controller
        .active()
        .and_then(|g| g.native_gameplay())
        .ok_or("native generation")?;
    let weights = NoItemWeights(native.source_digest());
    let admission = QuestRewardAdmission {
        native,
        weights: &weights,
        fee_payment: None,
        actor_conditions: None,
        carried_use: None,
    };
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let session = GameSessionId::decode(&fixture::id(fixture::SESSION)).map_err(fixture::debug)?;
    let mut revision = 1;
    for (index, mode) in [
        "storage",
        "operations",
        "participants",
        "foreign_overflow",
        "combined_overflow",
    ]
    .into_iter()
    .enumerate()
    {
        let quest = format!("oteryn:quest.engnpc_reward_{mode}");
        let foreign = format!("oteryn:quest.engnpc_foreign_{mode}");
        let count = match mode {
            "participants" => 9,
            "foreign_overflow" => 7,
            "combined_overflow" => 10,
            _ => 16,
        };
        let storage = |count, prefix: &str| {
            (0..count).map(|i| serde_json::json!({
            "track":format!("oteryn:quest-progress/{prefix}/{i:02}"), "minimum":0, "maximum":0, "set":1
        })).collect::<Vec<_>>()
        };
        let mut row = serde_json::json!({
            "key":format!("oteryn:quest-reward/engnpc-{mode}"), "quest":quest,
            "npc":"oteryn:npc.fixture", "keywords":["yes"], "topic":0, "next_topic":0,
            "storage":storage(count, mode), "take":[], "give":null, "outfit":null,
            "achievement":null, "reply":["Committed."], "sources":["engine fixture"]
        });
        if mode == "operations" {
            row["state_ops"] = (0..count)
                .map(|i| {
                    serde_json::json!({
                        "track":format!("oteryn:quest-progress/{mode}/{i:02}"),
                        "from":{"kind":"between","minimum":0,"maximum":0},
                        "effect":{"kind":"add","value":1}
                    })
                })
                .collect::<Vec<_>>()
                .into();
            row["storage"] = serde_json::json!([]);
        }
        if matches!(
            mode,
            "participants" | "foreign_overflow" | "combined_overflow"
        ) {
            row["participants"] = serde_json::json!([{"quest":foreign,"storage":storage(if mode == "foreign_overflow" {9} else {7},&format!("foreign-{mode}"))}]);
        }
        let callback: RewardCallback = serde_json::from_value(row)?;
        let mut tracks: Vec<_> = callback
            .storage
            .iter()
            .map(|s| s.track.clone())
            .chain(callback.state_ops.iter().map(|s| s.track.clone()))
            .map(|key| QuestTrack {
                key,
                quest: quest.clone(),
                initial: 0,
                min: 0,
                max: 1,
            })
            .collect();
        let mut transitions =
            vec![reward_callbacks::primary_transition(&callback).map_err(fixture::debug)?];
        for (ordinal, participant) in callback.participants.iter().enumerate() {
            tracks.extend(participant.storage.iter().map(|s| QuestTrack {
                key: s.track.clone(),
                quest: foreign.clone(),
                initial: 0,
                min: 0,
                max: 1,
            }));
            transitions.push(QuestTransition {
                key: reward_callbacks::participant_key(&callback.key, ordinal),
                quest: foreign.clone(),
                effects: participant
                    .storage
                    .iter()
                    .map(|s| oteryn_gameplay_quests::quest::QuestEffect {
                        track: s.track.clone(),
                        from: oteryn_gameplay_quests::quest::QuestComparison::Eq(0),
                        effect: oteryn_gameplay_quests::quest::QuestEffectKind::Set(1),
                    })
                    .collect(),
                completes: false,
                experience: None,
                world_change: None,
            });
        }
        let catalogue = Arc::new(
            QuestStateCatalogue::new("content-1", tracks.clone(), transitions.clone())
                .map_err(fixture::debug)?,
        );
        let request = QuestTransitionRequest {
            transition_key: callback.key.clone(),
            cause: QuestCause::Command(CommandRef::new(
                session,
                CommandId::new(9100 + u64::try_from(index)?).map_err(fixture::debug)?,
            )),
        };
        let fence = fixture::fence(revision)?;
        let before = h
            .root
            .read_character_quest_state(&authority, fence.character_id)
            .await
            .map_err(fixture::debug)?;
        if !mode.ends_with("overflow") {
            let mut refusing = transitions;
            refusing
                .first_mut()
                .ok_or("primary transition")?
                .effects
                .last_mut()
                .ok_or("final effect")?
                .from = oteryn_gameplay_quests::quest::QuestComparison::Eq(1);
            let refusing = Arc::new(
                QuestStateCatalogue::new("content-1", tracks, refusing).map_err(fixture::debug)?,
            );
            let refused = h
                .root
                .commit_fixture_quest_reward(
                    &authority,
                    &h.node,
                    fence,
                    request.clone(),
                    refusing,
                    admission,
                    callback.clone(),
                )
                .await
                .map_err(fixture::debug)?;
            assert_eq!(
                refused,
                QuestTransitionOutcome::Refused(
                    oteryn_gameplay_quests::quest::QuestRefusal::StageMismatch
                )
            );
            assert_eq!(
                before,
                h.root
                    .read_character_quest_state(&authority, fence.character_id)
                    .await
                    .map_err(fixture::debug)?
            );
        }
        let committed = h
            .root
            .commit_fixture_quest_reward(
                &authority,
                &h.node,
                fence,
                request.clone(),
                catalogue.clone(),
                admission,
                callback.clone(),
            )
            .await;
        if mode.ends_with("overflow") {
            assert!(committed.is_err(), "oversized source must refuse: {mode}");
            assert_eq!(
                before,
                h.root
                    .read_character_quest_state(&authority, fence.character_id)
                    .await
                    .map_err(fixture::debug)?
            );
            let sources: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_quest_reward_sources WHERE transition_key=$1",
            )
            .bind(&callback.key)
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(sources, 0);
            continue;
        }
        let committed = committed.map_err(fixture::debug)?;
        let QuestTransitionOutcome::Committed(receipt) = committed else {
            return Err(format!("{mode}: {committed:?}").into());
        };
        assert_eq!(receipt.changes.len(), count);
        assert_eq!(
            receipt.committed_character_revision.get(),
            receipt.original_character_revision.get() + 1
        );
        let stored: (i64,i64,i64) = sqlx::query_as("SELECT count(*),min(value),max(value) FROM game_character_quest_tracks WHERE quest_key=$1 OR quest_key=$2").bind(&quest).bind(&foreign).fetch_one(&h.pool).await?;
        assert_eq!(stored, (16, 1, 1));
        let source: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_quest_reward_sources WHERE transition_key=$1",
        )
        .bind(&callback.key)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(source, 1);
        let replay = h
            .root
            .commit_fixture_quest_reward(
                &authority, &h.node, fence, request, catalogue, admission, callback,
            )
            .await
            .map_err(fixture::debug)?;
        assert_eq!(replay, QuestTransitionOutcome::AlreadyCommitted(receipt));
        revision += 1;
    }
    Ok(())
}
