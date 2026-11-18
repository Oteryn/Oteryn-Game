//! Crystal 201189f8 data-global/scripts/quests/kilmaresh/1-fafnars-wrath/6-mind-travel.lua:1-39.
use super::fixture::{Harness, debug, id};
use super::quest_world_callbacks::{local, wire};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, authored_bindings::Bindings, world_callbacks::Position};
use oteryn_protocol_oteryn::world_object::{
    COMMAND_TYPE_USE_INTENT, UseDisposition, UseIntent, UseTarget, decode_use_result, encode_item_use_intent,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.fixture_kilmaresh_potion";
const KEY: &str = "oteryn:quest-reward/fixture-kilmaresh/mind-potion";
const STATE: &str = "oteryn:quest-progress/fixture-kilmaresh/mind-entered";
const DRAW: &str = "oteryn:quest-progress/fixture-kilmaresh/side";
const POTION: &str = "oteryn:item.tibia.i31350";
const RING: &str = "oteryn:item.tibia.i31306";
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/kilmaresh/1-fafnars-wrath/6-mind-travel.lua:1-39";

const START: Position = Position { x: 33865, y: 31533, floor: -7 };
const LANDINGS: [Position;2] = [
    Position { x: 33913, y: 31653, floor: -9 },
    Position { x: 33921, y: 31650, floor: -9 },
];

#[test]
fn hallucinogen_use_checks_the_ring_slot_and_lands_on_the_persisted_brain_side() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::kill_reward_e2e::scenario_thread::run_async(exercise)
}

fn catalogue() -> TestResult<QuestStateCatalogue> {
    let data = json!({"schema": 1, "quests": [{
        "quest": QUEST, "recipe_sha256": Sha256::digest(SOURCE).iter().map(|b| format!("{b:02x}")).collect::<String>(),
        "sources": [SOURCE],
        "tracks": [
            {"key": STATE, "initial": -1, "minimum": -1, "maximum": 1},
            {"key": DRAW, "initial": -1, "minimum": -1, "maximum": 2}
        ],
        "stages": [{
            "key": "oteryn:quest-stage/fixture-kilmaresh/mind-potion", "completes": false,
            "trigger": {"kind": "carried_use", "callback": {
                "key": KEY, "quest": QUEST, "npc": "oteryn:npc.faloriel",
                "keywords": [], "topic": 0, "next_topic": 0, "carried_item": POTION,
                "take": [{"item": POTION, "count": 1}],
                "storage": [{"track": STATE, "minimum": -1, "maximum": 1, "set": 1}],
                "state_ops": [{"track": DRAW, "from": {"kind": "any"}, "effect": {"kind": "draw", "sides": 2}}],
                "self_target": {"minimum": {"x": 33857, "y": 31526, "floor": -7},
                                "maximum": {"x": 33873, "y": 31540, "floor": -7}},
                "carried_relocation": {"ring": RING, "destinations": LANDINGS, "effect": 38},
                "reply": ["You feel shaky and dizzy, the world turns dark around you. Then your sight clears again - and you are somewhere else."],
                "sources": [SOURCE]
            }}
        }]
    }]});
    let base = QuestStateCatalogue::empty("content-1").ok_or("fixture catalogue")?;
    Ok(Bindings::parse(&serde_json::to_vec(&data)?)?.attach(&base)?)
}

async fn exercise(admin: String) -> TestResult {
    let h = Harness::create_with_admission_seed(admin, "mind_potion", true, false).await?;
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
    let mut deps = Dependencies::new()?;
    let (controller, pin) = super::death_ml_loss_composed::production_generation_with_pin(
        deps.admitted.world_id, "content-1",
    )?;
    let generation = controller.active().ok_or("content generation")?;
    let tiles: Vec<_> = std::iter::once(START).chain(LANDINGS)
        .map(|p| (p.x, p.y, p.floor, 0, 1948)).collect();
    let world = super::quest_callbacks::source_world_spatial((START.x, START.y, START.floor), &tiles)?;
    super::arena_match::fixture::runtime_at(&mut deps, generation, &pin, &world, local(START))?;
    let cells = world.movement_cells(deps.room.movement_cells()).map_err(debug)?;
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = Some(generation);
    owner.movement_cells = (&cells).into();
    owner.quest_catalogue = Some(Arc::new(catalogue()?));
    deps.initialize(&owner).await;
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18])
            .ok_or("USE capabilities")?;
    let fence = owner.current_quest_fence(admitted.game_session_id).await
        .map_err(|()| "quest fence")?.ok_or("quest session")?;
    admitted.item_fence = Some(fence.item_fence());
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(generation.identity().server_artifact_digest().as_slice())
        .bind(generation.identity().client_artifact_digest().as_slice())
        .bind(world.frame_binding_digest(admitted.world_id).as_slice()).execute(&h.pool).await?;
    super::item_look::seed_main_backpack(&h, &[(POTION, 1), (RING, 1)]).await?;
    owner.admit_quest_session(&admitted).await;
    let potion = generation.native_gameplay().and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", POTION, "definition-r1")).ok_or("potion definition")?;
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::ItemByDefinition(potion), use_with: None, use_with_map_item: None,
        use_with_position: None,
    }).map_err(debug)?;
    let rejected = wire(&owner, &mut admitted, 1, COMMAND_TYPE_USE_INTENT, payload.clone()).await?;
    assert_eq!(decode_use_result(&rejected).map_err(debug)?, UseDisposition::Rejected);
    assert_eq!(owner.runtime.lock().await.read_actor_position(deps.actor).map_err(debug)?.position(), local(START));
    {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
        sqlx::query(
            "WITH ring AS (
                DELETE FROM game_item_container_entries e USING game_item_instances i
                WHERE e.item_instance_id=i.item_instance_id AND i.definition_production_key=$1
                RETURNING e.character_id,e.item_instance_id,e.world_id,e.placed_transaction_id
             ) INSERT INTO game_character_equipment_slots(character_id,slot,item_instance_id,world_id,placed_transaction_id)
             SELECT character_id,8,item_instance_id,world_id,placed_transaction_id FROM ring"
        ).bind(RING).execute(&mut *tx).await?;
        tx.commit().await?;
    }
    let committed = wire(&owner, &mut admitted, 2, COMMAND_TYPE_USE_INTENT, payload.clone()).await?;
    assert_eq!(decode_use_result(&committed).map_err(debug)?, UseDisposition::Committed);
    let state = h.root.read_character_quest_state(&authority, fence.character_id).await.map_err(debug)?;
    let selected = *state.tracks().get(DRAW).ok_or("persisted brain side")?;
    let expected = LANDINGS.get(usize::try_from(selected - 1)?).copied().ok_or("bounded brain side")?;
    assert_eq!(owner.runtime.lock().await.read_actor_position(deps.actor).map_err(debug)?.position(), local(expected));
    assert_eq!(wire(&owner, &mut admitted, 2, COMMAND_TYPE_USE_INTENT, payload).await?, committed);
    let (receipts, remaining): (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1),
                (SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$2)"
    ).bind(KEY).bind(POTION).fetch_one(&h.pool).await?;
    assert_eq!((receipts, remaining), (1, 0));
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}
