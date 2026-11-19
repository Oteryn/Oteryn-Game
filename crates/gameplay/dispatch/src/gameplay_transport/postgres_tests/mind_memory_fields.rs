//! Crystal 201189f8 data-global/scripts/quests/kilmaresh/1-fafnars-wrath/6-mind-travel.lua:117-143.
use super::fixture::{Harness, debug, id};
use super::quest_world_callbacks_fixtures::wire;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, authored_bindings::Bindings};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.fixture_kilmaresh_shards";
const KEY: &str = "oteryn:quest-callback/fixture-kilmaresh/energy-field";
const TRACK: &str = "oteryn:quest-progress/fixture-kilmaresh/memories";
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/kilmaresh/1-fafnars-wrath/6-mind-travel.lua:117-143";
// Crystal startup/tables/item.lua:2382-2391 places action 40004 on energy fields.
const POS: (u16, u16, i8) = (33906, 31628, -9);
const FIELD: u16 = 2126;
const BAG: &str = "oteryn:item.tibia.i2853";
const COIN: &str = "oteryn:item.tibia.i3031";
const SHARDS: [&str; 3] = [
    "oteryn:item.tibia.i31354",
    "oteryn:item.tibia.i31355",
    "oteryn:item.tibia.i31356",
];

#[test]
fn mind_energy_field_step_removes_one_owned_shard_without_changing_memories() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(async |admin| exercise(admin, false).await)
}

#[test]
fn mind_energy_field_debits_a_shard_in_a_closed_nested_bag() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(async |admin| exercise(admin, true).await)
}

fn catalogue() -> TestResult<QuestStateCatalogue> {
    let data = json!({"schema":1,"quests":[{
        "quest": QUEST, "sources": [SOURCE],
        "recipe_sha256": Sha256::digest(SOURCE).iter().map(|b| format!("{b:02x}")).collect::<String>(),
        "tracks": [{"key": TRACK, "initial": 2, "minimum": -1, "maximum": 4}],
        "stages": [{
            "key": "oteryn:quest-stage/fixture-kilmaresh/field", "completes": false,
            "trigger": {"kind": "step_in", "callback": {
                "key": KEY, "quest": QUEST, "trigger": "step_in",
                "selector": {"native": {"position": {"x": POS.0, "y": POS.1, "floor": POS.2},
                                        "ordinal": 1, "appearance": FIELD}},
                "requires_world_admission": true, "passages": [], "offerings": [],
                "reversible": false, "return_on_failure": false, "sources": [SOURCE],
                "effects": {"random_step_item": true},
                "storage": [{"track": TRACK, "minimum": -1, "maximum": 4,
                             "set": 2, "preserve": true}],
                "item_on_target": {"item": SHARDS[0], "alternatives": &SHARDS[1..],
                                   "count": 1, "on_step": true}
            }}
        }]
    }]});
    let base = QuestStateCatalogue::empty("content-1").ok_or("fixture catalogue")?;
    Ok(Bindings::parse(&serde_json::to_vec(&data)?)?.attach(&base)?)
}

async fn exercise(admin: String, nested: bool) -> TestResult {
    let h = Harness::create_with_admission_seed(admin, "mind_memory_fields", true, false).await?;
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut deps = Dependencies::new()?;
    let (controller, pin) = super::death_ml_loss_composed_fixtures::production_generation_with_pin(
        deps.admitted.world_id,
        "content-1",
    )?;
    let generation = controller.active().ok_or("content generation")?;
    let world = super::quest_callbacks_fixtures::source_world(POS, 0, FIELD, 0)?;
    let start = MovementLocalPosition {
        x: i32::from(POS.0 - 1),
        y: i32::from(POS.1),
        floor: 9,
    };
    super::arena_match_fixtures::fixture::runtime_at(&mut deps, generation, &pin, &world, start)?;
    let cells = world
        .movement_cells(deps.room.movement_cells())
        .map_err(debug)?;
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = Some(generation);
    owner.movement_cells = (&cells).into();
    owner.quest_catalogue = Some(Arc::new(catalogue()?));
    deps.initialize(&owner).await;
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest session")?;
    admitted.item_fence = Some(fence.item_fence());
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(generation.identity().server_artifact_digest().as_slice())
        .bind(generation.identity().client_artifact_digest().as_slice())
        .bind(world.frame_binding_digest(admitted.world_id).as_slice()).execute(&h.pool).await?;
    let owned = if nested {
        // Crystal 6-mind-travel.lua:126 uses getItemById(id, true).
        // The closed bag contains one matching shard and an unrelated coin.
        super::item_look_fixtures::seed_main_backpack(&h, &[(BAG, 1), (SHARDS[1], 1), (COIN, 1)]).await?;
        nest_backpack_contents(&h).await?;
        1
    } else {
        super::item_look_fixtures::seed_main_backpack(&h, &SHARDS.map(|key| (key, 1))).await?;
        3
    };
    owner.admit_quest_session(&admitted).await;
    for command in 1..=7u64 {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("step clock")?;
        let direction = if command % 2 == 1 {
            StepDirection::East
        } else {
            StepDirection::West
        };
        let result = wire(
            &owner,
            &mut admitted,
            command,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            encode_step_intent(direction),
        )
        .await?;
        assert_eq!(
            decode_step_result(&result).map_err(debug)?,
            StepDisposition::Moved
        );
        let remaining: i64 = sqlx::query_scalar(
            "SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances
             WHERE lifecycle=1 AND definition_production_key=ANY($1::text[])",
        )
        .bind(SHARDS.as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(
            remaining,
            owned - i64::try_from(command.div_ceil(2))?.min(owned)
        );
    }
    let (receipts, preserved): (i64, i64) = sqlx::query_as(
        "SELECT count(*), count(*) FILTER (WHERE values_before = ARRAY[2]::bigint[]
                                            AND values_after = values_before)
         FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(KEY)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        receipts, owned,
        "an inventory without a shard makes no debit"
    );
    assert_eq!(
        preserved, receipts,
        "debit receipts preserve Memories progress"
    );
    if nested {
        let (debits, debited_nested): (i64, i64) = sqlx::query_as(
            "SELECT count(*), count(*) FILTER (WHERE location_kind='nested'
                AND item_instance_id=encode($2,'hex')::uuid AND quantity=1)
             FROM game_quest_callback_consumptions WHERE transition_key=$1",
        )
        .bind(KEY)
        .bind(id(72).as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!((debits, debited_nested), (1, 1));
        let surviving_contents: Vec<String> = sqlx::query_scalar(
            "SELECT i.definition_production_key FROM game_item_nested_entries n
             JOIN game_item_instances i USING (item_instance_id)
             WHERE n.parent_item_instance_id=encode($1,'hex')::uuid AND i.lifecycle=1",
        )
        .bind(id(71).as_slice())
        .fetch_all(&h.pool)
        .await?;
        assert_eq!(surviving_contents, [COIN]);
    }
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}

async fn nest_backpack_contents(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    let moved = sqlx::query(
        "WITH entries AS (
           DELETE FROM game_item_container_entries
           WHERE parent_item_instance_id=encode($1,'hex')::uuid
             AND item_instance_id IN (encode($2,'hex')::uuid,encode($3,'hex')::uuid)
           RETURNING item_instance_id,world_id,placement_ordinal,placed_transaction_id
         ) INSERT INTO game_item_nested_entries(
           item_instance_id,parent_item_instance_id,world_id,placement_ordinal,placed_transaction_id)
         SELECT item_instance_id,encode($4,'hex')::uuid,world_id,placement_ordinal,placed_transaction_id
         FROM entries",
    )
    .bind(id(70).as_slice())
    .bind(id(72).as_slice())
    .bind(id(73).as_slice())
    .bind(id(71).as_slice())
    .execute(&mut *tx)
    .await?;
    assert_eq!(moved.rows_affected(), 2);
    tx.commit().await?;
    Ok(())
}
