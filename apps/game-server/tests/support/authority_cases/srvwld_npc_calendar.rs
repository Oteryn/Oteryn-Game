//! Tamoril's seasonal source guard consumes PostgreSQL UTC time and the real NPC owner.
use crate::bestiary_postgres_harness as f;
use crate::durability::character_progression::CharacterProgressionError;
use crate::durability::item_loop_move::ItemLoopContent;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{ItemDefinitionFacts, ItemWeightLookup, ItemWeightRefused};
use crate::durability::quest_state::{QuestTransitionOutcome, npc_program::NpcProgramRequest};
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_quests::quest::npc_rules::{NpcInventorySnapshot, NpcRuleCatalog, NpcRuleInput, NpcRulePlan, NpcRuleTrigger};
use oteryn_gameplay_quests::quest::{QuestComparison, QuestEffect, QuestEffectKind, QuestRefusal, QuestStateCatalogue, QuestTrack, QuestTransition};
use std::collections::BTreeMap;
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.the_first_dragon_quest";
const NPC: &str = "oteryn:npc.tamoril";

#[test]
fn srvwld_npc_calendar_postgres_utc_boundaries_and_source_resets() -> f::TestResult {
    run("utc_boundaries", false)
}
#[test]
fn srvwld_npc_calendar_postgres_current_clock_and_stale_fence() -> f::TestResult {
    run("current_clock", true)
}
fn run(name: &'static str, durable: bool) -> f::TestResult {
    let Some(admin) = f::configured_admin() else { return Ok(()); };
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || -> Result<(), String> {
        f::runtime().map_err(f::debug)?.block_on(async {
            let started = std::time::Instant::now();
            let h = f::Harness::create(admin, &format!("srvwld_calendar_{name}"), true).await?;
            let result = if durable { current_clock(&h).await } else { boundaries(&h).await };
            h.cleanup().await?;
            result?;
            assert!(started.elapsed().as_secs_f64() > 0.01);
            eprintln!("SRVWLD_PG_EXECUTED npc_calendar case={name} duration={:?}", started.elapsed());
            Ok::<(), Box<dyn std::error::Error>>(())
        }).map_err(f::debug)
    })?.join().map_err(|_| "SRVWLD calendar PG thread failed")?.map_err(Into::into)
}

// This dialogue has no item operations. Any accidental inventory participant
// fails closed instead of borrowing unrelated fixture content authority.
struct NoItems;
impl ItemWeightLookup for NoItems {
    fn generation_digest(&self) -> [u8; 32] { [0; 32] }
    fn weight_of(&self, _: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> { Err(ItemWeightRefused::Unknown) }
}
impl ItemLoopContent for NoItems {
    fn loop_content_digest(&self) -> [u8; 32] { [0; 32] }
    fn loop_facts(&self, _: &TypedDefinitionRef) -> Option<ItemDefinitionFacts> { None }
}

fn plan(epoch: i64) -> f::TestResult<NpcRulePlan> {
    let runtime = NpcRuleCatalog::runtime().map_err(f::debug)?;
    let programme = runtime.program(QUEST).ok_or("Tamoril source programme")?;
    let source = NpcRuleCatalog::from_json_documents(&[&serde_json::to_string(&programme.document)?]).map_err(f::debug)?;
    let tracks = programme.document.tracks.iter().map(|t| (t.key.clone(), t.initial)).collect();
    let inventory = NpcInventorySnapshot::default();
    let outfits = BTreeMap::new();
    source.plan_storage(&NpcRuleInput {
        npc: NPC, trigger: NpcRuleTrigger::Say, message: "yes", topic: 6,
        tracks: &tracks, inventory: &inventory, now_epoch_seconds: epoch,
        // Intentionally wrong cached month: the annual guard must use UTC epoch.
        calendar_month: 8, level: 100, vocation: "oteryn:vocation.knight", sex: 0, outfits: &outfits,
    }).map_err(f::debug)?.ok_or_else(|| "seasonal source plan".into())
}

async fn boundaries(h: &f::Harness) -> f::TestResult {
    for (date, inside) in [
        ("2024-01-13 23:59:59+00", false), ("2024-01-14 00:00:00+00", true),
        ("2024-02-12 23:59:59+00", true), ("2024-02-13 00:00:00+00", false),
        ("2024-02-13 13:59:59+14", true), ("2025-01-14 00:00:00+00", true),
    ] {
        let epoch: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM $1::timestamptz))::bigint")
            .bind(date).fetch_one(&h.pool).await?;
        let result = plan(epoch)?;
        assert_eq!(!result.track_changes.is_empty(), inside, "date={date}");
        if inside {
            assert_eq!(result.track_changes.len(), 5);
            assert_eq!(result.next_topic, 0);
            for suffix in ["dragon_counter", "gelidrazah_access", "secrets_counter"] {
                assert!(result.track_changes.iter().any(|c| c.key.ends_with(suffix) && c.after == 0));
            }
        } else {
            assert!(result.effects.is_empty());
        }
    }
    Ok(())
}

async fn current_clock(h: &f::Harness) -> f::TestResult {
    let runtime = NpcRuleCatalog::runtime().map_err(f::debug)?;
    let programme = runtime.program(QUEST).ok_or("Tamoril source programme")?;
    let tracks = programme.document.tracks.iter().map(|t| QuestTrack {
        key: t.key.clone(), quest: t.quest.clone(), initial: t.initial, min: t.min, max: t.max,
    }).collect::<Vec<_>>();
    let digest = programme.digest.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let marker = QuestTransition {
        key: format!("oteryn:quest-npc-pin/{digest}"), quest: QUEST.into(),
        effects: vec![QuestEffect { track: tracks.first().ok_or("programme track")?.key.clone(),
            from: QuestComparison::Any, effect: QuestEffectKind::Computed }],
        completes: false, experience: None, world_change: None,
    };
    let catalogue = Arc::new(QuestStateCatalogue::new("content-1", tracks, vec![marker]).map_err(f::debug)?);
    assert!(runtime.admitted_for_npc(NPC, &catalogue).map_err(f::debug)?.is_some());
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let authority = h.root.open_character_authority(&seal).await.map_err(f::debug)?;
    let fence = f::fence(1)?;
    let permit = SpellLanePermit::of_fresh_scope(fence.runtime_scope).await.map_err(f::debug)?;
    let session = GameSessionId::decode(&f::id(f::SESSION)).map_err(f::debug)?;
    // Forge an eligible preview date; the production commit must re-evaluate
    // using PostgreSQL transaction time, not the preview or caller's topic.
    let eligible = plan(1_705_190_400)?;
    let request = |ordinal| -> f::TestResult<NpcProgramRequest> { Ok(NpcProgramRequest {
        command: CommandRef::new(session, CommandId::new(ordinal).map_err(f::debug)?),
        npc: NPC.into(), trigger: NpcRuleTrigger::Say, message: "yes".into(), topic: 6,
        transition_key: eligible.selected_rule_keys.first().ok_or("acceptance rule")?.clone(),
    }) };
    let before = h.root.read_character_quest_state(&authority, fence.character_id).await.map_err(f::debug)?;
    let mut stale = fence;
    stale.character_lease_generation += 1;
    assert!(matches!(h.root.commit_character_npc_program(&authority, &h.node, &permit, stale,
        request(9100)?, catalogue.clone(), &NoItems).await, Err(CharacterProgressionError::AuthorityRejected)));
    assert_eq!(before, h.root.read_character_quest_state(&authority, fence.character_id).await.map_err(f::debug)?);
    let epoch: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM transaction_timestamp()))::bigint")
        .fetch_one(&h.pool).await?;
    let expected = plan(epoch)?;
    let outcome = h.root.commit_character_npc_program(&authority, &h.node, &permit, fence,
        request(9101)?, catalogue, &NoItems).await.map_err(f::debug)?;
    if expected.track_changes.is_empty() {
        assert_eq!(outcome.transition, QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch));
        assert_eq!(before, h.root.read_character_quest_state(&authority, fence.character_id).await.map_err(f::debug)?);
        let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_npc_program_receipts").fetch_one(&h.pool).await?;
        assert_eq!(receipts, 0);
    } else {
        assert!(matches!(outcome.transition, QuestTransitionOutcome::Committed(_)));
        let replay = h.root.reconcile_npc_program(&authority, fence.character_id, request(9101)?).await
            .map_err(f::debug)?.ok_or("seasonal receipt")?;
        assert!(matches!(replay.transition, QuestTransitionOutcome::AlreadyCommitted(_)));
        assert_eq!(outcome.reply, replay.reply);
    }
    Ok(())
}
