//! Crystal 201189f8 data-global/scripts/lib/register_actions.lua:1035-1067.
use super::fixture::{Harness, debug, id};
use super::quest_world_callbacks::wire;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, authored_bindings::Bindings};
use oteryn_protocol_oteryn::world_object::{
    COMMAND_TYPE_USE_INTENT, MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
    decode_use_result, encode_item_use_intent,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.fixture_library_scythe";
const PERMISSION: &str = "oteryn:quest-progress/fixture-library/permission";
const KEY: &str = "oteryn:quest-callback/fixture-library/scythe";
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/lib/register_actions.lua:1035-1067";
const ENTRY: (u16, u16, i8) = (32177, 31925, -7);
// Synthetic landing matches the fixture's small World slice, on the donor floor.
const LANDING: (u16, u16, i8) = (32177, 31925, -12);
const SCYTHES: [&str; 2] = ["oteryn:item.tibia.i3453", "oteryn:item.tibia.i9596"];

#[test]
fn library_scythe_dispatch_retains_either_tool_and_requires_permission_seven() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::kill_reward_e2e::scenario_thread::run_async(async |admin| {
        exercise(&admin, SCYTHES[0], 6).await?;
        exercise(&admin, SCYTHES[0], 7).await?;
        exercise(&admin, SCYTHES[1], 7).await
    })
}

fn catalogue(permission: i64) -> TestResult<QuestStateCatalogue> {
    let data = json!({"schema": 1, "quests": [{
        "quest": QUEST,
        "recipe_sha256": Sha256::digest(SOURCE).iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
        "sources": [SOURCE],
        "tracks": [{"key": PERMISSION, "initial": permission, "minimum": -1, "maximum": 7}],
        "stages": [{
            "key": "oteryn:quest-stage/fixture-library/scythe",
            "completes": false,
            "trigger": {"kind": "map_use", "callback": {
                "key": KEY, "quest": QUEST, "trigger": "use",
                "selector": {"position": {"x": ENTRY.0, "y": ENTRY.1, "floor": ENTRY.2}, "ordinal": 1},
                "reversible": false, "passages": [], "offerings": [], "return_on_failure": false,
                "requires_world_admission": true,
                "storage": [{"track": PERMISSION, "minimum": 7, "maximum": 7, "set": 7}],
                "teleport": {"x": LANDING.0, "y": LANDING.1, "floor": LANDING.2},
                "effects": {"teleport_effect": 11, "refused_tool_effect": 3},
                "item_on_target": {"item": SCYTHES[0], "alternatives": [SCYTHES[1]], "count": 1,
                                   "reusable": true, "selected_instance": true},
                "sources": [SOURCE]
            }}
        }]
    }]});
    let base = QuestStateCatalogue::empty("content-1").ok_or("fixture catalogue")?;
    Ok(Bindings::parse(&serde_json::to_vec(&data)?)?.attach(&base)?)
}

async fn exercise(admin: &str, tool: &str, permission: i64) -> TestResult {
    let h =
        Harness::create_with_admission_seed(admin.to_owned(), "library_scythe_route", true, false)
            .await?;
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
        ENTRY,
        &[
            (ENTRY.0, ENTRY.1, ENTRY.2, 0, 1948),
            (LANDING.0, LANDING.1, LANDING.2, 0, 1948),
        ],
    )?;
    let start = MovementLocalPosition {
        x: i32::from(ENTRY.0 - 1),
        y: i32::from(ENTRY.1),
        floor: 7,
    };
    super::arena_match::fixture::runtime_at(&mut deps, active, &pin, &world, start)?;
    let cells = world
        .movement_cells(deps.room.movement_cells())
        .map_err(debug)?;
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = (&cells).into();
    owner.quest_catalogue = Some(Arc::new(catalogue(permission)?));
    deps.initialize(&owner).await;
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18])
            .ok_or("item USE capabilities")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest session")?;
    admitted.item_fence = Some(fence.item_fence());
    super::item_look::seed_main_backpack(&h, &[(tool, 1)]).await?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(active.identity().server_artifact_digest().as_slice())
        .bind(active.identity().client_artifact_digest().as_slice())
        .bind(world.frame_binding_digest(admitted.world_id).as_slice()).execute(&h.pool).await?;
    owner.admit_quest_session(&admitted).await;

    let definition = active
        .native_gameplay()
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", tool, "definition-r1"))
        .ok_or("scythe definition")?;
    let source = crate::map::overlay::TilePos {
        x: ENTRY.0,
        y: ENTRY.1,
        floor: ENTRY.2,
    };
    let key = crate::map::view::placement_key(source, 1).ok_or("scythe target")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let intent = encode_item_use_intent(&UseIntent {
        target: UseTarget::ItemByDefinition(definition),
        use_with: None,
        use_with_position: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement,
            expected_revision: 0,
        })),
    })
    .map_err(debug)?;
    let reply = wire(&owner, &mut admitted, 1, COMMAND_TYPE_USE_INTENT, intent).await?;
    let allowed = permission == 7;
    assert_eq!(
        decode_use_result(&reply).map_err(debug)?,
        if allowed {
            UseDisposition::Committed
        } else {
            UseDisposition::Rejected
        }
    );
    let park = MovementLocalPosition {
        x: i32::from(LANDING.0),
        y: i32::from(LANDING.1),
        floor: 12,
    };
    assert_eq!(
        deps.runtime
            .lock()
            .await
            .read_actor_position(deps.actor)
            .map_err(debug)?
            .position(),
        if allowed { park } else { start }
    );
    let (retained_reads, consumed): (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_quest_world_tool_reads WHERE transition_key = $1),
                (SELECT count(*) FROM game_quest_callback_consumptions WHERE transition_key = $1)",
    )
    .bind(KEY)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!((retained_reads, consumed), (i64::from(allowed), 0));
    let map = owner.bundle_map().ok_or("bundle map")?;
    let cues = map.facts.movement_effects_since(0);
    assert_eq!(
        cues.last().map(|(_, _, effect)| *effect),
        Some(if allowed { 11 } else { 3 })
    );
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}
