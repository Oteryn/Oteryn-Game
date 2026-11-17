//! Cursed Crystal: unmodified compiled source markers enter ordinary STEP dispatch.
use super::TestResult;
use super::fixture::{Harness, debug};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::foundation::ExactActorRef;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position, Trigger};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};

// Crystal 201189f movements_StepIn_CursedCrystal.lua:29,31,33,35.
// Position is the physical Bundle tile; the common harness owns runtime conversion.
type Route = (&'static str, Position, Position);
const fn at(x: u16, y: u16, floor: i8) -> Position {
    Position { x, y, floor }
}
const ROUTES: [Route; 4] = [
    (
        "crystal-stairs-29",
        at(31973, 32905, -11),
        at(31971, 32904, -10),
    ),
    (
        "crystal-stairs-31",
        at(31973, 32905, -10),
        at(31975, 32911, -9),
    ),
    (
        "crystal-stairs-33",
        at(32009, 32928, -9),
        at(32009, 32929, -10),
    ),
    (
        "crystal-stairs-35",
        at(32009, 32928, -10),
        at(32009, 32929, -9),
    ),
];

#[test]
fn cursed_crystal_source_stairs_step_replay_and_wrong_action_controls() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("cursed-crystal-stairs", || {
        for (name, trigger, _) in ROUTES {
            // Only the negative worlds change an existing compiled marker. The
            // admitted production callback catalogue is identical in every case.
            for action in [None, Some(25017), Some(65535)] {
                super::quest_world_callbacks::cursed_stairs_scenario(name, trigger, action)?;
                eprintln!("CURSED_STAIRS_POSTGRES_CASE_COMPLETED route={name} action={action:?}");
            }
        }
        eprintln!("CURSED_STAIRS_POSTGRES_COMPLETED cases=12");
        Ok(())
    })
}

pub(super) fn world(
    callback: &Callback,
    native: &crate::content::native_gameplay::NativeGameplayState,
    action_control: Option<u16>,
) -> TestResult<crate::map::boot::BundleWorld> {
    let from = callback.selector.position.ok_or("stair source")?;
    let to = callback.teleport.ok_or("stair destination")?;
    let tile = |x, y, floor| crate::map::overlay::TilePos { x, y, floor };
    let start = tile(from.x - 1, from.y, from.floor);
    let mut route_cells = vec![
        start,
        tile(from.x, from.y, from.floor),
        tile(to.x, to.y, to.floor),
    ];
    if let Some((_, opposite, landing)) = ROUTES
        .iter()
        .find(|(_, opposite, _)| opposite.x == from.x && opposite.floor == to.floor)
        .filter(|_| from.x == 32009)
    {
        route_cells.push(tile(opposite.x, opposite.y, opposite.floor));
        route_cells.push(tile(landing.x, landing.y, landing.floor));
    }
    super::source_world_crop::compiled_region_action_control(
        tile(
            from.x.min(to.x) - 2,
            from.y.min(to.y) - 2,
            from.floor.min(to.floor),
        ),
        tile(
            from.x.max(to.x) + 2,
            from.y.max(to.y) + 2,
            from.floor.max(to.floor),
        ),
        start,
        native,
        Some(&route_cells),
        action_control.map(|action| (tile(from.x, from.y, from.floor), action)),
    )
}

async fn step(
    owner: &super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut super::AdmittedSession,
    command: u64,
    direction: StepDirection,
) -> TestResult {
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(direction),
    )
    .await?;
    assert_eq!(
        decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)] // Mirrors the shared composed callback harness.
pub(super) async fn exercise(
    owner: &super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut super::AdmittedSession,
    h: &Harness,
    actor: ExactActorRef,
    fence: CurrentCharacterGameplayFence,
    callback: &Callback,
    action_control: Option<u16>,
) -> TestResult {
    let (_, from, to) = ROUTES
        .iter()
        .find(|(name, _, _)| callback.key.ends_with(*name))
        .copied()
        .ok_or("literal stair route")?;
    assert_eq!(callback.trigger, Trigger::StepIn);
    assert_eq!(callback.selector.position, Some(from));
    assert_eq!(callback.selector.action, Some(25018));
    assert_eq!(callback.teleport, Some(to));
    assert_eq!(callback.quest, "oteryn:quest.tibia_tales");
    assert!(callback.storage.is_empty());
    let map = owner.bundle_map().ok_or("source map")?;
    assert!(map.facts.movement_effects_since(0).is_empty());
    let before = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    let receipts_before = receipts(h).await?;

    step(owner, admitted, 1, StepDirection::East).await?;
    let snapshot = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?;
    assert_eq!(
        snapshot.position(),
        super::quest_world_callbacks::local(if action_control.is_none() { to } else { from }),
        "source AID {action_control:?}: {}",
        callback.key
    );
    let effects = map.facts.movement_effects_since(0);
    if action_control.is_none() {
        assert_eq!(effects.len(), 1, "one stair arrival effect");
        assert_eq!(
            (
                effects[0].1.x,
                effects[0].1.y,
                effects[0].1.floor,
                effects[0].2
            ),
            (to.x, to.y, to.floor, 11)
        );
    } else {
        assert!(
            effects.is_empty(),
            "wrong source AID must not select the stair callback"
        );
    }

    // A replay enters the same real command dispatcher, with its original ID.
    step(owner, admitted, 1, StepDirection::East).await?;
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?,
        snapshot,
        "STEP replay moved or turned the actor twice"
    );
    assert_eq!(
        map.facts.movement_effects_since(0),
        effects,
        "STEP replay duplicated a stair effect"
    );

    if action_control.is_none() && from.x == 32009 {
        // The paired source stairs are directly north of each landing. Two new
        // commands re-enter both physical markers without test-position resets.
        let opposite = ROUTES
            .iter()
            .find(|(_, p, _)| p.x == from.x && p.floor == to.floor)
            .map(|(_, _, destination)| *destination)
            .ok_or("paired stair")?;
        step(owner, admitted, 2, StepDirection::North).await?;
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            super::quest_world_callbacks::local(opposite)
        );
        step(owner, admitted, 3, StepDirection::North).await?;
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            super::quest_world_callbacks::local(to)
        );
        assert_eq!(
            map.facts.movement_effects_since(0).len(),
            3,
            "each fresh stair entry emits exactly one effect"
        );
    }
    let after = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(
        after.tracks(),
        before.tracks(),
        "stateless stairs wrote quest progress"
    );
    assert_eq!(
        receipts(h).await?,
        receipts_before,
        "stateless stairs wrote durable receipts"
    );
    Ok(())
}

async fn receipts(h: &Harness) -> TestResult<(i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_receipts), (SELECT count(*) FROM game_quest_callback_consumptions)")
        .fetch_one(&h.pool).await?)
}
