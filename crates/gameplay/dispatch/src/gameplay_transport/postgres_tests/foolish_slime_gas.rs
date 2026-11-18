//! An exact Slime death creates a two-second sample target for the selected Bozo flask.
use super::quest_world_callbacks::{self, tool_fixture};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::debug};
use crate::gameplay_transport::{
    connection::{FreshAdmissionAuthority, UseCommand},
    item_view::ItemKey,
};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::{
    authored_bindings::{Bindings, BoundQuest, Stage, Track, Trigger},
    reward_callbacks::RewardCallback,
    world_callbacks::{Callback, Offering, Position},
};
use oteryn_protocol_oteryn::world_object::{MapUseTarget, UseDisposition, UseIntent, UseTarget};
use std::{num::NonZeroU64, time::Duration};

const QUEST: &str = "oteryn:quest.what_a_foolish_quest";
const QUESTLINE: &str = "oteryn:quest-progress/quest/u8_1/what_afoolish_quest/questline";
const FLASK: &str = "oteryn:item.tibia.i135";
const FILLED: &str = "oteryn:item.tibia.i107";
const GAS: &str = "oteryn:item.tibia.i2127";
const SLIME: &str = "oteryn:creature.slime";

#[test]
fn only_the_original_slime_poison_gas_inside_two_seconds_fills_the_selected_flask() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("foolish-slime-gas", move || scenario(admin))
}

fn scenario(admin: String) -> TestResult {
    let at = Position {
        x: 32588,
        y: 31644,
        floor: -3,
    };
    let geometry: Callback = serde_json::from_value(serde_json::json!({
        "key": "oteryn:quest-callback/fixture/slime-map", "quest": QUEST,
        "trigger": "use", "selector": { "position": at, "appearance": 100 },
        "reversible": false, "passages": [], "storage": [], "offerings": [], "return_on_failure": false,
        "sources": ["PostgreSQL dispatch fixture"]
    }))?;
    let world = quest_world_callbacks::world(&geometry, at)?;
    // Crystal 201189f8 data-global/npc/bozo.lua:76-104 and
    // data-global/scripts/quests/others/actions_exterminator_flask.lua:2,23-39.
    // The source lifetime is data/items/items.xml:4356-4360.
    let sample_callback: RewardCallback = serde_json::from_value(serde_json::json!({
        "key": "oteryn:quest-reward/fixture/slime-gas", "quest": QUEST, "npc": "oteryn:npc.bozo",
        "keywords": ["sample"], "topic": 0, "next_topic": 0, "repeatable": true,
        "carried_item": FLASK, "creature_target": SLIME, "corpse_target": { "appearance": 2127, "maximum_age_ms": 2000 },
        "storage": [],
        "take": [{ "item": FLASK, "count": 1 }], "give": { "item": FILLED, "count": 1 },
        "reply": ["This mission stinks ... and now you do as well!"],
        "sources": ["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/others/actions_exterminator_flask.lua:2,23-39"]
    }))?;
    let bindings = Bindings {
        schema: 1,
        quests: vec![BoundQuest {
            quest: QUEST.into(),
            recipe_sha256: "1".repeat(64),
            occurrence: None,
            admission: None,
            tracks: vec![Track {
                key: QUESTLINE.into(),
                initial: 3,
                minimum: 0,
                maximum: 39,
            }],
            stages: vec![Stage {
                key: "gas".into(),
                completes: false,
                trigger: Trigger::CarriedUse {
                    callback: Box::new(sample_callback),
                },
            }],
            sources: vec![
                "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/bozo.lua:76-104"
                    .into(),
            ],
        }],
    };
    tool_fixture::run_bound_world(
        admin,
        "foolish_slime_gas",
        world,
        at,
        bindings,
        async |owner, admitted, h, fence| {
            quest_world_callbacks::input_item(
                owner,
                fence.item_fence(),
                &Offering {
                    item: FLASK.into(),
                    count: 1,
                    position: at,
                },
                33,
                true,
            )
            .await?;
            let flask: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(item_instance_id) FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1")
            .bind(FLASK).fetch_one(&h.pool).await?;
            let flask: [u8; 16] = flask.try_into().map_err(|_| "flask item UUID")?;
            let dead = kill_slime(owner, &admitted, at, 40).await?;
            let expired = gas_rows(h).await?;
            let [(old, 2000)] = expired.as_slice() else {
                return Err("one two-second Slime gas".into());
            };
            assert!(!owner.runtime.lock().await.contains_live_creature(dead));
            tokio::time::sleep(Duration::from_millis(2050)).await;
            let stale = sample(owner, admitted, flask, old.as_slice(), 41).await?;
            assert_ne!(stale, UseDisposition::Committed);
            kill_slime(owner, &admitted, at, 42).await?;
            let fresh = gas_rows(h)
                .await?
                .into_iter()
                .filter(|(id, _)| id != old)
                .collect::<Vec<_>>();
            let [(gas, 2000)] = fresh.as_slice() else {
                return Err("another two-second Slime gas".into());
            };
            assert_eq!(
                sample(owner, admitted, flask, gas, 43).await?,
                UseDisposition::Committed
            );
            let used: (Vec<u8>, Vec<u8>, i32) = sqlx::query_as(
            "SELECT uuid_send(corpse_item),uuid_send(source_item),maximum_age_ms FROM game_quest_corpse_samples",
        ).fetch_one(&h.pool).await?;
            assert_eq!(used, (gas.clone(), flask.to_vec(), 2000));
            let materials: Vec<(String, i64)> = sqlx::query_as("SELECT definition_production_key, quantity FROM game_item_instances WHERE definition_production_key IN ($1,$2) AND lifecycle=1")
            .bind(FLASK).bind(FILLED).fetch_all(&h.pool).await?;
            assert_eq!(materials, [(FILLED.into(), 1)]);
            Ok(())
        },
    )
}

async fn kill_slime(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    at: Position,
    command: u64,
) -> TestResult<crate::foundation::ExactActorRef> {
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native")?;
    let actor = {
        let mut runtime = owner.runtime.lock().await;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        let health = runtime
            .companion_policy(SLIME)
            .map_err(debug)?
            .maximum_health;
        let spawn = runtime
            .commit_quest_creature_spawn(&[(SLIME, health, quest_world_callbacks::local(at))])
            .map_err(debug)?;
        *spawn.first().ok_or("Slime")?
    };
    super::engworldchg_journals::kill_actor(owner, admitted, actor, command).await?;
    Ok(actor)
}

async fn gas_rows(h: &super::fixture::Harness) -> TestResult<Vec<(Vec<u8>, i64)>> {
    Ok(sqlx::query_as("SELECT uuid_send(i.item_instance_id), game_corpse_retirement_deadline(i.item_instance_id)-r.materialized_at \
        FROM game_item_instances i JOIN game_item_mint_receipts r USING(item_instance_id) WHERE i.definition_production_key=$1 AND i.lifecycle=1")
        .bind(GAS).fetch_all(&h.pool).await?)
}

async fn sample(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    flask: [u8; 16],
    gas: &[u8],
    command_id: u64,
) -> TestResult<UseDisposition> {
    let gas: [u8; 16] = gas.try_into().map_err(|_| "Slime gas item UUID")?;
    let result = owner
        .use_quest_tool(
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id,
                item_fence: admitted.item_fence,
            },
            UseIntent {
                target: UseTarget::Item(NonZeroU64::MIN),
                use_with: None,
                use_with_map_item: Some(MapUseTarget::Item(
                    NonZeroU64::new(2).ok_or("gas handle")?,
                )),
                use_with_position: None,
            },
            Some(ItemKey::Instance(flask)),
            Some(ItemKey::Instance(gas)),
        )
        .await
        .map_err(|()| "Slime USE unavailable")?;
    Ok(result.ok_or("Slime USE result")?.disposition)
}
