//! Crystal 201189f dreamers_challenge_quest/movement_tic_teleport.lua:1-49.
use super::fixture::{Harness, debug};
use super::quest_world_callbacks::{input_item, local, scenario, wire, world_cells};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::map::{
    overlay::TilePos,
    view::{MapFacts, placement_key},
};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    world_callbacks::{self, Callback, Position},
};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};
use std::sync::Arc;

pub(super) const NAME: &str = "dreamers-board-exit";
const PORTAL: Position = at(32836, 32288);
const EXIT: Position = at(32874, 32275);
const LEVERS: [Position; 2] = [at(32836, 32278), at(32834, 32285)];

const fn at(x: u16, y: u16) -> Position {
    Position { x, y, floor: -14 }
}

pub(super) fn binding() -> TestResult<Callback> {
    let passages = LEVERS.map(|position| {
        serde_json::json!({
            "position":position, "appearance":2773, "replacement":2772, "if_present":true
        })
    });
    // This test generation serves stackable apples. Bind that native fixture
    // definition to the donor's nine board cells for the custody proof.
    let offerings = super::dreamers_board_portal::TOKENS.map(|(position, _)| {
        serde_json::json!({
            "position":position, "item":"oteryn:item.tibia.i3585", "count":1
        })
    });
    Ok(serde_json::from_value(serde_json::json!({
        "key":"oteryn:quest-callback/ground-cleanup/dreamers-challenge-board-exit",
        "quest":"oteryn:quest.dreamer_s_challenge_quest", "trigger":"step_in",
        "selector":{"position":PORTAL, "appearance":1949},
        "teleport":EXIT,"return_on_failure":true,"reversible":false,
        "requires_world_admission":true,
        "storage":[],"offerings":offerings,"passages":passages,
        "effects":{"cleanup_offerings":true,"portal_reset_passable":true,"transform":6298,"teleport_effect":11},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/dreamers_challenge_quest/movement_tic_teleport.lua:1-49"]
    }))?)
}

pub(super) fn world(callback: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    world_cells(callback, PORTAL, 1949, 1, &[], &[6298], None, &[])
}

pub(super) fn catalogue(
    base: &QuestStateCatalogue,
    callback: &Callback,
) -> TestResult<Arc<QuestStateCatalogue>> {
    let binding = serde_json::json!({"schema":1,"callbacks":[callback]}).to_string();
    let base = base
        .with_fixture_transition(world_callbacks::primary_transition(callback)?)
        .map_err(debug)?;
    Ok(Arc::new(world_callbacks::with_fixture_world_document(
        &base, &binding,
    )?))
}

#[test]
fn dreamers_tic_tac_exit_retires_present_token_stacks_and_resets_both_levers() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || scenario(NAME, PORTAL, false))
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    fence: CurrentCharacterGameplayFence,
    callback: &Callback,
) -> TestResult {
    let first = callback.offerings.first().ok_or("first token slot")?;
    let mut stack = first.clone();
    stack.count = 2;
    input_item(owner, fence.item_fence(), &stack, 9, false).await?;
    let response = wire(
        owner,
        admitted,
        1,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(StepDirection::East),
    )
    .await?;
    assert_eq!(
        decode_step_result(&response).map_err(debug)?,
        StepDisposition::Moved
    );
    let actor = admitted.runtime_actor.ok_or("actor")?;
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        local(EXIT)
    );
    let map = owner.bundle_map().ok_or("map")?;
    for (position, appearance) in LEVERS
        .into_iter()
        .map(|p| (p, 2772))
        .chain([(PORTAL, 6298)])
    {
        let pos = TilePos {
            x: position.x,
            y: position.y,
            floor: position.floor,
        };
        let id = map
            .overlay
            .base()
            .tile(pos.x, pos.y, pos.floor)
            .ok_or("tile")?
            .ids()[1];
        assert_eq!(
            map.facts
                .base_entry(pos, 1, id)
                .ok_or("object")?
                .appearance_id,
            appearance
        );
    }
    // Crystal movement_tic_teleport.lua:2,27-33,48-50 transforms the traversed
    // portal to a walkable crack. The immutable map still advertises 1949.
    let portal = TilePos {
        x: PORTAL.x,
        y: PORTAL.y,
        floor: PORTAL.floor,
    };
    let key = placement_key(portal, 1).ok_or("portal key")?;
    let reset_revision = map.facts.object_revision(key);
    {
        let mut runtime = owner.runtime.lock().await;
        let proof = crate::map::floor::prepare_quest_callback_teleport(
            &runtime,
            owner.movement_cells(),
            actor,
            admitted.game_session_id,
            runtime.read_actor_position(actor).map_err(debug)?,
            local(Position {
                x: PORTAL.x - 1,
                ..PORTAL
            }),
            false,
            &crate::map::floor::BundleDoors(&map.facts),
        )
        .map_err(debug)?;
        runtime.commit_source_step(proof).map_err(debug)?;
    }
    let response = wire(
        owner,
        admitted,
        2,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(StepDirection::East),
    )
    .await?;
    assert_eq!(
        decode_step_result(&response).map_err(debug)?,
        StepDisposition::Moved
    );
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        local(PORTAL),
    );
    assert_eq!(map.facts.object_revision(key), reset_revision);

    let removed: Vec<(i32, i32)> = sqlx::query_as(
        "SELECT c.quantity, i.lifecycle::integer FROM game_quest_callback_consumptions c
         JOIN game_item_instances i USING(item_instance_id) WHERE c.transition_key=$1",
    )
    .bind(&callback.key)
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(removed, [(2, 2)]);
    Ok(())
}
