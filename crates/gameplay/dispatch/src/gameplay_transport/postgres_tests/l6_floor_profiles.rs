//! Physical admission only: all source parts and the uncharged helmet retain real custody.
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};

const TILE: Position = Position {
    x: 7,
    y: 1,
    floor: -7,
};

#[test]
fn l6_floor_profiles_eight_exact_items_survive_custody_drop_and_replay() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED l6_floor_profiles");
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            super::quest_world_callbacks::scenario("l6-floor-profiles", TILE, false)
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "floor profile scenario panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED l6_floor_profiles duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    fence: CurrentCharacterGameplayFence,
) -> TestResult {
    eprintln!("POSTGRES_DATABASE_READY l6_floor_profiles");
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[
                4,
                6,
                11,
                12,
                14,
                15,
                18,
                oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_STACK_COUNT_V1,
            ],
        )
        .ok_or("profile custody capabilities")?;
    let keys: Vec<_> = (3222..=3229)
        .map(|id| format!("oteryn:item.tibia.i{id}"))
        .collect();
    for (index, key) in keys.iter().enumerate() {
        super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            &Offering {
                position: TILE,
                item: key.clone(),
                count: 1,
            },
            60 + u64::try_from(index)?,
            true,
        )
        .await?;
    }
    let reference = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|n| n.item_index())
        .and_then(|i| i.definition_ref("Item", "oteryn:item.tibia.i3229", "definition-r1"))
        .ok_or("helmet reference")?;
    let destination = oteryn_protocol_oteryn::item_view::WorldTilePosition {
        x: 7,
        y: 1,
        floor: -7,
    };
    let (next, result, payload) = super::map_seed_planting::drop_exchange_at(
        owner,
        *admitted,
        1,
        None,
        reference,
        1,
        false,
        destination,
    )
    .await?;
    *admitted = next;
    assert_eq!(
        result,
        Some(oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved)
    );
    let (next, replay, _) = super::map_seed_planting::drop_exchange_at(
        owner,
        *admitted,
        1,
        Some(payload),
        reference,
        1,
        false,
        destination,
    )
    .await?;
    *admitted = next;
    assert_eq!(replay, result);
    let actual: Vec<(String, i64)> = sqlx::query_as("SELECT definition_production_key,quantity FROM game_item_instances WHERE definition_production_key=ANY($1::text[]) AND lifecycle=1 ORDER BY definition_production_key")
        .bind(&keys).fetch_all(&harness.pool).await?;
    assert_eq!(
        actual,
        keys.into_iter().map(|key| (key, 1)).collect::<Vec<_>>()
    );
    let ground: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i3229' AND i.lifecycle=1")
        .fetch_one(&harness.pool).await?;
    assert_eq!(ground, 1);
    let receipts: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_item_loop_move_receipts WHERE command_id=1")
            .fetch_one(&harness.pool)
            .await?;
    assert_eq!(receipts, 1);
    Ok(())
}
