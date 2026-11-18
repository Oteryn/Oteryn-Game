//! The selected base object is removed and restored by its Channel decay deadline.
use super::fixture::{Harness, debug, id, join_two};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::{ControllerBinding, SessionContinuity};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{
    QuestTrack,
    world_callbacks::{Callback, Offering, Position},
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
    super::kill_reward_e2e::scenario_thread::run("scrying-ball", move || scenario(admin, false))
}

#[test]
fn scrying_ball_holds_the_selected_object_until_interrupted_use_reconciles() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("scrying-ball-lost-ack", move || {
        scenario(admin, true)
    })
}

fn scenario(admin: String, interrupted: bool) -> TestResult {
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
        async move |owner, admitted, h, fence| {
            use super::quest_world_callbacks::tool_fixture::{placement, use_held_tool, use_tool};
            let map = owner.bundle_map().ok_or("bundle map")?;
            let (key, _) = placement(map, trigger, 2)?;
            let (decoy_key, _) = placement(map, decoy, 1)?;
            let rival = if interrupted {
                Some(admit_rival(h, owner, admitted, decoy).await?)
            } else {
                None
            };
            let clock = owner.freeze_owner_clock_for_test();
            let used_at = 2_000_000;
            let published_at = if interrupted { used_at + 1_000_000 } else { used_at };
            let deadline = published_at + 60_000_000;
            // The frozen clock reserves zero as the real-clock sentinel.
            owner
                .states
                .test_owner_time
                .store(used_at + 1, Ordering::Relaxed);
            let mut retry_peer = None;
            if let Some(rival) = rival {
                owner.states.quest_callbacks.interrupt_after_commit.store(true, Ordering::Release);
                assert!(use_tool(owner, admitted, fence, MIRROR, trigger, 2).await.is_err());
                assert!(owner.has_pending_quest_callback(admitted.game_session_id));
                assert!(!map.facts.base_hidden(key));
                assert_eq!(map.facts.next_movement_decay_time(), None);
                assert_eq!(live_mirrors(h).await?, 1, "only the rival's mirror remains");
                let (rival, refusal) = use_held_tool(owner, rival, MIRROR, trigger, 2).await?;
                assert_eq!(refusal, UseDisposition::Rejected);
                assert_eq!(live_mirrors(h).await?, 1);
                assert_eq!(map.facts.object_revision(key), 0);
                owner.states.test_owner_time.store(published_at + 1, Ordering::Relaxed);
                assert!(owner.reconcile_quest_callbacks(admitted.game_session_id).await);
                assert!(!owner.has_pending_quest_callback(admitted.game_session_id));
                retry_peer = Some(rival);
            } else {
                let (_, result) = use_tool(owner, admitted, fence, MIRROR, trigger, 2).await?;
                assert_eq!(result, UseDisposition::Committed);
            }
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
            if let Some(rival) = retry_peer {
                // The same admitted rival can use the restored Ball, proving the
                // earlier refusal came from the outstanding shared removal.
                let (_, result) = use_held_tool(owner, rival, MIRROR, trigger, 2).await?;
                assert_eq!(result, UseDisposition::Committed);
                assert!(map.facts.base_hidden(key));
            }
            assert_eq!(live_mirrors(h).await?, 0);
            drop(clock);
            Ok(())
        },
    )
}

async fn live_mirrors(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1",
    )
    .bind(MIRROR)
    .fetch_one(&h.pool)
    .await?)
}

async fn admit_rival(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    position: Position,
) -> TestResult<AdmittedSession> {
    use crate::durability::{item_mint::TypedDefinitionRef, starter_grant::StarterTemplate};
    use crate::gameplay_transport::{item_fence_of, item_ref_admission::NativeItemWeights};
    use crate::world_party_runtime_postgres_cases::{
        FreshPlayerOwners, committed_fresh_player_with_operation,
    };

    let now = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
        .fetch_one(&h.pool)
        .await?;
    let (current, operation) = committed_fresh_player_with_operation(
        &FreshPlayerOwners {
            root: h.root.clone(),
            node: h.node.clone(),
            recovery: &h.recovery,
        },
        60,
        63,
        171,
        now,
    )
    .await?;
    let fence = item_fence_of(current).ok_or("rival item fence")?;
    let actor = {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime
            .reserve_fresh_session(fence.game_session_id)
            .map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .bind_attacker_lease(actor, fence.game_session_id, current.current_character_lease())
            .map_err(debug)?;
        runtime
            .initialize_pinned_test_position(actor, super::quest_world_callbacks::local(position))
            .map_err(debug)?;
        actor
    };
    let mut rival = AdmittedSession {
        game_session_id: fence.game_session_id,
        runtime_actor: Some(actor),
        controller: Some(ControllerBinding {
            transport: current.current_transport().ok_or("rival transport")?,
            account_id: id(63),
        }),
        continuity: SessionContinuity::FRESH,
        item_fence: Some(fence),
        ..admitted
    };
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native items")?;
    h.root
        .initialize_admission_equipment(
            owner.character,
            owner.holder,
            &fence,
            native.source_digest(),
            &operation,
        )
        .await
        .map_err(debug)?;
    h.root
        .grant_starter_items(
            owner.character,
            owner.holder,
            &fence,
            &NativeItemWeights(native),
            &StarterTemplate {
                definition: TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: "oteryn:item.tibia.i2854".into(),
                    revision_ref: "definition-r1".into(),
                },
                ..StarterTemplate::main()
            },
        )
        .await
        .map_err(debug)?;
    owner.admit_progression_session(&mut rival).await;
    owner.admit_quest_session(&rival).await;
    super::quest_world_callbacks::input_item(
        owner,
        fence,
        &Offering {
            position,
            item: MIRROR.into(),
            count: 1,
        },
        3303,
        true,
    )
    .await?;
    Ok(rival)
}
