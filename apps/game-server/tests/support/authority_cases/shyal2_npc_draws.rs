//! The fenced source NPC writer commits draws with quest tracks and replay receipts.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::item_loop_move::ItemLoopContent;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{ItemDefinitionFacts, ItemWeightLookup, ItemWeightRefused};
use crate::durability::quest_state::{QuestTransitionOutcome, npc_program::NpcProgramRequest};
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack,
    QuestTransition,
    npc_rules::{NpcRuleCatalog, NpcRuleTrigger},
};
use std::collections::BTreeSet;
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.shadows_of_yalahar_quest";
const ORDER: [&str; 3] = [
    "oteryn:quest-progress/shyal2/mission10-order1",
    "oteryn:quest-progress/shyal2/mission10-order2",
    "oteryn:quest-progress/shyal2/mission10-order3",
];
const PART: &str = "oteryn:quest-progress/shyal2/mission12-repair-item";

struct NoItems;

impl ItemWeightLookup for NoItems {
    fn generation_digest(&self) -> [u8; 32] {
        [1; 32]
    }

    fn weight_of(&self, _: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
        Err(ItemWeightRefused::Unknown)
    }
}

impl ItemLoopContent for NoItems {
    fn loop_content_digest(&self) -> [u8; 32] {
        [1; 32]
    }

    fn loop_facts(&self, _: &TypedDefinitionRef) -> Option<ItemDefinitionFacts> {
        None
    }
}

#[test]
fn shyal2_telas_ingredient_order_commits_three_distinct_draws_and_replays() -> fixture::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, "shyal2_telas", &ORDER, 6, 1)
}

#[test]
fn shyal2_golem_servant_repair_requests_reset_the_pool_and_replay() -> fixture::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, "shyal2_golem", &[PART], 12, 13)
}

fn run(
    admin: String,
    label: &'static str,
    tracks: &'static [&'static str],
    pool: i64,
    requests: u64,
) -> fixture::TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = fixture::Harness::create(admin, label, true).await?;
                    let result = exercise(&h, tracks, pool, requests).await;
                    h.cleanup().await?;
                    result?;
                    eprintln!(
                        "POSTGRES_CASE_COMPLETED {label} duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "draw PG thread failed")?
        .map_err(Into::into)
}

async fn exercise(
    h: &fixture::Harness,
    tracks: &[&str],
    pool: i64,
    requests: u64,
) -> fixture::TestResult {
    let (npc, keyword, reference) = if tracks.len() == 3 {
        (
            "oteryn:npc.telas",
            "mission",
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/telas.lua:385-399",
        )
    } else {
        (
            "oteryn:npc.golem_servant",
            "yes",
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/golem_servant.lua:84-108",
        )
    };
    let rule = "oteryn:quest-npc/shyal2/draw";
    let definitions: Vec<_> = tracks
        .iter()
        .map(|key| QuestTrack {
            key: (*key).into(),
            quest: QUEST.into(),
            initial: 0,
            min: 0,
            max: pool,
        })
        .collect();
    let doc_tracks: Vec<_> = definitions
        .iter()
        .map(|track| {
            serde_json::json!({
                "key": track.key, "quest": QUEST, "initial": track.initial,
                "min": track.min, "max": track.max, "sources": [reference],
            })
        })
        .collect();
    let source = NpcRuleCatalog::from_json_documents(&[&serde_json::json!({
        "schema": 1, "quest": QUEST, "tracks": doc_tracks,
        "rules": [{"key": rule, "quest": QUEST, "npc": npc, "trigger": "say",
            "keywords": [keyword], "guards": [],
            "effects": [{"kind": "draw", "tracks": tracks,
                "candidates": (1..=pool).collect::<Vec<_>>()}],
            "sources": [reference]}],
    })
    .to_string()])
    .map_err(fixture::debug)?;
    let digest = source.programs()[0]
        .digest
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let catalogue = Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            definitions,
            vec![QuestTransition {
                key: format!("oteryn:quest-npc-pin/{digest}"),
                quest: QUEST.into(),
                effects: vec![QuestEffect {
                    track: tracks[0].into(),
                    from: QuestComparison::Any,
                    effect: QuestEffectKind::Computed,
                }],
                completes: false,
                experience: None,
                world_change: None,
            }],
        )
        .map_err(fixture::debug)?,
    );
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(fixture::id(fixture::WORLD).as_slice())
        .bind(fixture::id(fixture::CHANNEL).as_slice())
        .execute(&h.pool)
        .await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(fixture::id(fixture::WORLD).as_slice())
        .bind(fixture::id(fixture::CHANNEL).as_slice())
        .bind([1_u8; 32].as_slice())
        .bind([2_u8; 32].as_slice())
        .bind([3_u8; 32].as_slice())
        .execute(&h.pool)
        .await?;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let permit = SpellLanePermit::of_fresh_scope(fixture::fence(1)?.runtime_scope)
        .await
        .map_err(fixture::debug)?;
    let session = GameSessionId::decode(&fixture::id(fixture::SESSION)).map_err(fixture::debug)?;
    let mut seen = BTreeSet::new();
    for number in 0..requests {
        let fence = fixture::fence(h.root_revision().await?.parse()?)?;
        let request = NpcProgramRequest {
            command: CommandRef::new(
                session,
                CommandId::new(8100 + number).map_err(fixture::debug)?,
            ),
            npc: npc.into(),
            trigger: NpcRuleTrigger::Say,
            message: keyword.into(),
            topic: 0,
            transition_key: rule.into(),
        };
        let invoke = || {
            h.root.commit_fixture_npc_program(
                &authority,
                &h.node,
                &permit,
                fence,
                request.clone(),
                catalogue.clone(),
                &NoItems,
                &source,
            )
        };
        let commit = invoke().await.map_err(fixture::debug)?;
        let QuestTransitionOutcome::Committed(receipt) = commit.transition else {
            return Err(format!("draw commit: {commit:?}").into());
        };
        let state = h
            .root
            .read_character_quest_state(&authority, fence.character_id)
            .await
            .map_err(fixture::debug)?;
        let values = receipt
            .changes
            .iter()
            .map(|change| {
                assert_eq!(state.tracks().get(&change.track), Some(&change.after));
                assert!((1..=pool).contains(&change.after));
                change.after
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(values.len(), tracks.len());
        seen.extend(values);
        let replay = invoke().await.map_err(fixture::debug)?;
        assert_eq!(
            replay.transition,
            QuestTransitionOutcome::AlreadyCommitted(receipt)
        );
    }
    if requests == 13 {
        assert!(seen.len() < usize::try_from(requests)?);
    }
    Ok(())
}
