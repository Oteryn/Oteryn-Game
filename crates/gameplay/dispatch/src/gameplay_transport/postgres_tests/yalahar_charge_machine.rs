//! Crystal 201189f8 data-global/scripts/quests/shadows_of_yalahar_quest/
//! movements_mission05.lua:9-18: one of six charges per delivery, during stages 1-3.
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

const QUEST: &str = "oteryn:quest.fixture_yalahar_charge";
const MISSION: &str = "oteryn:quest-progress/fixture-yalahar/mission05";
const CHARGE: &str = "oteryn:quest-progress/fixture-yalahar/charge-current";
const KEY: &str = "oteryn:quest-callback/fixture-yalahar/charging-machine";
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/shadows_of_yalahar_quest/movements_mission05.lua:9-18";
const TILE: (u16, u16, i8) = (32868, 31261, -11);

#[test]
fn yalahar_step_assigns_one_persistent_charge_and_refuses_a_second() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::kill_reward_e2e::scenario_thread::run_async(async |admin| exercise(&admin).await)
}

fn catalogue() -> TestResult<QuestStateCatalogue> {
    let binding = json!({"schema": 1, "quests": [{
        "quest": QUEST,
        "recipe_sha256": Sha256::digest(SOURCE).iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
        "sources": [SOURCE],
        "tracks": [
            {"key": MISSION, "initial": 1, "minimum": -1, "maximum": 4},
            {"key": CHARGE, "initial": -1, "minimum": -1, "maximum": 6}
        ],
        "stages": [{
            "key": "oteryn:quest-stage/fixture-yalahar/charge-machine",
            "completes": false,
            "trigger": {"kind": "step_in", "callback": {
                "key": KEY, "quest": QUEST, "trigger": "step_in",
                "selector": {"position": {"x": TILE.0, "y": TILE.1, "floor": TILE.2}, "ordinal": 0},
                "reversible": false, "passages": [], "offerings": [], "return_on_failure": false,
                "requires_world_admission": true,
                "storage": [{"track": MISSION, "minimum": 1, "maximum": 3, "set": 1, "preserve": true}],
                "state_operations": [{
                    "track": CHARGE, "from": {"kind": "between", "minimum": -1, "maximum": 0},
                    "effect": {"kind": "draw", "sides": 6}
                }],
                "sources": [SOURCE]
            }}
        }]
    }]});
    let base = QuestStateCatalogue::empty("content-1").ok_or("fixture catalogue")?;
    Ok(Bindings::parse(&serde_json::to_vec(&binding)?)?.attach(&base)?)
}

async fn exercise(admin: &str) -> TestResult {
    let h = Harness::create_with_admission_seed(
        admin.to_owned(),
        "yalahar_charge_machine",
        true,
        false,
    )
    .await?;
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut deps = Dependencies::new()?;
    let (controller, activation) = super::death_ml_loss_composed::production_generation_with_pin(
        deps.admitted.world_id,
        "content-1",
    )?;
    let generation = controller.active().ok_or("active generation")?;
    let world = super::quest_callbacks::source_world(TILE, 25030, 1948, 0)?;
    super::arena_match::fixture::runtime_at(
        &mut deps,
        generation,
        &activation,
        &world,
        MovementLocalPosition {
            x: i32::from(TILE.0 - 1),
            y: i32::from(TILE.1),
            floor: -i16::from(TILE.2),
        },
    )?;
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
    owner.admit_progression_session(&mut admitted).await;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(generation.identity().server_artifact_digest().as_slice())
        .bind(generation.identity().client_artifact_digest().as_slice())
        .bind(world.frame_binding_digest(admitted.world_id).as_slice()).execute(&h.pool).await?;
    owner.admit_quest_session(&admitted).await;

    let mut assigned = None;
    for (command, direction) in [
        (1, StepDirection::East),
        (2, StepDirection::West),
        (3, StepDirection::East),
    ] {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("step clock")?;
        let reply = wire(
            &owner,
            &mut admitted,
            command,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            encode_step_intent(direction),
        )
        .await?;
        assert_eq!(
            decode_step_result(&reply).map_err(debug)?,
            StepDisposition::Moved
        );
        let state = h
            .root
            .read_character_quest_state(&authority, fence.character_id)
            .await
            .map_err(debug)?;
        let charge = *state.tracks().get(CHARGE).ok_or("persisted charge")?;
        assert!((1..=6).contains(&charge));
        assert_eq!(*assigned.get_or_insert(charge), charge);
    }
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key = $1",
    )
    .bind(KEY)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        receipts, 1,
        "a second entry cannot reroll the undelivered charge"
    );
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}
