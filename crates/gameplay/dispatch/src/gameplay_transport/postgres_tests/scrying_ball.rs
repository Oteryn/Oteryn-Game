//! The selected base object is removed and restored by its Channel decay deadline.
use super::TestResult;
use super::fixture::{debug, join_two};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{
    QuestTrack,
    world_callbacks::{Callback, Position},
};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use std::{sync::atomic::Ordering, time::Duration};

const TRACK: &str = "oteryn:quest-progress/quest/u8_1/to_blind_the_enemy/questline";
const QUEST: &str = "oteryn:quest.to_blind_the_enemy_quest";
const MIRROR: &str = "oteryn:item.tibia.i3463";

#[test]
fn scrying_ball_use_consumes_mirror_and_restores_exact_shared_object_after_sixty_seconds()
-> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("scrying-ball", move || scenario(admin))
}

fn scenario(admin: String) -> TestResult {
    // See Crystal 201189f8 data-global/scripts/quests/to_blind_the_enemy_quest/mirror.lua:10-21.
    let trigger = Position {
        x: 32592,
        y: 31645,
        floor: -2,
    };
    let decoy = Position {
        x: trigger.x,
        y: trigger.y + 1,
        ..trigger
    };
    let callback: Callback = serde_json::from_value(serde_json::json!({
        "key": "oteryn:quest-callback/fixture/scrying-ball",
        "quest": QUEST,
        "trigger": "use",
        "selector": {
            "appearance": 132,
            "native": { "position": trigger, "appearance": 132, "ordinal": 2 }
        },
        "requires_world_admission": true,
        "reversible": false,
        "passages": [],
        "offerings": [],
        "return_on_failure": false,
        "storage": [{ "track": TRACK, "minimum": 1, "maximum": 1, "set": 2 }],
        "item_on_target": {
            "item": MIRROR, "count": 1, "target_remove": true, "selected_instance": true
        },
        "effects": { "reset_after_ms": 60000 },
        "sources": ["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/to_blind_the_enemy_quest/mirror.lua:10-21"]
    }))?;
    let world =
        super::quest_world_callbacks::world_with_objects(&callback, trigger, &[(decoy, 132)], &[])?;
    super::quest_world_callbacks::tool_fixture::run_world(
        admin,
        "scrying_ball",
        world,
        callback,
        vec![QuestTrack {
            key: TRACK.into(),
            quest: QUEST.into(),
            initial: 1,
            min: 0,
            max: 2,
        }],
        async |owner, admitted, h, fence| {
            use super::quest_world_callbacks::tool_fixture::{placement, use_tool};
            let map = owner.bundle_map().ok_or("bundle map")?;
            let (key, _) = placement(map, trigger, 2)?;
            let (decoy_key, _) = placement(map, decoy, 1)?;
            let clock = owner.freeze_owner_clock_for_test();
            let used_at = 2_000_000;
            let deadline = used_at + 60_000_000;
            // The frozen clock reserves zero as the real-clock sentinel.
            owner
                .states
                .test_owner_time
                .store(used_at + 1, Ordering::Relaxed);
            let (_, result) = use_tool(owner, admitted, fence, MIRROR, trigger, 2).await?;
            assert_eq!(result, UseDisposition::Committed);
            assert!(map.facts.base_hidden(key));
            assert!(!map.facts.base_hidden(decoy_key));
            assert_eq!(map.facts.object_revision(key), 1);
            assert_eq!(map.facts.object_revision(decoy_key), 0);
            assert_eq!(map.facts.next_movement_decay_time(), Some(deadline));
            assert_eq!(map.facts.expire_movement_items(deadline - 1), 0);
            assert!(map.facts.base_hidden(key));
            // Use the same Channel owner cycle as serving, without a connection tick.
            owner.states.test_owner_time.store(deadline + 1, Ordering::Relaxed);
            let stop = oteryn_foundation::CancellationToken::new();
            let observe = async {
                while map.facts.base_hidden(key) {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                stop.cancel();
            };
            tokio::time::timeout(
                Duration::from_secs(15),
                join_two(owner.run_source_owner_cycles(&stop), observe),
            ).await.map_err(|_| "Channel cycle did not restore the Scrying Ball at its deadline")?;
            assert!(!map.facts.base_hidden(key));
            assert!(!map.facts.base_hidden(decoy_key));
            assert_eq!(map.facts.object_revision(key), 2);
            assert_eq!(map.facts.object_revision(decoy_key), 0);
            assert_eq!(map.facts.next_movement_decay_time(), None);
            assert_eq!(map.facts.expire_movement_items(deadline + 1), 0);
            let copy = owner
                .root
                .read_character_quest_state(owner.character, fence.character_id)
                .await
                .map_err(debug)?;
            assert_eq!(copy.tracks().get(TRACK), Some(&2));
            let mirrors: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1",
        ).bind(MIRROR).fetch_one(&h.pool).await?;
            assert_eq!(mirrors, 0);
            drop(clock);
            Ok(())
        },
    )
}
