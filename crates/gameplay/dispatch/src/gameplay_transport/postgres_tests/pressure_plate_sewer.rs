//! Crystal movements_sewer.lua:1-45: uid25030 creates/removes one sewer435.
use super::TestResult;
use super::fixture::{Harness, debug};
use super::quest_world_callbacks::{local, wire};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::foundation::ExactActorRef;
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::AdmittedSession;
use crate::map::overlay::TilePos;
use crate::map::view::{MapFacts, compose_tile};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_spatial::{
    StepDirection, StepDisposition, decode_step_result, encode_step_intent,
};

#[test]
fn devil_helmet_pressure_plate_creates_sewer_then_removes_it_on_exit_and_replay() -> TestResult {
    scenario("devil-helmet-sewer")
}

#[test]
fn devil_helmet_pressure_plate_preserves_existing_sewer_until_exit() -> TestResult {
    scenario("devil-helmet-sewer-existing")
}

#[test]
fn devil_helmet_pressure_plate_ignores_unrelated_unique_id() -> TestResult {
    scenario("devil-helmet-sewer-unrelated")
}

fn scenario(name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("devil-helmet-sewer", move || {
        super::quest_world_callbacks::scenario(
            name,
            Position {
                x: 32480,
                y: 32170,
                floor: -14,
            },
            false,
        )
    })?;
    println!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    actor: ExactActorRef,
    fence: CurrentCharacterGameplayFence,
    trigger: Position,
    name: &str,
) -> TestResult {
    let position = TilePos {
        x: 32482,
        y: 32170,
        floor: -14,
    };
    assert_eq!(
        sewers(owner, position)?,
        usize::from(name.ends_with("existing")),
        "initial source sewer"
    );
    let before = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    for (command, direction, expected_count) in [
        (1, StepDirection::East, 1),
        (2, StepDirection::West, 0),
        (3, StepDirection::East, 1),
        (4, StepDirection::West, 0),
    ] {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let payload = encode_step_intent(direction);
        let reply = wire(owner, admitted, command, 1, payload.clone()).await?;
        assert_eq!(
            decode_step_result(&reply).map_err(debug)?,
            StepDisposition::Moved
        );
        let expected_count = if name.ends_with("unrelated") {
            0
        } else {
            expected_count
        };
        assert_eq!(
            sewers(owner, position)?,
            expected_count,
            "source sewer after command {command}"
        );
        let expected_position = if command % 2 == 1 {
            trigger
        } else {
            Position {
                x: trigger.x - 1,
                ..trigger
            }
        };
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            local(expected_position)
        );
        let revision = owner
            .bundle_map()
            .ok_or("map")?
            .facts
            .movement_items(position)
            .first()
            .map(|entry| entry.source);
        assert_eq!(wire(owner, admitted, command, 1, payload).await?, reply);
        assert_eq!(sewers(owner, position)?, expected_count);
        assert_eq!(
            owner
                .bundle_map()
                .ok_or("map")?
                .facts
                .movement_items(position)
                .first()
                .map(|entry| entry.source),
            revision,
            "replay retains the same physical object"
        );
    }
    assert_eq!(
        owner
            .root
            .read_character_quest_state(owner.character, fence.character_id)
            .await
            .map_err(debug)?,
        before
    );
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        after, receipts,
        "physical plate must not invent quest progress"
    );
    Ok(())
}

fn sewers(owner: &ComposedFreshAdmission<'_, '_, '_>, position: TilePos) -> TestResult<usize> {
    let map = owner.bundle_map().ok_or("map")?;
    let tile = compose_tile(&map.overlay, &map.facts, position)
        .map_err(debug)?
        .ok_or("sewer tile")?;
    Ok(tile
        .entries
        .iter()
        .filter(|entry| entry.facts.appearance_id == 435)
        .count())
}
