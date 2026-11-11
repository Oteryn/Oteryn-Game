//! Source scopes share a canonical quest without sharing completion receipts.
use crate::bestiary_postgres_harness as f;
use crate::durability::item_loop_move::ItemLoopContent;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{ItemDefinitionFacts, ItemWeightLookup, ItemWeightRefused};
use crate::durability::quest_state::{QuestTransitionOutcome, npc_program::NpcProgramRequest};
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestRefusal, QuestStateCatalogue, QuestTrack,
    QuestTransition,
    npc_rules::{NpcRuleCatalog, NpcRuleTrigger},
};
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.outfit_and_addon_quests";
const MARKER: &str = "oteryn:quest-progress/source-completion/outfit_and_addon_quests/citizen-";
struct NoItems;
impl ItemWeightLookup for NoItems {
    fn generation_digest(&self) -> [u8; 32] {
        [0; 32]
    }
    fn weight_of(&self, _: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
        Err(ItemWeightRefused::Unknown)
    }
}
impl ItemLoopContent for NoItems {
    fn loop_content_digest(&self) -> [u8; 32] {
        [0; 32]
    }
    fn loop_facts(&self, _: &TypedDefinitionRef) -> Option<ItemDefinitionFacts> {
        None
    }
}

#[test]
fn qm54_source_sagas_persist_replay_and_isolate_aliases() -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            f::runtime()
                .map_err(f::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = f::Harness::create(admin, "qm54_source_completion", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "M54_PG_EXECUTED source_sagas duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "M54 source completion thread failed")?
        .map_err(Into::into)
}

async fn exercise(h: &f::Harness) -> f::TestResult {
    let runtime = NpcRuleCatalog::runtime().map_err(f::debug)?;
    let source = runtime
        .programs()
        .iter()
        .find(|p| {
            p.document.program_key.as_deref()
                == Some("oteryn:quest-npc-program.citizen-source-sagas")
        })
        .ok_or("Citizen source programme")?;
    let mut tracks: Vec<_> = source
        .document
        .tracks
        .iter()
        .map(|t| QuestTrack {
            key: t.key.clone(),
            quest: t.quest.clone(),
            initial: t.initial,
            min: t.min,
            max: t.max,
        })
        .collect();
    // Model already committed source progress through catalogue defaults. The production
    // NPC writer still reads and validates every prerequisite through its native owner.
    for (suffix, owner, value) in [
        ("citizen_outfits/addon_backpack", QUEST, 3),
        ("citizen_outfits/addon_hat", QUEST, 2),
        (
            "citizen_outfits_rook/addon_backpack_rook",
            "oteryn:quest.citizen_outfits_quest_on_rook",
            3,
        ),
        (
            "citizen_outfits_rook/addon_hat_rook",
            "oteryn:quest.citizen_outfits_quest_on_rook",
            2,
        ),
    ] {
        tracks.push(QuestTrack {
            key: format!("oteryn:quest-progress/quest/u7_8/{suffix}"),
            quest: owner.into(),
            initial: value,
            min: 0,
            max: value,
        });
    }
    let digest = source
        .digest
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let binding = QuestTransition {
        key: format!("oteryn:quest-npc-pin/{digest}"),
        quest: QUEST.into(),
        effects: vec![QuestEffect {
            track: format!("{MARKER}mainland"),
            from: QuestComparison::Any,
            effect: QuestEffectKind::Computed,
        }],
        completes: false,
        experience: None,
        world_change: None,
    };
    let catalogue =
        Arc::new(QuestStateCatalogue::new("content-1", tracks, vec![binding]).map_err(f::debug)?);
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(f::debug)?;
    let fence = f::fence(1)?;
    let permit = SpellLanePermit::of_fresh_scope(fence.runtime_scope)
        .await
        .map_err(f::debug)?;
    let session = GameSessionId::decode(&f::id(f::SESSION)).map_err(f::debug)?;
    let request = |npc: &str, ordinal| -> f::TestResult<NpcProgramRequest> {
        Ok(NpcProgramRequest {
            command: CommandRef::new(session, CommandId::new(ordinal).map_err(f::debug)?),
            npc: format!("oteryn:npc.{npc}"),
            trigger: NpcRuleTrigger::Say,
            message: "addon".into(),
            topic: 0,
            transition_key: format!("oteryn:quest-npc/citizen-source/{npc}"),
        })
    };
    let mut stale = fence;
    stale.character_lease_generation += 1;
    assert!(
        h.root
            .commit_character_npc_program(
                &authority,
                &h.node,
                &permit,
                stale,
                request("lubo", 9850)?,
                catalogue.clone(),
                &NoItems,
                &crate::durability::quest_state::grizzly::NoGrizzlyPremium
            )
            .await
            .is_err()
    );
    assert_eq!(h.root_revision().await?, "1");
    let first = h
        .root
        .commit_character_npc_program(
            &authority,
            &h.node,
            &permit,
            fence,
            request("lubo", 9851)?,
            catalogue.clone(),
            &NoItems,
            &crate::durability::quest_state::grizzly::NoGrizzlyPremium,
        )
        .await
        .map_err(f::debug)?;
    assert!(
        matches!(first.transition, QuestTransitionOutcome::Committed(_)),
        "{first:?}"
    );
    let replay = h
        .root
        .commit_character_npc_program(
            &authority,
            &h.node,
            &permit,
            fence,
            request("lubo", 9851)?,
            catalogue.clone(),
            &NoItems,
            &crate::durability::quest_state::grizzly::NoGrizzlyPremium,
        )
        .await
        .map_err(f::debug)?;
    assert!(matches!(
        replay.transition,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    let alias = h
        .root
        .commit_character_npc_program(
            &authority,
            &h.node,
            &permit,
            f::fence(2)?,
            request("hanna", 9852)?,
            catalogue.clone(),
            &NoItems,
            &crate::durability::quest_state::grizzly::NoGrizzlyPremium,
        )
        .await
        .map_err(f::debug)?;
    assert_eq!(
        alias.transition,
        QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
    );
    assert_eq!(h.root_revision().await?, "2");
    let rook = h
        .root
        .commit_character_npc_program(
            &authority,
            &h.node,
            &permit,
            f::fence(2)?,
            request("amber", 9853)?,
            catalogue,
            &NoItems,
            &crate::durability::quest_state::grizzly::NoGrizzlyPremium,
        )
        .await
        .map_err(f::debug)?;
    assert!(
        matches!(rook.transition, QuestTransitionOutcome::Committed(_)),
        "{rook:?}"
    );
    let markers: Vec<(String, i64)> = sqlx::query_as("SELECT track_key,value FROM game_character_quest_tracks WHERE quest_key=$1 ORDER BY track_key LIMIT 3").bind(QUEST).fetch_all(&h.pool).await?;
    assert_eq!(
        markers,
        vec![
            (format!("{MARKER}mainland"), 1),
            (format!("{MARKER}rook"), 1)
        ]
    );
    let completed: bool = sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE quest_key=$1").bind(QUEST).fetch_one(&h.pool).await?;
    assert!(!completed);
    let receipts: (i64, bool) = sqlx::query_as(
        "SELECT count(*),bool_or(completes) FROM game_character_quest_receipts WHERE quest_key=$1",
    )
    .bind(QUEST)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(receipts, (2, false));
    Ok(())
}
