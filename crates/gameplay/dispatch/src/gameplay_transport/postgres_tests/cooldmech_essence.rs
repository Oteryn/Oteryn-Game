//! Essence portal cooldown through the production STEP dispatch.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, quest_world_callbacks as world};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, encode_step_intent,
};

pub(super) const NAME: &str = "cooldmech-essence";
const TRACK: &str = "oteryn:quest-progress/cults-of-tibia/essence-entered-at";
const ENTRY: Position = Position {
    x: 33096,
    y: 31963,
    floor: -15,
};
const LANDING: Position = Position {
    x: 33095,
    y: 31953,
    floor: -15,
};

#[test]
fn cooldmech_essence_step_reopens_after_twenty_hours_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run(NAME, || world::scenario(NAME, ENTRY, false))
}

pub(super) fn geometry(trigger: Position) -> TestResult<Callback> {
    let mut fixture = super::authored_world_rewards::placeholder(trigger)?;
    fixture.selector.appearance = Some(1949);
    fixture.selector.unique = None;
    fixture.teleport = Some(LANDING);
    Ok(fixture)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    origin: Position,
) -> TestResult {
    let epoch = 1_700_000_000;
    harness.root.set_test_quest_clock(epoch)?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let session = admitted.game_session_id;
    for (command, advance, expected, stamp) in [
        (1, 0, LANDING, epoch),
        (2, 71_999, origin, epoch),
        (3, 1, LANDING, epoch + 72_000),
    ] {
        harness.root.advance_test_quest_clock(advance)?;
        owner.clock_origin -= std::time::Duration::from_secs(2);
        world::wire(
            owner,
            admitted,
            command,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            encode_step_intent(StepDirection::East),
        )
        .await?;
        let mut runtime = owner.runtime.lock().await;
        let at = runtime.read_actor_position(actor).map_err(debug)?;
        assert_eq!(at.position(), world::local(expected));
        if expected == LANDING {
            let map = owner.bundle_map().ok_or("map")?;
            let proof = crate::map::floor::prepare_quest_callback_teleport(
                &runtime,
                owner.movement_cells(),
                actor,
                session,
                at,
                world::local(origin),
                false,
                &crate::map::floor::BundleDoors(map.facts.doors()),
            )
            .map_err(debug)?;
            runtime.commit_source_step(proof).map_err(debug)?;
        }
        drop(runtime);
        let stored: i64 =
            sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
                .bind(TRACK)
                .fetch_one(&harness.pool)
                .await?;
        assert_eq!(stored, stamp);
    }
    Ok(())
}
