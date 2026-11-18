//! Crystal 201189f8 data-global/scripts/quests/kilmaresh/1-fafnars-wrath/6-mind-travel.lua:117-143.
use super::fixture::{Harness, debug, id};
use super::quest_world_callbacks::wire;
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
const TRACK: &str = "oteryn:quest-progress/fixture-kilmaresh/field";
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/kilmaresh/1-fafnars-wrath/6-mind-travel.lua:117-143";
const POS: (u16, u16, i8) = (33912, 31653, -9);
const SHARDS: [&str; 3] = [
    "oteryn:item.tibia.i31354",
    "oteryn:item.tibia.i31355",
    "oteryn:item.tibia.i31356",
];

#[test]
fn mind_energy_field_step_removes_exactly_one_owned_shard() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::kill_reward_e2e::scenario_thread::run_async(exercise)
}

fn catalogue() -> TestResult<QuestStateCatalogue> {
    let data = json!({"schema":1,"quests":[{
        "quest": QUEST, "sources": [SOURCE],
        "recipe_sha256": Sha256::digest(SOURCE).iter().map(|b| format!("{b:02x}")).collect::<String>(),
        "tracks": [{"key": TRACK, "initial": -1, "minimum": -1, "maximum": 1}],
        "stages": [{
            "key": "oteryn:quest-stage/fixture-kilmaresh/field", "completes": false,
            "trigger": {"kind": "step_in", "callback": {
                "key": KEY, "quest": QUEST, "trigger": "step_in",
                "selector": {"native": {"position": {"x": POS.0, "y": POS.1, "floor": POS.2},
                                        "ordinal": 1, "appearance": 1948}},
                "requires_world_admission": true, "passages": [], "offerings": [],
                "reversible": false, "return_on_failure": false, "sources": [SOURCE],
                "effects": {"random_step_item": true},
                "storage": [{"track": TRACK, "minimum": -1, "maximum": 1, "set": 1}],
                "item_on_target": {"item": SHARDS[0], "alternatives": &SHARDS[1..],
                                   "count": 1, "on_step": true}
            }}
        }]
    }]});
    let base = QuestStateCatalogue::empty("content-1").ok_or("fixture catalogue")?;
    Ok(Bindings::parse(&serde_json::to_vec(&data)?)?.attach(&base)?)
}

async fn exercise(admin: String) -> TestResult {
    let h = Harness::create_with_admission_seed(admin, "mind_memory_fields", true, false).await?;
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut deps = Dependencies::new()?;
    let (controller, pin) = super::death_ml_loss_composed::production_generation_with_pin(
        deps.admitted.world_id,
        "content-1",
    )?;
    let generation = controller.active().ok_or("content generation")?;
    let world = super::quest_callbacks::source_world(POS, 0, 1948, 0)?;
    let start = MovementLocalPosition {
        x: i32::from(POS.0 - 1),
        y: i32::from(POS.1),
        floor: 9,
    };
    super::arena_match::fixture::runtime_at(&mut deps, generation, &pin, &world, start)?;
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
    super::item_look::seed_main_backpack(&h, &SHARDS.map(|key| (key, 1))).await?;
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
        assert_eq!(remaining, 3 - i64::try_from(command.div_ceil(2).min(3))?);
    }
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(KEY)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(receipts, 3, "an empty inventory makes no debit");
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}
