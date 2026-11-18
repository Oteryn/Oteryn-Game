//! Crystal 201189f8 data-global/lib/quests/svargrond_arena.lua:217-232.
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};

const INSIDE: Position = Position {
    x: 32208,
    y: 31098,
    floor: -7,
};
const OUTSIDE: Position = Position {
    x: 32215,
    y: 31098,
    floor: -7,
};

#[test]
fn native_arena_entry_retires_only_the_original_top_ground_item() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(
        "qcx61-pit-ground-reset",
        super::arena_match::native_pit_ground_reset,
    )
}

pub(super) async fn seed(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult<[Vec<u8>; 3]> {
    let fence = admitted.item_fence.ok_or("item fence")?;
    let mut ids = Vec::new();
    for (index, (appearance, position)) in [(3031, INSIDE), (3035, INSIDE), (3035, OUTSIDE)]
        .into_iter()
        .enumerate()
    {
        super::quest_world_callbacks::input_item(
            owner,
            fence,
            &Offering {
                position,
                item: format!("oteryn:item.tibia.i{appearance}"),
                count: 1,
            },
            516 + u64::try_from(index)?,
            false,
        )
        .await?;
        let cell = crate::durability::map_item_mint_audit::map_item_spatial_position(
            position.x,
            position.y,
            position.floor,
        );
        let id: Vec<u8> = sqlx::query_scalar(
            "SELECT uuid_send(item_instance_id) FROM game_item_ground_locations
             WHERE spatial_position=$1 ORDER BY stack_ordinal DESC LIMIT 1",
        )
        .bind(cell)
        .fetch_one(&h.pool)
        .await?;
        ids.push(id);
    }
    owner.states.loop_ground.invalidate();
    ids.try_into().map_err(|_| "three observed items".into())
}

pub(super) async fn assert_retirement(
    h: &Harness,
    [under, top, outside]: [Vec<u8>; 3],
) -> TestResult {
    let result: (i16, bool, i64) = sqlx::query_as(
        "SELECT i.lifecycle,g.item_instance_id IS NOT NULL,
             (SELECT count(*) FROM game_item_decay_retire_receipts r
              JOIN game_item_audit_outbox a USING(item_instance_id,transaction_id,event_id)
              WHERE r.item_instance_id=i.item_instance_id AND r.cause_kind='ARENA_RESET')
         FROM game_item_instances i LEFT JOIN game_item_ground_locations g USING(item_instance_id)
         WHERE i.item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(top)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(result, (2, false, 1));
    for unchanged in [under, outside] {
        let still_ground: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM game_item_ground_locations g
             JOIN game_item_instances i USING(item_instance_id)
             WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.lifecycle=1)",
        )
        .bind(unchanged)
        .fetch_one(&h.pool)
        .await?;
        assert!(still_ground);
    }
    Ok(())
}
