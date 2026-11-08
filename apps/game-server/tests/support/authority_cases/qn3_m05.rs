//! M05: item, point and transaction-clock writes share the fenced source-NPC receipt.
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

const QUEST: &str = "oteryn:quest.clock_fixture";
const TRACK: &str = "oteryn:quest-progress/clock_fixture/stage";
const POINT: &str = "oteryn:quest-progress/clock_fixture/points";
const RULE: &str = "oteryn:quest-npc/clock_fixture/accept";

struct Content(bool);
impl ItemWeightLookup for Content {
    fn wheel_premium_current(&self, account: [u8; 16]) -> bool {
        self.0 && account == fixture::id(40)
    }
    fn generation_digest(&self) -> [u8; 32] {
        [1; 32]
    }
    fn weight_of(&self, _: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
        Ok(1)
    }
}
impl ItemLoopContent for Content {
    fn loop_content_digest(&self) -> [u8; 32] {
        [1; 32]
    }
    fn loop_facts(&self, definition: &TypedDefinitionRef) -> Option<ItemDefinitionFacts> {
        if definition.production_key == "fixture:b3.backpack" {
            Some(crate::item_transfer_postgres_cases::backpack_facts())
        } else {
            Some(crate::item_transfer_postgres_cases::facts(
                &definition.production_key,
                crate::durability::item_transfer::ItemStackClass::NonStackable,
            ))
        }
    }
}

#[test]
fn qn3_m05_item_points_and_clock_commit_together() -> fixture::TestResult {
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
                    let h = fixture::Harness::create(admin, "qn3_m05", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "M05_PG_EXECUTED item_points_clock duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "M05 PG thread failed")?
        .map_err(Into::into)
}

async fn exercise(h: &fixture::Harness) -> fixture::TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("M05_POSTGRES_VERSION={version}");
    let reference = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data/npclib/npc.lua:13";
    let text = serde_json::json!({"schema":1,"quest":QUEST,
        "tracks":[{"key":TRACK,"quest":QUEST,"initial":0,"min":0,"max":4102444800_i64,"sources":[reference]},{"key":POINT,"quest":QUEST,"initial":0,"min":0,"max":100,"sources":[reference]}],
        "rules":[{"key":RULE,"quest":QUEST,"npc":"oteryn:npc.fixture","trigger":"say",
            "keywords":["mission"],"guards":[{"kind":"storage_time","track":TRACK,"op":"le"}],
            "effects":[{"kind":"add","track":POINT,"value":1},{"kind":"set_now_plus","track":TRACK,"value":72000},{"kind":"give_item","item":"oteryn:item.tibia.i27654","count":1}],"reply":["Accepted."],"sources":[reference]}]
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
            vec![
                QuestTrack {
                    key: TRACK.into(),
                    quest: QUEST.into(),
                    initial: 0,
                    min: 0,
                    max: 4_102_444_800,
                },
                QuestTrack {
                    key: POINT.into(),
                    quest: QUEST.into(),
                    initial: 0,
                    min: 0,
                    max: 100,
                },
            ],
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
    let no_room = h
        .root
        .commit_fixture_npc_program(
            &authority,
            &h.node,
            &permit,
            fence,
            request(9500)?,
            catalog.clone(),
            &Content(true),
            &source,
        )
        .await
        .map_err(fixture::debug)?;
    assert!(matches!(
        no_room.transition,
        QuestTransitionOutcome::Refused(_)
    ));
    assert_eq!(h.root_revision().await?, "1");
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(receipts, 0);
    // Seed a historical backpack; all triggers stay enabled during the actual command.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','fixture:b3.backpack','definition-r1',1,1,encode($3,'hex')::uuid)")
        .bind(fixture::id(91).as_slice()).bind(fixture::id(fixture::WORLD).as_slice()).bind(fixture::id(92).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_slots VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
        .bind(fixture::id(fixture::CHARACTER).as_slice()).bind(fixture::id(91).as_slice()).bind(fixture::id(fixture::WORLD).as_slice()).bind(fixture::id(93).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    let before: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM now()))::bigint")
        .fetch_one(&h.pool)
        .await?;
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
        return Err(format!("clock/item admission: {good:?}").into());
    };
    let after: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM now()))::bigint")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(receipt.changes.len(), 2);
    assert_eq!(
        receipt
            .changes
            .iter()
            .find(|c| c.track == POINT)
            .ok_or("missing point change")?
            .after,
        1
    );
    assert!(
        (before + 72000..=after + 72000).contains(
            &receipt
                .changes
                .iter()
                .find(|c| c.track == TRACK)
                .ok_or("missing clock change")?
                .after
        )
    );
    let minted: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_quest_npc_item_sources WHERE mint_transaction_id IS NOT NULL",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(minted, 1);
    assert!(good.inventory_changed);
    assert_eq!(h.root_revision().await?, "2");
    let cooldown = h
        .root
        .commit_fixture_npc_program(
            &authority,
            &h.node,
            &permit,
            fixture::fence(2)?,
            request(9503)?,
            catalog.clone(),
            &Content(true),
            &source,
        )
        .await
        .map_err(fixture::debug)?;
    assert!(matches!(
        cooldown.transition,
        QuestTransitionOutcome::Refused(_)
    ));
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
