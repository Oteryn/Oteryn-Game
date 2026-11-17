//! The exact Outlaw Camp final chest through USE, durable contents, and replay.
use super::fixture::Harness;
use super::questpc8_authored_programmes::{object_target, use_target};
use super::{TestResult, quest_world_callbacks};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::map::overlay::TilePos;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::UseDisposition;

pub(super) const CASE: &str = "authored-outlaw-camp-reward";
const CHEST: TilePos = TilePos {
    x: 32620,
    y: 32198,
    floor: -10,
};

#[test]
fn outlaw_camp_reward_chest_commits_sword_and_gem_once_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run(CASE, move || {
        quest_world_callbacks::scenario_with_admin(
            CASE,
            Position {
                x: CHEST.x,
                y: CHEST.y,
                floor: CHEST.floor,
            },
            admin,
        )
    })
}

pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    super::quest_callbacks::source_world_rectangle_with_prefix(
        (CHEST.x, CHEST.y, CHEST.floor),
        &[(CHEST.x, 0)],
        2472,
        2,
        2,
        &[2317],
    )
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
) -> TestResult {
    let target = object_target(owner, CHEST, 2)?;
    assert_eq!(
        use_target(owner, admitted, 1, &target).await?,
        UseDisposition::Committed
    );
    assert_eq!(
        use_target(owner, admitted, 1, &target).await?,
        UseDisposition::Committed
    );
    let target = object_target(owner, CHEST, 2)?;
    assert_eq!(
        use_target(owner, admitted, 2, &target).await?,
        UseDisposition::Rejected
    );

    let contents: Vec<(String, i64)> = sqlx::query_as(
        "SELECT child.definition_production_key, child.quantity
         FROM game_item_instances AS root
         JOIN game_item_nested_entries AS edge ON edge.parent_item_instance_id=root.item_instance_id
         JOIN game_item_instances AS child ON child.item_instance_id=edge.item_instance_id
         WHERE root.definition_production_key='oteryn:item.tibia.i2853' ORDER BY edge.placement_ordinal",
    ).fetch_all(&harness.pool).await?;
    assert_eq!(
        contents,
        [
            ("oteryn:item.tibia.i3295".into(), 1),
            ("oteryn:item.tibia.i3039".into(), 1)
        ]
    );
    let completed: bool = sqlx::query_scalar(
        "SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE quest_key=$1",
    ).bind("oteryn:quest.the_outlaw_camp_quest").fetch_one(&harness.pool).await?;
    assert!(completed);
    Ok(())
}
