//! Essence portal cooldown through the production STEP dispatch.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, quest_world_callbacks as world};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, encode_step_intent,
};

pub(super) const NAME: &str = "cooldmech-essence";
const BOSS: &str = "oteryn:creature.essence_of_malice";
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
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let session = admitted.game_session_id;
    let character = owner
        .current_quest_fence(session)
        .await
        .map_err(debug)?
        .ok_or("fence")?
        .character_id;
    for (command, expected) in [(1, LANDING), (2, origin), (3, LANDING)] {
        if command == 2 {
            sqlx::query(
                "INSERT INTO game_character_boss_cooldowns
                (character_id,boss_key,available_at,admission,reset_epoch)
                VALUES(encode($1,'hex')::uuid,$2,
                  floor(extract(epoch FROM statement_timestamp()) * 1000)::bigint + 72000000,
                  game_character_uuid_v7(),0)",
            )
            .bind(character.as_bytes().as_slice())
            .bind(BOSS)
            .execute(&harness.pool)
            .await?;
        } else if command == 3 {
            sqlx::query(
                "UPDATE game_character_boss_cooldowns SET available_at=0
                WHERE character_id=encode($1,'hex')::uuid AND boss_key=$2",
            )
            .bind(character.as_bytes().as_slice())
            .bind(BOSS)
            .execute(&harness.pool)
            .await?;
        }
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
            let closed = crate::map::floor::BundleDoors(map.facts.doors());
            let proof = crate::map::floor::prepare_quest_callback_teleport(
                &runtime,
                owner.movement_cells(),
                actor,
                session,
                at,
                world::local(origin),
                false,
                &closed,
            )
            .map_err(debug)?;
            runtime.commit_source_step(proof).map_err(debug)?;
        }
        drop(runtime);
        let rows: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_boss_cooldowns
             WHERE character_id=encode($1,'hex')::uuid AND boss_key=$2",
        )
        .bind(character.as_bytes().as_slice())
        .bind(BOSS)
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(
            rows,
            i64::from(command != 1),
            "the portal never starts a cooldown"
        );
    }
    Ok(())
}
