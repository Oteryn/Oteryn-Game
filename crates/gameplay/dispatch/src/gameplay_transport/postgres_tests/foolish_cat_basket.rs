//! Real basket USE creates portable Ground custody and one exact timed creature.
use super::quest_world_callbacks::{self, tool_fixture};
use super::{TestResult, fixture::debug};
use crate::gameplay_transport::{
    connection::{FreshAdmissionAuthority, UseCommand},
    item_move::{LoopMove, LoopMoveResult, LoopTarget},
    item_view::ItemKey,
    loop_move::{loop_ground, move_loop_item},
};
use oteryn_gameplay_quests::quest::{
    QuestTrack,
    world_callbacks::{Callback, Position},
};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use std::sync::atomic::Ordering;

const QUEST: &str = "oteryn:quest.what_a_foolish_quest";
const QUESTLINE: &str = "oteryn:quest-progress/quest/u8_1/what_afoolish_quest/questline";
const BASKET: &str = "oteryn:quest-progress/quest/u8_1/what_afoolish_quest/cat_basket";
const KITTY: &str = "oteryn:creature.kitty";
const TOY: &str = "oteryn:item.tibia.i123";

#[test]
fn cat_basket_creates_a_portable_floor_toy_and_despawns_its_exact_kitty_after_ten_seconds()
-> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("foolish-cat-basket", move || scenario(admin, false))
}

#[test]
fn cat_basket_floor_reward_refreshes_ground_after_an_interrupted_use_acknowledgement() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("foolish-cat-basket-interrupted", move || {
        scenario(admin, true)
    })
}

fn scenario(admin: String, interrupted: bool) -> TestResult {
    // Crystal 201189f8 data-global/scripts/quests/what_a_foolish_quest/action_cat_basket.lua:18-39.
    let basket = Position {
        x: 32311,
        y: 31744,
        floor: -9,
    };
    let below = Position {
        y: basket.y + 1,
        ..basket
    };
    let callback: Callback = serde_json::from_value(serde_json::json!({
        "key": "oteryn:quest-callback/fixture/foolish-cat-basket",
        "quest": QUEST, "trigger": "use",
        "selector": {"appearance": 122, "native": {"position": basket, "appearance": 122, "ordinal": 2}},
        "requires_world_admission": true, "reversible": false, "passages": [],
        "offerings": [], "return_on_failure": false,
        "storage": [
            {"track": QUESTLINE, "minimum": 19, "maximum": 19, "set": 19, "preserve": true},
            {"track": BASKET, "minimum": -1, "maximum": 0, "set": 1}
        ],
        "effects": {
            "floor_reward": {"item": TOY, "count": 1},
            "creature_spawn": {"creature": KITTY, "position": below, "lifetime_ms": 10000}
        },
        "sources": ["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/what_a_foolish_quest/action_cat_basket.lua:18-39"]
    }))?;
    let world = quest_world_callbacks::world_with_cells(&callback, basket, &[below])?;
    tool_fixture::run_world(
        admin,
        "foolish_cat_basket",
        world,
        callback,
        vec![
            QuestTrack {
                key: QUESTLINE.into(),
                quest: QUEST.into(),
                initial: 19,
                min: -1,
                max: 40,
            },
            QuestTrack {
                key: BASKET.into(),
                quest: QUEST.into(),
                initial: -1,
                min: -1,
                max: 1,
            },
        ],
        async move |owner, admitted, h, fence| {
            let _clock = owner.freeze_owner_clock_for_test();
            let used_at = 2_000_000;
            let deadline = used_at + 10_000_000;
            // The frozen clock reserves zero as the real-clock sentinel.
            owner
                .states
                .test_owner_time
                .store(used_at + 1, Ordering::Relaxed);
            let map = owner.bundle_map().ok_or("basket map")?;
            let (_, target) = tool_fixture::placement(map, basket, 2)?;
            let projection_before = if interrupted {
                let index = owner.active_generation
                    .and_then(|generation| generation.native_gameplay())
                    .and_then(|content| content.item_index())
                    .map(|index| index.as_ref());
                let cached = loop_ground(owner, map.overlay.base(), index)
                    .await.ok_or("pre-USE Ground projection")?;
                owner.states.quest_callbacks.interrupt_after_commit.store(true, Ordering::Release);
                Some((cached, index))
            } else {
                None
            };
            let outcome = owner
                .use_object(
                    admitted.runtime_actor.ok_or("actor")?,
                    UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id: 51,
                        item_fence: Some(fence.item_fence()),
                    },
                    target,
                )
                .await;
            if let Some((before, index)) = projection_before {
                assert_eq!(outcome.disposition, UseDisposition::Rejected);
                assert!(owner.has_pending_quest_callback(admitted.game_session_id));
                let toy: Vec<u8> = sqlx::query_scalar(
                    "SELECT uuid_send(g.item_instance_id) FROM game_item_ground_locations g
                     JOIN game_native_map_item_receipts r USING(item_instance_id)
                     WHERE r.definition_key=$1",
                ).bind(TOY).fetch_one(&h.pool).await?;
                let toy: [u8; 16] = toy.try_into().map_err(|_| "floor toy item UUID")?;
                assert!(before.ground_entry(&toy).is_none());
                let after = loop_ground(owner, map.overlay.base(), index)
                    .await.ok_or("post-USE Ground projection")?;
                let (at, _) = after.ground_entry(&toy).ok_or("committed toy in Ground projection")?;
                assert_eq!((at.x, at.y, at.floor), (basket.x, basket.y, basket.floor));
                return Ok(());
            }
            assert_eq!(outcome.disposition, UseDisposition::Committed);
            let kitty = {
                let runtime = owner.runtime.lock().await;
                let actors = runtime.current_live_creature_census().map_err(debug)?;
                let [kitty] = actors.as_slice() else {
                    return Err("one Kitty".into());
                };
                assert!(runtime.matches_live_creature_identity(*kitty, KITTY.as_bytes()));
                assert_eq!(
                    runtime
                        .read_actor_position(*kitty)
                        .map_err(debug)?
                        .position(),
                    quest_world_callbacks::local(below)
                );
                *kitty
            };
            let (toy_id, quantity, at, pickup_qualified, same_transaction): (Vec<u8>, i64, Vec<u8>, bool, bool) = sqlx::query_as(
            "SELECT uuid_send(i.item_instance_id),i.quantity,g.spatial_position,game_item_loop_map_mint_source(i.item_instance_id), \
             q.xmin::text::numeric=mod(r.created_xact_id::text::numeric,4294967296) \
             FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) \
             JOIN game_native_map_item_receipts r USING(item_instance_id) \
             JOIN game_character_quest_receipts q ON q.cause_id=(r.source_attributes->>'session')::uuid \
             AND q.cause_kind='use' AND q.cause_ordinal=(r.source_attributes->>'command')::numeric \
             AND q.transition_key=r.source_attributes->>'transition' \
             WHERE i.definition_production_key=$1 AND i.lifecycle=1",
        ).bind(TOY).fetch_one(&h.pool).await?;
            assert_eq!(quantity, 1);
            assert_eq!(
                at,
                crate::durability::map_item_mint_audit::map_item_spatial_position(
                    basket.x,
                    basket.y,
                    basket.floor
                )
            );
            assert!(
                pickup_qualified,
                "toy uses ordinary Ground pickup authority"
            );
            assert!(
                same_transaction,
                "the Quest and floor item share one commit"
            );

            let toy_id: [u8; 16] = toy_id.try_into().map_err(|_| "floor toy item UUID")?;
            let pickup = move_loop_item(
                owner,
                admitted.runtime_actor.ok_or("actor")?,
                UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: 52,
                    item_fence: Some(fence.item_fence()),
                },
                LoopMove {
                    item: ItemKey::Instance(toy_id),
                    count: None,
                    target: LoopTarget::MainBackpack,
                },
            )
            .await;
            assert_eq!(pickup, LoopMoveResult::Committed);
            let custody: (bool, bool) = sqlx::query_as(
                "SELECT EXISTS(SELECT 1 FROM game_item_ground_locations
                    WHERE item_instance_id=encode($1,'hex')::uuid),
                 EXISTS(SELECT 1 FROM game_item_container_entries
                    WHERE item_instance_id=encode($1,'hex')::uuid
                    AND character_id=encode($2,'hex')::uuid)",
            )
            .bind(toy_id.as_slice())
            .bind(fence.character_id.as_bytes().as_slice())
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(custody, (false, true), "the same toy enters the Character backpack");

            owner
                .states
                .test_owner_time
                .store(deadline, Ordering::Relaxed);
            owner.drain_quest_encounter_deadlines().await;
            assert!(owner.runtime.lock().await.contains_live_creature(kitty));
            owner
                .states
                .test_owner_time
                .store(deadline + 1, Ordering::Relaxed);
            owner.drain_quest_encounter_deadlines().await;
            assert!(!owner.runtime.lock().await.contains_live_creature(kitty));
            let copy = owner
                .root
                .read_character_quest_state(owner.character, fence.character_id)
                .await
                .map_err(debug)?;
            assert_eq!(copy.tracks().get(BASKET), Some(&1));
            Ok(())
        },
    )
}
