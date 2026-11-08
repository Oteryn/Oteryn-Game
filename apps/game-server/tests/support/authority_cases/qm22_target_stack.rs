//! M22: the production carried-craft owner captures one locked target stack.
use super::engitem_reward_text::{Weights, command, definition, equip_backpack};
use crate::bestiary_postgres_harness as f;
use crate::durability::item_craft::ItemCraftRequest;
use crate::durability::item_loop_move::ItemLoopContent;
use crate::durability::item_transfer::{ItemDefinitionFacts, ItemWeightLookup, ItemWeightRefused};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::reward_claim_mint::QuestRewardAdmission;
use crate::durability::spell_owner_commit::SpellLanePermit;
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_items::item_ref_admission::generation_item_facts;
use oteryn_gameplay_quests::quest::reward_callbacks::{self, RewardCallback};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTrack};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use std::sync::Arc;

struct Content<'a>(&'a NativeGameplayState, u32);
impl ItemWeightLookup for Content<'_> {
    fn generation_digest(&self) -> [u8; 32] {
        self.0.source_digest()
    }
    fn weight_of(
        &self,
        d: &crate::durability::item_mint::TypedDefinitionRef,
    ) -> Result<u32, ItemWeightRefused> {
        Weights(self.0).weight_of(d)
    }
}
impl ItemLoopContent for Content<'_> {
    fn loop_content_digest(&self) -> [u8; 32] {
        self.0.source_digest()
    }
    fn loop_facts(
        &self,
        d: &crate::durability::item_mint::TypedDefinitionRef,
    ) -> Option<ItemDefinitionFacts> {
        let mut facts = generation_item_facts(self.0, d).ok()?;
        if facts.container_capacity.is_some() {
            facts.container_capacity = Some(self.1);
        }
        Some(facts)
    }
}

#[test]
fn qm22_postgres_whole_target_stack_is_atomic_and_replayed() -> f::TestResult {
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
                    let h = f::Harness::create(admin, "qm22_stack", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "M22_PG_EXECUTED whole_target_stack duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "M22 PG thread failed")?
        .map_err(Into::into)
}

async fn snapshot(h: &f::Harness) -> f::TestResult<serde_json::Value> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_object('items',(SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM game_item_instances i),'entries',(SELECT jsonb_agg(to_jsonb(e) ORDER BY item_instance_id) FROM game_item_container_entries e),'crafts',(SELECT count(*) FROM game_item_craft_records),'audit',(SELECT count(*) FROM game_item_audit_outbox))")
        .fetch_one(&h.pool).await?)
}

async fn exercise(h: &f::Harness) -> f::TestResult {
    let (controller, _) =
        oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(
            f::scope()?.world_id(),
            None,
        );
    let native = controller
        .active()
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(f::id(f::WORLD).as_slice()).bind(f::id(f::CHANNEL).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(f::id(f::WORLD).as_slice()).bind(f::id(f::CHANNEL).as_slice()).bind(native.source_digest().as_slice()).bind([2_u8;32].as_slice()).bind([3_u8;32].as_slice()).execute(&h.pool).await?;
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(f::debug)?;
    let weights = Weights(native);
    equip_backpack(h, &authority, native, &weights).await?;
    for (index, item, count) in [(0, 3736, 2), (1, 10297, 7), (2, 10297, 4)] {
        let track = format!("oteryn:quest-progress/qm22/seed-{index}");
        let callback: RewardCallback = serde_json::from_value(serde_json::json!({
            "key":format!("oteryn:quest-reward/qm22/seed-{index}"), "quest":format!("oteryn:quest.qm22_fixture_{index}"),
            "npc":"oteryn:npc.fixture", "keywords":["yes"], "topic":0,"next_topic":0,
            "storage":[{"track":track,"minimum":0,"maximum":0,"set":1}], "take":[],
            "give":{"item":format!("oteryn:item.tibia.i{item}"),"count":count,"contents":[]},
            "outfit":null,"achievement":null,"reply":["Granted."],"sources":["engine fixture"]
        }))?;
        let catalogue = Arc::new(
            QuestStateCatalogue::new(
                "content-1",
                vec![QuestTrack {
                    key: track,
                    quest: callback.quest.clone(),
                    initial: 0,
                    min: 0,
                    max: 1,
                }],
                vec![reward_callbacks::primary_transition(&callback).map_err(f::debug)?],
            )
            .map_err(f::debug)?,
        );
        let request = QuestTransitionRequest {
            transition_key: callback.key.clone(),
            cause: QuestCause::Command(command(9750 + index)?),
        };
        let outcome = h
            .root
            .commit_fixture_quest_reward(
                &authority,
                &h.node,
                f::fence(1 + index)?,
                request,
                catalogue,
                QuestRewardAdmission {
                    native,
                    weights: &weights,
                    fee_payment: None,
                    actor_conditions: None,
                    carried_use: None,
                    creature_use: None,
                    peer_use: None,
                },
                callback,
            )
            .await
            .map_err(f::debug)?;
        assert!(
            matches!(outcome, QuestTransitionOutcome::Committed(_)),
            "seed {index}: {outcome:?}"
        );
    }
    let source: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(item_instance_id) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3736' AND lifecycle=1").fetch_one(&h.pool).await?;
    let target: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(item_instance_id) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i10297' AND quantity=7 AND lifecycle=1").fetch_one(&h.pool).await?;
    let request = ItemCraftRequest {
        command: command(9790)?,
        source: source.try_into().map_err(|_| "source uuid")?,
        tool: None,
        ingredient: Some(target.try_into().map_err(|_| "target uuid")?),
        station: Some(definition(10297)),
        intent: vec![22],
        gathering: None,
        enchanting: None,
    };
    let lane = SpellLanePermit::of_fresh_scope(f::scope()?)
        .await
        .map_err(f::debug)?;
    let before = snapshot(h).await?;
    let refused = h
        .root
        .craft_item(
            &lane,
            &authority,
            &h.node,
            f::item_fence()?,
            request.clone(),
            native,
            &Content(native, 2),
            false,
        )
        .await?;
    assert_eq!(refused, Some(UseDisposition::NoRoom));
    assert_eq!(snapshot(h).await?, before);
    // A direct caller cannot weaken a captured whole-stack debit to six of seven.
    let mut tx = h.pool.begin().await?;
    let forged = sqlx::query("INSERT INTO game_item_craft_records(transaction_id,event_id,game_session_id,command_id,character_id,world_id,intent,recipe,source_item_instance_id,parent_item_instance_id,source_ordinal,before_item,outputs,occurred_at,envelope_sha256,consume_source,ingredient_item_instance_id,ingredient_before_item,ingredient_consumed_quantity) SELECT game_character_uuid_v7(),game_character_uuid_v7(),encode($1,'hex')::uuid,9789,e.character_id,i.world_id,$2,'oteryn:item-conversion.tainted-antler-talisman',i.item_instance_id,e.parent_item_instance_id,e.placement_ordinal,to_jsonb(i),jsonb_build_array(jsonb_build_object('item',game_character_uuid_v7(),'key','oteryn:item.tibia.i22008','revision','definition-r1','count',1,'ordinal','10000')),1700000000000,decode(repeat('00',32),'hex'),true,x.item_instance_id,to_jsonb(x),6 FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) CROSS JOIN game_item_instances x WHERE i.item_instance_id=encode($3,'hex')::uuid AND x.item_instance_id=encode($4,'hex')::uuid")
        .bind(f::id(f::SESSION).as_slice()).bind([22_u8].as_slice())
        .bind(request.source.as_slice()).bind(request.ingredient.ok_or("ingredient")?.as_slice())
        .execute(&mut *tx).await;
    let error = forged.err().ok_or("partial target debit was admitted")?;
    assert_eq!(
        error.as_database_error().and_then(|e| e.code()).as_deref(),
        Some("23514")
    );
    tx.rollback().await?;
    assert_eq!(snapshot(h).await?, before);
    let result = h
        .root
        .craft_item(
            &lane,
            &authority,
            &h.node,
            f::item_fence()?,
            request.clone(),
            native,
            &Content(native, 3),
            false,
        )
        .await?;
    assert_eq!(result, Some(UseDisposition::Committed));
    let counts: (i64,i64,i64,i64) = sqlx::query_as("SELECT (SELECT quantity FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3736' AND lifecycle=1),(SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i10297' AND quantity=0 AND lifecycle=2),(SELECT quantity FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i10297' AND lifecycle=1),(SELECT quantity FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i22008' AND lifecycle=1)").fetch_one(&h.pool).await?;
    assert_eq!(counts, (1, 1, 4, 1));
    let debit: i64 = sqlx::query_scalar(
        "SELECT ingredient_consumed_quantity FROM game_item_craft_records WHERE command_id=9790",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(debit, 7);
    let committed = snapshot(h).await?;
    let mut replay = request.clone();
    replay.source = [0; 16];
    replay.ingredient = None;
    assert_eq!(
        h.root
            .craft_item(
                &lane,
                &authority,
                &h.node,
                f::item_fence()?,
                replay,
                native,
                &Content(native, 3),
                true
            )
            .await?,
        Some(UseDisposition::Committed)
    );
    let mut changed = request;
    changed.intent = vec![23];
    assert_eq!(
        h.root
            .craft_item(
                &lane,
                &authority,
                &h.node,
                f::item_fence()?,
                changed,
                native,
                &Content(native, 3),
                true
            )
            .await?,
        Some(UseDisposition::Rejected)
    );
    assert_eq!(snapshot(h).await?, committed);
    Ok(())
}
