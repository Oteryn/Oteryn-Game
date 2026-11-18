//! The wire item-move caller drives both halves of the basin transaction.
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::item_loop_move::TOMB_BASIN_COIN;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::item_view::{ItemMoveOutcome, WorldTilePosition};

pub(super) const BASIN: Position = Position { x: 33098, y: 32816, floor: -13 };
pub(super) const ARRIVAL: Position = Position { x: 33093, y: 32824, floor: -13 };

#[test]
fn tombs_basin_commits_coin_and_flame_occupant_arrival() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()); };
    scenario("tombs-basin-commit")
}

#[test]
fn tombs_basin_rolls_back_coin_when_arrival_insert_fails() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()); };
    scenario("tombs-basin-rollback")
}

fn scenario(name: &'static str) -> TestResult {
    super::kill_reward_e2e::scenario_thread::run(name, || {
        super::quest_world_callbacks::scenario(name, BASIN, false)
    })
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &super::fixture::Harness,
    fence: CurrentCharacterGameplayFence,
    rollback: bool,
) -> TestResult {
    use crate::gameplay_transport::connection::FreshAdmissionAuthority;
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 12, 14, 15, 18, oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_STACK_COUNT_V1],
        ).ok_or("coal basin item move capabilities")?;
    super::quest_world_callbacks::input_item(owner, fence.item_fence(), &Offering {
        position: BASIN,
        item: TOMB_BASIN_COIN.into(),
        count: 1,
    }, 60, true).await?;
    if rollback {
        sqlx::raw_sql("CREATE FUNCTION fail_tombs_basin_arrival() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected tomb basin arrival failure' USING ERRCODE='23514'; END; $$; CREATE TRIGGER fail_tombs_basin_arrival BEFORE INSERT ON game_character_pending_arrivals FOR EACH ROW EXECUTE FUNCTION fail_tombs_basin_arrival()")
            .execute(&h.pool).await?;
    }
    let reference = owner.active_generation.and_then(|g| g.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", TOMB_BASIN_COIN, "definition-r1"))
        .ok_or("scarab coin reference")?;
    let (next, outcome, _) = super::map_seed_planting::drop_exchange_at(
        owner, *admitted, 1, None, reference, 1, false,
        WorldTilePosition { x: i32::from(BASIN.x), y: i32::from(BASIN.y), floor: i16::from(BASIN.floor) },
    ).await?;
    *admitted = next;
    if rollback {
        assert_eq!(outcome, Some(ItemMoveOutcome::Rejected));
    } else {
        assert_eq!(outcome, Some(ItemMoveOutcome::Moved));
    }
    let result: (i64, i64, i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_tomb_basin_offerings),
            (SELECT count(*) FROM game_character_pending_arrivals),
            (SELECT count(*) FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1),
            (SELECT count(*) FROM game_item_loop_move_receipts WHERE command_id=1),
            (SELECT count(*) FROM game_character_quest_tracks
              WHERE track_key='oteryn:quest-progress/quest/u7_4/the_ancient_tombs/default_start' AND value=1)"
    ).bind(TOMB_BASIN_COIN).fetch_one(&h.pool).await?;
    assert_eq!(result, if rollback { (0, 0, 1, 0, 0) } else { (1, 0, 0, 1, 1) });
    let actor = admitted.runtime_actor.ok_or("flame occupant")?;
    let position = owner.runtime.lock().await.read_actor_position(actor).map_err(super::fixture::debug)?.position();
    let x = if rollback { BASIN.x - 1 } else { ARRIVAL.x };
    let y = if rollback { BASIN.y } else { ARRIVAL.y };
    assert_eq!((position.x, position.y, position.floor), (i32::from(x), i32::from(y), 13));
    Ok(())
}
