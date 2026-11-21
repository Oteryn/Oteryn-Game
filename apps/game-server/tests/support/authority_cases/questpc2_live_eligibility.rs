//! Live eligibility reads current Character facts in the NPC receipt transaction.
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
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.eligibility_fixture";
const TRACK: &str = "oteryn:quest-progress/eligibility_fixture/stage";
const RULE: &str = "oteryn:quest-npc/eligibility_fixture/accept";

struct Content(bool);
impl ItemWeightLookup for Content {
    fn wheel_premium_current(&self, account: [u8; 16]) -> bool {
        self.0 && account == fixture::id(40)
    }
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
fn questpc2_live_level_vocation_premium_are_fenced_atomic_and_replay_bound() -> fixture::TestResult
{
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
                    let h =
                        fixture::Harness::create(admin, "questpc2_live_eligibility", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "M08_PG_EXECUTED live_eligibility duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "equipped PG thread failed")?
        .map_err(Into::into)
}

async fn exercise(h: &fixture::Harness) -> fixture::TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("M08_POSTGRES_VERSION={version}");
    let reference = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data/npclib/npc.lua:13";
    let text = serde_json::json!({"schema":1,"quest":QUEST,
        "tracks":[{"key":TRACK,"quest":QUEST,"initial":0,"min":0,"max":1,"sources":[reference]}],
        "rules":[{"key":RULE,"quest":QUEST,"npc":"oteryn:npc.fixture","trigger":"say",
            "keywords":["mission"],"guards":[{"kind":"level","op":"ge","value":50}, {"kind":"vocation","value":"oteryn:vocation.none"}, {"kind":"premium","value":true},
                {"kind":"storage","track":TRACK,"op":"eq","value":0}],
            "effects":[{"kind":"set","track":TRACK,"value":1}],"reply":["Accepted."],"sources":[reference]}]
    }).to_string();
    let source = NpcRuleCatalog::from_json_documents(&[&text]).map_err(fixture::debug)?;
    let digest = source.programs()[0]
        .digest
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let catalog = Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![QuestTrack {
                key: TRACK.into(),
                quest: QUEST.into(),
                initial: 0,
                min: 0,
                max: 1,
            }],
            vec![QuestTransition {
                key: format!("oteryn:quest-npc-pin/{digest}"),
                quest: QUEST.into(),
                effects: vec![QuestEffect {
                    track: TRACK.into(),
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
    let request = |ordinal| -> fixture::TestResult<NpcProgramRequest> {
        Ok(NpcProgramRequest {
            command: CommandRef::new(
                GameSessionId::decode(&fixture::id(fixture::SESSION)).map_err(fixture::debug)?,
                CommandId::new(ordinal).map_err(fixture::debug)?,
            ),
            npc: "oteryn:npc.fixture".into(),
            trigger: NpcRuleTrigger::Say,
            message: "mission".into(),
            topic: 0,
            transition_key: RULE.into(),
        })
    };
    let wrong = h
        .root
        .commit_fixture_npc_program(
            &authority,
            &h.node,
            &permit,
            fence,
            request(9500)?,
            catalog.clone(),
            &Content(false),
            &source,
        )
        .await
        .map_err(fixture::debug)?;
    assert!(matches!(
        wrong.transition,
        QuestTransitionOutcome::Refused(_)
    ));
    assert_eq!(h.root_revision().await?, "1");
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(count, 0);
    // Historical progression setup only; tested commands retain all triggers.
    for level in [49_i64, 50] {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE game_character_progression_state SET level=$1")
            .bind(level)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        if level == 49 {
            let refused = h
                .root
                .commit_fixture_npc_program(
                    &authority,
                    &h.node,
                    &permit,
                    fence,
                    request(9503)?,
                    catalog.clone(),
                    &Content(true),
                    &source,
                )
                .await
                .map_err(fixture::debug)?;
            assert!(matches!(
                refused.transition,
                QuestTransitionOutcome::Refused(_)
            ));
            assert_eq!(h.root_revision().await?, "1");
        }
    }
    let mut stale = fence;
    stale.character_lease_generation += 1;
    assert!(
        h.root
            .commit_fixture_npc_program(
                &authority,
                &h.node,
                &permit,
                stale,
                request(9501)?,
                catalog.clone(),
                &Content(true),
                &source
            )
            .await
            .is_err()
    );
    let good = h
        .root
        .commit_fixture_npc_program(
            &authority,
            &h.node,
            &permit,
            fence,
            request(9502)?,
            catalog.clone(),
            &Content(true),
            &source,
        )
        .await
        .map_err(fixture::debug)?;
    let QuestTransitionOutcome::Committed(receipt) = good.transition else {
        return Err(format!("live eligibility admission: {good:?}").into());
    };
    assert_eq!(receipt.changes.len(), 1);
    assert_eq!(receipt.changes[0].after, 1);
    assert!(!good.inventory_changed);
    assert_eq!(h.root_revision().await?, "2");
    let replay = h
        .root
        .commit_fixture_npc_program(
            &authority,
            &h.node,
            &permit,
            fence,
            request(9502)?,
            catalog,
            &Content(false),
            &source,
        )
        .await
        .map_err(fixture::debug)?;
    assert!(matches!(
        replay.transition,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    assert_eq!(h.root_revision().await?, "2");
    let mut changed = request(9502)?;
    changed.message = "different".into();
    assert!(
        h.root
            .reconcile_npc_program(&authority, fence.character_id, changed)
            .await
            .is_err()
    );
    Ok(())
}
