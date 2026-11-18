//! Crystal 201189f8 actions_mission02.lua:7-23,60-104 and data/items/items.xml:24509-24548.
use super::fixture::{Harness, debug, id};
use super::quest_world_callbacks::wire;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, authored_bindings::Bindings};
use oteryn_protocol_oteryn::world_object::{
    COMMAND_TYPE_USE_INTENT, UseDisposition, WorldObjectTarget, decode_use_result,
    encode_use_intent,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.fixture_yalahar_cannon";
const CORE_SAFE: &str = "oteryn:quest-progress/fixture-yalahar/core-safe";
const CANNON: (u16, u16, i8) = (32867, 31257, -11);
const COLUMN: (u16, u16, i8) = (32867, 31253, -11);
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/shadows_of_yalahar_quest/actions_mission02.lua:7-23,60-104";

#[test]
fn yalahar_world_cycle_changes_which_cannon_use_can_make_the_core_safe() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::kill_reward_e2e::scenario_thread::run_async(async |admin| exercise(&admin).await)
}

fn catalogue() -> TestResult<QuestStateCatalogue> {
    let stages: Vec<_> = [9133, 9137].into_iter().enumerate().map(|(index, required)| {
        json!({
            "key": format!("oteryn:quest-stage/fixture-yalahar/cannon-{required}"),
            "completes": false,
            "trigger": {"kind": "map_use", "callback": {
                "key": format!("oteryn:quest-callback/fixture-yalahar/cannon-{required}"),
                "quest": QUEST, "trigger": "use", "reversible": false,
                "selector": {"position": {"x": CANNON.0, "y": CANNON.1, "floor": CANNON.2}, "ordinal": 1},
                "effects": {"cycling_appearance": {
                    "position": {"x": COLUMN.0, "y": COLUMN.1, "floor": COLUMN.2},
                    "ordinal": 1, "appearances": [9133, 9137], "interval_ms": 10_000, "required": required
                }},
                "storage": [{"track": CORE_SAFE, "minimum": -1, "maximum": 0, "set": index}],
                "requires_world_admission": true, "passages": [], "offerings": [], "return_on_failure": false,
                "sources": [SOURCE, "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data/items/items.xml:24509-24548"]
            }}
        })
    }).collect();
    let data = json!({"schema": 1, "quests": [{
        "quest": QUEST, "recipe_sha256": format!("{:x}", Sha256::digest(SOURCE)),
        "sources": [SOURCE],
        "tracks": [{"key": CORE_SAFE, "initial": -1, "minimum": -1, "maximum": 1}],
        "stages": stages
    }]});
    let base = QuestStateCatalogue::empty("content-1").ok_or("fixture catalogue")?;
    Ok(Bindings::parse(&serde_json::to_vec(&data)?)?.attach(&base)?)
}

async fn exercise(admin: &str) -> TestResult {
    let h = Harness::create_with_admission_seed(admin, "yalahar_cannon_cycle", true, false).await?;
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
    let active = controller.active().ok_or("content generation")?;
    let world = super::quest_callbacks::source_world_spatial(
        CANNON,
        &[
            (CANNON.0, CANNON.1, CANNON.2, 0, 9132),
            (COLUMN.0, COLUMN.1, COLUMN.2, 0, 9133),
            (COLUMN.0 + 1, COLUMN.1, COLUMN.2, 0, 9137),
        ],
    )?;
    let start = MovementLocalPosition {
        x: i32::from(CANNON.0 - 1),
        y: i32::from(CANNON.1),
        floor: 11,
    };
    super::arena_match::fixture::runtime_at(&mut deps, active, &pin, &world, start)?;
    let cells = world
        .movement_cells(deps.room.movement_cells())
        .map_err(debug)?;
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = (&cells).into();
    owner.quest_catalogue = Some(Arc::new(catalogue()?));
    deps.initialize(&owner).await;
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18])
            .ok_or("map USE capabilities")?;
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
        .bind(active.identity().server_artifact_digest().as_slice())
        .bind(active.identity().client_artifact_digest().as_slice())
        .bind(world.frame_binding_digest(admitted.world_id).as_slice()).execute(&h.pool).await?;
    owner.admit_quest_session(&admitted).await;
    let cannon = crate::map::overlay::TilePos {
        x: CANNON.0,
        y: CANNON.1,
        floor: CANNON.2,
    };
    let column = crate::map::overlay::TilePos {
        x: COLUMN.0,
        y: COLUMN.1,
        floor: COLUMN.2,
    };
    let key = crate::map::view::placement_key(cannon, 1).ok_or("cannon")?;
    let guarded = crate::map::view::placement_key(column, 1).ok_or("column")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    for (command, tick, expected) in [(1, 0, 0), (2, 10_000_000, 1)] {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("use clock")?;
        let map = owner.bundle_map().ok_or("world scheduler map")?;
        {
            let runtime = owner.runtime.lock().await;
            runtime.owner_fence().map_err(debug)?;
            owner.advance_quest_appearances(map, tick);
        }
        assert_eq!(map.facts.object_revision(guarded), command - 1);
        let intent = encode_use_intent(&WorldObjectTarget {
            placement: placement.clone(),
            expected_revision: map.facts.object_revision(key),
        })
        .map_err(debug)?;
        let reply = wire(
            &owner,
            &mut admitted,
            command,
            COMMAND_TYPE_USE_INTENT,
            intent,
        )
        .await?;
        assert_eq!(
            decode_use_result(&reply).map_err(debug)?,
            UseDisposition::Committed
        );
        let state = h
            .root
            .read_character_quest_state(&authority, fence.character_id)
            .await
            .map_err(debug)?;
        assert_eq!(state.tracks().get(CORE_SAFE), Some(&expected));
    }
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}
