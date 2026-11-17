//! Real NPC writer qualification without mounting dispatch's entire test binary.
use crate::bestiary_postgres_harness as f;
use crate::durability::item_loop_move::ItemLoopContent;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{ItemDefinitionFacts, ItemWeightLookup, ItemWeightRefused};
use crate::durability::quest_state::{QuestTransitionOutcome, npc_program::NpcProgramRequest};
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_dispatch::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator;
use oteryn_gameplay_quests::quest::{
    QuestRefusal,
    npc_rules::{NpcRuleCatalog, NpcRuleTrigger, random_outcome::SANDRA_LOTTERY},
};
use std::sync::Arc;

struct Content<'a> {
    native: &'a NativeGameplayState,
    premium: &'a SpellPremiumCoordinator,
}

impl ItemWeightLookup for Content<'_> {
    fn generation_digest(&self) -> [u8; 32] {
        self.native.source_digest()
    }

    fn weight_of(&self, definition: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
        self.loop_facts(definition)
            .and_then(|facts| facts.weight)
            .ok_or(ItemWeightRefused::Unknown)
    }

    fn wheel_premium_current(&self, account: [u8; 16]) -> bool {
        self.premium.view(account).premium_gate().premium_current()
    }
}

impl ItemLoopContent for Content<'_> {
    fn loop_content_digest(&self) -> [u8; 32] {
        self.native.source_digest()
    }

    fn loop_facts(&self, definition: &TypedDefinitionRef) -> Option<ItemDefinitionFacts> {
        oteryn_gameplay_items::item_ref_admission::generation_item_facts(self.native, definition)
            .ok()
    }

    fn npc_outfit_content(&self) -> Option<&NativeGameplayState> {
        Some(self.native)
    }
}

#[derive(Clone, Copy)]
enum Case {
    Win,
    Blank,
    Missing,
    Full,
    Replay,
    Odds,
}

#[test]
fn sandra_lottery_pg_seeded_win() -> f::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, "lottery_win", Case::Win)
}

#[test]
fn sandra_lottery_pg_seeded_blank() -> f::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, "lottery_blank", Case::Blank)
}

#[test]
fn sandra_lottery_pg_missing_input_is_atomic() -> f::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, "lottery_missing", Case::Missing)
}

#[test]
fn sandra_lottery_pg_full_inventory_is_atomic() -> f::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, "lottery_full", Case::Full)
}

#[test]
fn sandra_lottery_pg_replayed_command_applies_once() -> f::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, "lottery_replay", Case::Replay)
}

#[test]
fn sandra_lottery_pg_odds_equal_donor() -> f::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, "lottery_odds", Case::Odds)
}

fn run(admin: String, tag: &'static str, case: Case) -> f::TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            f::runtime()
                .map_err(f::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = f::Harness::create(admin, tag, true).await?;
                    let result = exercise(&h, case).await;
                    let cleanup = h.cleanup().await;
                    result?;
                    cleanup?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!("LOTTERY_PG_EXECUTED {tag} duration={:?}", started.elapsed());
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "Sandra PG worker failed")?
        .map_err(Into::into)
}

async fn exercise(h: &f::Harness, case: Case) -> f::TestResult {
    use crate::durability::quest_state::npc_program::sandra_lottery::draw_for_request;
    let fence = f::fence(1)?;
    let (controller, _) =
        oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(
            fence.runtime_scope.world_id(),
            None,
        );
    let generation = controller.active().ok_or("active generation")?;
    let native = generation.native_gameplay().ok_or("native generation")?;
    let premium =
        SpellPremiumCoordinator::new(Default::default(), None, None, Some(h.root.clone()));
    let content = Content {
        native,
        premium: &premium,
    };
    let lowered = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(f::debug)?;
    let catalogue = Arc::new(lowered.catalogue().clone());
    let source = NpcRuleCatalog::runtime()?
        .admitted_for_npc("oteryn:npc.sandra", &catalogue)?
        .ok_or("Sandra programme admission")?;
    assert!(
        source
            .programs()
            .iter()
            .flat_map(|p| &p.document.rules)
            .any(|rule| rule.key == "oteryn:quest-npc/sandra/lottery")
    );
    assert_eq!(SANDRA_LOTTERY.total_weight(), Some(98));
    assert_eq!(
        SANDRA_LOTTERY
            .outcomes
            .iter()
            .map(|o| o.weight)
            .collect::<Vec<_>>(),
        [1, 97]
    );
    assert_eq!(
        SANDRA_LOTTERY.outcomes[0].item,
        Some("oteryn:item.tibia.i5958")
    );
    assert_eq!(SANDRA_LOTTERY.outcomes[1].item, None);
    let has_ticket = !matches!(case, Case::Missing | Case::Odds);
    let full = matches!(case, Case::Full);
    seed_inventory(h, &content, has_ticket, full).await?;
    sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,$3,1,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(f::id(f::CHARACTER).as_slice())
        .bind("oteryn:quest-progress/quest/u7_8/mage_and_summoner_outfits/addon_belt")
        .bind("oteryn:quest.outfit_and_addon_quests").execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(f::id(f::WORLD).as_slice()).bind(f::id(f::CHANNEL).as_slice()).execute(&h.pool).await?;
    let identity = generation.identity();
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(f::id(f::WORLD).as_slice()).bind(f::id(f::CHANNEL).as_slice())
        .bind(identity.server_artifact_digest().as_slice())
        .bind(identity.client_artifact_digest().as_slice())
        .bind(identity.frame_binding_digest().as_slice()).execute(&h.pool).await?;
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(f::debug)?;
    let permit = SpellLanePermit::of_fresh_scope(fence.runtime_scope)
        .await
        .map_err(f::debug)?;
    let before = snapshot(h).await?;
    let draws: &[bool] = if full {
        &[true, false]
    } else {
        &[!matches!(case, Case::Blank)]
    };
    for &win in draws {
        let mut selected = None;
        for ordinal in 6275..16275 {
            let request = NpcProgramRequest {
                command: CommandRef::new(
                    fence.game_session_id,
                    CommandId::new(ordinal).map_err(f::debug)?,
                ),
                npc: "oteryn:npc.sandra".into(),
                trigger: NpcRuleTrigger::Say,
                message: "lottery".into(),
                topic: 0,
                transition_key: "oteryn:quest-npc/sandra/lottery".into(),
            };
            if draw_for_request(content.loop_content_digest(), &request, fence.character_id)
                .is_some_and(|draw| (draw == 0) == win)
            {
                selected = Some(request);
                break;
            }
        }
        let request = selected.ok_or("seeded outcome occurrence")?;
        let result = h
            .root
            .commit_character_npc_program(
                &authority,
                &h.node,
                &permit,
                fence,
                request.clone(),
                catalogue.clone(),
                &content,
                &premium,
            )
            .await
            .map_err(f::debug)?;
        if full || !has_ticket {
            assert!(
                matches!(result.transition, QuestTransitionOutcome::Refused(_)),
                "{result:?}"
            );
            if full {
                assert_eq!(
                    result.transition,
                    QuestTransitionOutcome::Refused(QuestRefusal::CapacityExceeded)
                );
            }
            assert!(!result.inventory_changed);
            assert_eq!(snapshot(h).await?, before);
            continue;
        }
        assert!(
            matches!(result.transition, QuestTransitionOutcome::Committed(_)),
            "{result:?}"
        );
        assert!(result.inventory_changed);
        let live: Vec<(String, i64)> = sqlx::query_as("SELECT definition_production_key,sum(quantity)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key IN ('oteryn:item.tibia.i5957','oteryn:item.tibia.i5958') GROUP BY definition_production_key ORDER BY definition_production_key")
            .fetch_all(&h.pool).await?;
        assert_eq!(
            live,
            if win {
                vec![("oteryn:item.tibia.i5958".into(), 1)]
            } else {
                vec![]
            }
        );
        let committed = snapshot(h).await?;
        let replay = h
            .root
            .commit_character_npc_program(
                &authority,
                &h.node,
                &permit,
                fence,
                request,
                catalogue.clone(),
                &content,
                &premium,
            )
            .await
            .map_err(f::debug)?;
        assert!(
            matches!(
                replay.transition,
                QuestTransitionOutcome::AlreadyCommitted(_)
            ),
            "{replay:?}"
        );
        assert_eq!(snapshot(h).await?, committed);
    }
    Ok(())
}

async fn seed_inventory(
    h: &f::Harness,
    content: &Content<'_>,
    ticket: bool,
    full: bool,
) -> f::TestResult {
    let bag = TypedDefinitionRef {
        family: "Item".into(),
        production_key: "oteryn:item.tibia.i2854".into(),
        revision_ref: "definition-r1".into(),
    };
    let capacity = content
        .loop_facts(&bag)
        .and_then(|facts| facts.container_capacity)
        .ok_or("native backpack capacity")?;
    let mut tx = h.pool.begin().await?;
    // Historical fixture rows only; all production-command triggers remain enabled.
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',1,1,encode($4,'hex')::uuid)")
        .bind(f::id(90).as_slice()).bind(f::id(f::WORLD).as_slice()).bind(&bag.production_key).bind(f::id(180).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_slots VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
        .bind(f::id(f::CHARACTER).as_slice()).bind(f::id(90).as_slice()).bind(f::id(f::WORLD).as_slice()).bind(f::id(180).as_slice()).execute(&mut *tx).await?;
    let count = if full { capacity } else { u32::from(ticket) };
    for ordinal in 0..count {
        let key = if ordinal == 0 && ticket {
            SANDRA_LOTTERY.input
        } else {
            "oteryn:item.tibia.i3052"
        };
        let item = f::id(100 + u8::try_from(ordinal)?);
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',1,1,encode($4,'hex')::uuid)")
            .bind(item.as_slice()).bind(f::id(f::WORLD).as_slice()).bind(key).bind(f::id(180).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,encode($6,'hex')::uuid)")
            .bind(f::id(f::CHARACTER).as_slice()).bind(f::id(90).as_slice()).bind(item.as_slice()).bind(f::id(f::WORLD).as_slice()).bind(i64::from(ordinal)).bind(f::id(180).as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    let entries: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(entries, i64::from(count));
    Ok(())
}

async fn snapshot(h: &f::Harness) -> f::TestResult<String> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_array((SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid),(SELECT coalesce(jsonb_agg(to_jsonb(i) ORDER BY item_instance_id),'[]') FROM game_item_instances i),(SELECT coalesce(jsonb_agg(to_jsonb(e) ORDER BY placement_ordinal),'[]') FROM game_item_container_entries e),(SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY track_key),'[]') FROM game_character_quest_tracks t),(SELECT count(*) FROM game_quest_npc_program_receipts),(SELECT count(*) FROM game_character_quest_receipts))::text")
        .bind(f::id(f::CHARACTER).as_slice()).fetch_one(&h.pool).await?)
}
