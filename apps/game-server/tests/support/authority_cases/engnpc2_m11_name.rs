//! M11 admits the real current Character name in the source NPC receipt transaction.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::item_loop_move::ItemLoopContent;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{ItemDefinitionFacts, ItemWeightLookup, ItemWeightRefused};
use crate::durability::quest_state::{QuestTransitionOutcome, npc_program::NpcProgramRequest};
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_quests::quest::{
    loader::load_embedded_quest_state,
    npc_rules::{NpcRuleCatalog, NpcRuleTrigger},
};
use std::sync::Arc;

struct Content;
impl ItemWeightLookup for Content {
    fn generation_digest(&self) -> [u8; 32] {
        [1; 32]
    }
    fn weight_of(&self, _: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
        Err(ItemWeightRefused::Unknown)
    }
}
impl ItemLoopContent for Content {
    fn loop_content_digest(&self) -> [u8; 32] {
        [1; 32]
    }
    fn loop_facts(&self, _: &TypedDefinitionRef) -> Option<ItemDefinitionFacts> {
        None
    }
}

#[test]
fn engnpc2_m11_current_name_is_fenced_and_replay_bound() -> fixture::TestResult {
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
                    let h = fixture::Harness::create(admin, "engnpc2_m11", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "ENGNPC2_PG_EXECUTED m11_current_name duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "M11 PG thread failed")?
        .map_err(Into::into)
}

const TRACK: &str = "oteryn:quest-progress/quest/u12_00/the_dream_courts/haunted_house/questline";
const RULE: &str = "oteryn:quest-npc/the_dream_courts/stricken_soul/name_grant";

async fn exercise(h: &fixture::Harness) -> fixture::TestResult {
    let source = NpcRuleCatalog::runtime().map_err(fixture::debug)?;
    let lowered = load_embedded_quest_state("content-1").map_err(fixture::debug)?;
    let catalog = Arc::new(lowered.catalogue().clone());
    assert!(
        source
            .admitted_for_npc("oteryn:npc.stricken_soul", &catalog)
            .map_err(fixture::debug)?
            .is_some()
    );
    assert_eq!(
        catalog.track(TRACK).ok_or("missing native track")?.initial,
        0
    );
    assert_eq!(catalog.track(TRACK).ok_or("missing native track")?.max, 6);
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(fixture::id(fixture::WORLD).as_slice()).bind(fixture::id(fixture::CHANNEL).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(fixture::id(fixture::WORLD).as_slice()).bind(fixture::id(fixture::CHANNEL).as_slice())
        .bind([1_u8;32].as_slice()).bind([2_u8;32].as_slice()).bind([3_u8;32].as_slice()).execute(&h.pool).await?;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let fence = fixture::fence(1)?;
    let permit = SpellLanePermit::of_fresh_scope(fence.runtime_scope)
        .await
        .map_err(fixture::debug)?;
    let request = |ordinal, message: &str, topic| -> fixture::TestResult<NpcProgramRequest> {
        Ok(NpcProgramRequest {
            command: CommandRef::new(
                GameSessionId::decode(&fixture::id(fixture::SESSION)).map_err(fixture::debug)?,
                CommandId::new(ordinal).map_err(fixture::debug)?,
            ),
            npc: "oteryn:npc.stricken_soul".into(),
            trigger: NpcRuleTrigger::Say,
            message: message.into(),
            topic,
            transition_key: RULE.into(),
        })
    };
    // No preseeded quest state: the two source confirmations only change dialogue topics.
    let tracks = std::collections::BTreeMap::new();
    let mut topic = 0;
    for expected in [2, 3] {
        let ask = request(9600, "yes", topic)?;
        let plan = crate::durability::quest_state::npc_program::preview_with_player_name(
            &ask,
            &tracks,
            &catalog,
            0,
            &crate::durability::quest_state::npc_program::NpcEligibility::default(),
            Some("Fixture Hero"),
        )
        .map_err(fixture::debug)?
        .ok_or("missing yes branch")?;
        topic = plan.next_topic;
        assert_eq!(topic, expected);
        assert!(plan.track_changes.is_empty());
    }
    // A forged/stale preview name can select a branch, but cannot authorize its durable write.
    let forged = request(9601, "Another Hero", 3)?;
    let forged_plan = crate::durability::quest_state::npc_program::preview_with_player_name(
        &forged,
        &tracks,
        &catalog,
        0,
        &crate::durability::quest_state::npc_program::NpcEligibility::default(),
        Some("Another Hero"),
    )
    .map_err(fixture::debug)?
    .ok_or("missing forged preview")?;
    assert_eq!(forged_plan.selected_rule_keys, [RULE]);
    for bad in [
        forged,
        request(9602, "Fixture Hero", 2)?,
        request(9603, "yes Fixture Hero", 3)?,
    ] {
        let refused = h
            .root
            .commit_character_npc_program(
                &authority,
                &h.node,
                &permit,
                fence,
                bad,
                catalog.clone(),
                &Content,
                &crate::durability::quest_state::grizzly::NoGrizzlyPremium,
            )
            .await
            .map_err(fixture::debug)?;
        assert!(matches!(
            refused.transition,
            QuestTransitionOutcome::Refused(_)
        ));
    }
    assert_eq!(h.root_revision().await?, "1");
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(count, 0);
    let mut stale = fence;
    stale.character_lease_generation += 1;
    assert!(
        h.root
            .commit_character_npc_program(
                &authority,
                &h.node,
                &permit,
                stale,
                request(9604, "Fixture Hero", topic)?,
                catalog.clone(),
                &Content,
                &crate::durability::quest_state::grizzly::NoGrizzlyPremium
            )
            .await
            .is_err()
    );
    let good = request(9605, "My name is FIXTURE HERO", topic)?;
    let committed = h
        .root
        .commit_character_npc_program(
            &authority,
            &h.node,
            &permit,
            fence,
            good.clone(),
            catalog.clone(),
            &Content,
            &crate::durability::quest_state::grizzly::NoGrizzlyPremium,
        )
        .await
        .map_err(fixture::debug)?;
    let QuestTransitionOutcome::Committed(receipt) = committed.transition else {
        return Err(format!("M11 commit: {committed:?}").into());
    };
    assert_eq!(receipt.changes.len(), 1);
    assert_eq!(receipt.changes[0].track, TRACK);
    assert_eq!(receipt.changes[0].after, 1);
    assert_eq!(h.root_revision().await?, "2");
    let replay = h
        .root
        .commit_character_npc_program(
            &authority,
            &h.node,
            &permit,
            fence,
            good.clone(),
            catalog,
            &Content,
            &crate::durability::quest_state::grizzly::NoGrizzlyPremium,
        )
        .await
        .map_err(fixture::debug)?;
    assert!(matches!(
        replay.transition,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    assert_eq!(h.root_revision().await?, "2");
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(count, 1);
    let mut changed = good;
    changed.message = "Another Hero".into();
    assert!(
        h.root
            .reconcile_npc_program(&authority, fence.character_id, changed)
            .await
            .is_err()
    );
    Ok(())
}
