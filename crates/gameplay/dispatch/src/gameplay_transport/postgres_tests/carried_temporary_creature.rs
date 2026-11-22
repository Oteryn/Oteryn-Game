use super::fixture::debug;
use super::quest_world_callbacks_fixtures::{input_item, npc_scenario};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::QuestStateCatalogue;
use oteryn_gameplay_quests::quest::authored_bindings::Bindings;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, UseIntent, UseTarget};
use std::sync::Arc;

const ORIGIN: Position = Position {
    x: 33294,
    y: 31677,
    floor: -15,
};
const ITEM: &str = "oteryn:item.tibia.i3031";
const CAT: &str = "oteryn:creature.cat";

#[test]
fn carried_use_receipt_spawns_one_temporary_creature_and_expires_only_that_actor() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("carried-temporary-creature", || {
        npc_scenario(ORIGIN, |owner, admitted, harness| {
            Box::pin(exercise(owner, admitted, harness))
        })
    })
}

async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    harness: &super::fixture::Harness,
) -> TestResult {
    // Crystal supplies Kitty's lifetime. This synthetic carried-use binding
    // uses the generation's admitted gold coin and Cat to exercise the runtime owner.
    let quest = "oteryn:quest.what_a_foolish_quest";
    let track = "oteryn:quest-progress/what-a-foolish/cat-basket";
    let source = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/what_a_foolish_quest/action_cat_basket.lua:18-39";
    let recipe_sha256: String = <sha2::Sha256 as sha2::Digest>::digest(source)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let bindings = Bindings::parse(&serde_json::to_vec(&serde_json::json!({
        "schema": 1,
        "quests": [{
            "quest": quest,
            "recipe_sha256": recipe_sha256,
            "sources": [source],
            "tracks": [{"key": track, "initial": 0, "minimum": 0, "maximum": 1}],
            "stages": [{"key": "cat-basket", "completes": false, "trigger": {
                "kind": "carried_use",
                "callback": {
                    "key": "oteryn:quest-reward/what-a-foolish/cat-basket",
                    "quest": quest, "npc": "oteryn:npc.bozo",
                    "carried_item": ITEM,
                    "temporary_creature": {"creature": CAT, "lifetime_ms": 10000},
                    "keywords": ["basket"], "topic": 0, "next_topic": 0,
                    "storage": [{"track": track, "minimum": 0, "maximum": 0, "set": 1}],
                    "take": [], "present": [{"item": ITEM, "count": 1}], "reply": ["The queen's cat is not amused!"],
                    "sources": [source]
                }
            }}]
        }]
    }))?)?;
    let base = QuestStateCatalogue::empty("content-1").ok_or("base catalogue")?;
    owner.quest_catalogue = Some(Arc::new(bindings.attach(&base)?));
    let session = admitted.game_session_id;
    owner.refresh_quest_session(session, false).await;
    let actor = admitted.runtime_actor.ok_or("player actor")?;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let before = {
        let native = owner
            .active_generation
            .and_then(|generation| generation.native_gameplay())
            .ok_or("native gameplay")?;
        let mut runtime = owner.runtime.lock().await;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        let health = runtime.companion_policy(CAT).map_err(debug)?.maximum_health;
        let unrelated = crate::foundation::MovementLocalPosition {
            x: i32::from(ORIGIN.x) + 4,
            y: i32::from(ORIGIN.y),
            floor: -i16::from(ORIGIN.floor),
        };
        runtime
            .commit_quest_creature_spawn(&[(CAT, health, unrelated)])
            .map_err(debug)?;
        runtime.current_live_creature_census().map_err(debug)?
    };
    let offered = Offering {
        position: ORIGIN,
        item: ITEM.into(),
        count: 1,
    };
    input_item(owner, fence.item_fence(), &offered, 76, true).await?;
    let inventory = owner
        .root
        .read_item_use_inventory(owner.character, owner.holder, fence.item_fence())
        .await
        .map_err(debug)?;
    let carried_item = inventory
        .backpack
        .iter()
        .flat_map(|bag| &bag.entries)
        .find(|entry| entry.item.definition.production_key == ITEM)
        .ok_or("carried quest item")?
        .item
        .item_instance_id;

    let intent = UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    };
    let command = UseCommand {
        game_session_id: session,
        command_id: 6301,
        item_fence: Some(fence.item_fence()),
    };
    owner
        .states
        .quest_callbacks
        .interrupt_after_commit
        .store(true, std::sync::atomic::Ordering::Release);
    let lost = owner
        .use_event_item(
            actor,
            command,
            intent.clone(),
            Some(ItemKey::Instance(carried_item)),
            false,
        )
        .await;
    assert!(lost.is_err());
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .current_live_creature_census()
            .map_err(debug)?,
        before
    );
    for _ in 0..2 {
        let result = owner
            .use_event_item(
                actor,
                command,
                intent.clone(),
                Some(ItemKey::Instance(carried_item)),
                false,
            )
            .await
            .map_err(|()| "USE result")?
            .ok_or("carried callback")?;
        assert_eq!(result.disposition, UseDisposition::Committed);
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .current_live_creature_census()
                .map_err(debug)?
                .len(),
            before.len() + 1
        );
    }

    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-reward/what-a-foolish/cat-basket'",
    ).fetch_one(&harness.pool).await?;
    assert_eq!(receipts, 1);
    let due = owner
        .states
        .quest_callbacks
        .next_encounter_due_us()
        .ok_or("creature expiry")?;
    assert!(due > owner.owner_now().get());
    owner.drain_quest_encounter_deadlines().await;
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .current_live_creature_census()
            .map_err(debug)?
            .len(),
        before.len() + 1
    );
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(10))
        .ok_or("clock")?;
    owner.drain_quest_encounter_deadlines().await;
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .current_live_creature_census()
            .map_err(debug)?,
        before
    );
    let inventory = owner
        .root
        .read_item_use_inventory(owner.character, owner.holder, fence.item_fence())
        .await
        .map_err(debug)?;
    assert!(
        inventory
            .backpack
            .iter()
            .flat_map(|bag| &bag.entries)
            .any(|entry| entry.item.item_instance_id == carried_item)
    );
    Ok(())
}
