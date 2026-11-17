//! M41: the existing NPC receipt transaction owns its canonical achievement grant.
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

const QUEST: &str = "oteryn:quest.m41_achievement_fixture";
const TRACK: &str = "oteryn:quest-progress/m41_achievement_fixture/stage";
const RULE: &str = "oteryn:quest-npc/m41_achievement_fixture/finale";
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
fn m41_npc_achievement_is_atomic_canonical_fenced_and_replay_bound() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(64<<20).spawn(move || -> Result<(),String> {
        fixture::runtime().map_err(fixture::debug)?.block_on(async {
            let started=std::time::Instant::now();
            let h=fixture::Harness::create(admin,"m41_npc_achievements",true).await?;
            let result=exercise(&h).await;
            h.cleanup().await?;
            result?;
            assert!(started.elapsed().as_secs_f64()>0.01);
            eprintln!("M41_PG_EXECUTED npc_achievement_atomic_canonical_fenced_replay duration={:?}",started.elapsed());
            Ok::<(),Box<dyn std::error::Error>>(())
        }).map_err(fixture::debug)
    })?.join().map_err(|_|"M41 NPC achievement PG thread failed")?.map_err(Into::into)
}

fn source(name: &str) -> fixture::TestResult<(NpcRuleCatalog, Arc<QuestStateCatalogue>)> {
    let reference = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/eldoran_the_ambitious.lua:90";
    let text=serde_json::json!({"schema":1,"quest":QUEST,
        "tracks":[{"key":TRACK,"quest":QUEST,"initial":0,"min":0,"max":1,"sources":[reference]}],
        "rules":[{"key":RULE,"quest":QUEST,"npc":"oteryn:npc.fixture","trigger":"say","keywords":["yes"],
            "guards":[{"kind":"storage","track":TRACK,"op":"eq","value":0}],
            "effects":[{"kind":"set","track":TRACK,"value":1},{"kind":"achievement","name":name}],
            "reply":["Awarded."],"sources":[reference]}]}).to_string();
    let source = NpcRuleCatalog::from_json_documents(&[&text]).map_err(fixture::debug)?;
    let digest = source.programs()[0]
        .digest
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let catalogue = QuestStateCatalogue::new(
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
    .map_err(fixture::debug)?;
    Ok((source, Arc::new(catalogue)))
}

async fn exercise(h: &fixture::Harness) -> fixture::TestResult {
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
    let request = |id| -> fixture::TestResult<NpcProgramRequest> {
        Ok(NpcProgramRequest {
            command: CommandRef::new(
                GameSessionId::decode(&fixture::id(fixture::SESSION)).map_err(fixture::debug)?,
                CommandId::new(id).map_err(fixture::debug)?,
            ),
            npc: "oteryn:npc.fixture".into(),
            trigger: NpcRuleTrigger::Say,
            message: "yes".into(),
            topic: 0,
            transition_key: RULE.into(),
        })
    };
    let (unknown, unknown_catalogue) = source("M41 nonexistent achievement")?;
    let rejected = h
        .root
        .commit_fixture_npc_program(
            &authority,
            &h.node,
            &permit,
            fence,
            request(9718)?,
            unknown_catalogue,
            &Content,
            &unknown,
        )
        .await;
    assert!(!rejected.as_ref().is_ok_and(|outcome| matches!(
        &outcome.transition,
        QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_)
    )));
    assert_eq!(h.root_revision().await?, "1");
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(count, 0);
    let (source, catalogue) = source("Bastard")?;
    let mut stale = fence;
    stale.character_lease_generation += 1;
    assert!(
        h.root
            .commit_fixture_npc_program(
                &authority,
                &h.node,
                &permit,
                stale,
                request(9719)?,
                catalogue.clone(),
                &Content,
                &source
            )
            .await
            .is_err()
    );
    sqlx::raw_sql("CREATE FUNCTION m41_fail_achievement() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN RAISE EXCEPTION 'M41 forced achievement failure'; END$$; CREATE TRIGGER m41_fail_achievement BEFORE INSERT ON game_account_achievement_grant_requests FOR EACH ROW EXECUTE FUNCTION m41_fail_achievement()")
        .execute(&h.pool).await?;
    let failed = h
        .root
        .commit_fixture_npc_program(
            &authority,
            &h.node,
            &permit,
            fence,
            request(9720)?,
            catalogue.clone(),
            &Content,
            &source,
        )
        .await;
    assert!(
        failed.is_err(),
        "the achievement failure must abort the NPC transaction"
    );
    assert_eq!(h.root_revision().await?, "1");
    let rolled_back: (i64, i64, i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_character_quest_receipts), \
         (SELECT count(*) FROM game_quest_npc_program_receipts), \
         (SELECT count(*) FROM game_character_quest_states), \
         (SELECT count(*) FROM game_account_achievement_grant_requests), \
         (SELECT count(*) FROM game_account_achievements)",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        rolled_back,
        (0, 0, 0, 0, 0),
        "every NPC/quest/achievement row must roll back"
    );
    sqlx::raw_sql("DROP TRIGGER m41_fail_achievement ON game_account_achievement_grant_requests; DROP FUNCTION m41_fail_achievement()")
        .execute(&h.pool).await?;
    let committed = h
        .root
        .commit_fixture_npc_program(
            &authority,
            &h.node,
            &permit,
            fence,
            request(9720)?,
            catalogue.clone(),
            &Content,
            &source,
        )
        .await
        .map_err(fixture::debug)?;
    assert!(
        matches!(committed.transition, QuestTransitionOutcome::Committed(_)),
        "{committed:?}"
    );
    assert_eq!(h.root_revision().await?, "2");
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/bastard'").fetch_one(&h.pool).await?;
    assert_eq!(count, 1);
    let same:bool=sqlx::query_scalar("SELECT a.xmin=n.xmin AND a.xmin=q.xmin FROM game_account_achievement_grant_requests a JOIN game_quest_npc_program_receipts n USING(character_id) JOIN game_character_quest_receipts q USING(character_id) WHERE a.achievement_key='oteryn:achievement/bastard' AND n.command_id=9720 AND q.cause_ordinal=9720")
        .fetch_one(&h.pool).await?;
    assert!(
        same,
        "achievement, quest progress and NPC reply must share one transaction"
    );
    let replay = h
        .root
        .commit_fixture_npc_program(
            &authority,
            &h.node,
            &permit,
            fence,
            request(9720)?,
            catalogue,
            &Content,
            &source,
        )
        .await
        .map_err(fixture::debug)?;
    assert!(matches!(
        replay.transition,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    let mut changed = request(9720)?;
    changed.message = "different".into();
    assert!(
        h.root
            .reconcile_npc_program(&authority, fence.character_id, changed)
            .await
            .is_err()
    );
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_account_achievement_grant_requests")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(count, 1);
    assert_eq!(h.root_revision().await?, "2");
    Ok(())
}
