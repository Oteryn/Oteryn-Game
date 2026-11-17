//! Native mirror USE through the admitted PostgreSQL-backed command path.
use super::{TestResult, quest_world_callbacks};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};

pub(super) const NAME: &str = "blood-brothers-lersatio-mirror-1-first";
const MIRROR: Position = Position {
    x: 32965,
    y: 31457,
    floor: -2,
};

#[test]
fn lersatio_first_mirror_use_advances_durable_counter() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run("lersatio-first-use", || {
        quest_world_callbacks::scenario(NAME, MIRROR, false)
    })
}

pub(super) fn world(callback: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    quest_world_callbacks::world_with_objects(
        callback,
        MIRROR,
        &[
            (
                Position {
                    x: 32967,
                    y: 31457,
                    floor: -2,
                },
                8325,
            ),
            (
                Position {
                    x: 32966,
                    y: 31456,
                    floor: -2,
                },
                8667,
            ),
            (
                Position {
                    x: 32968,
                    y: 31456,
                    floor: -2,
                },
                8667,
            ),
        ],
        &[8326, 8721],
    )
}

pub(super) async fn exercise(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
) -> TestResult {
    use super::questpc8_authored_programmes::{object_target, use_target};
    use crate::map::overlay::TilePos;
    use oteryn_protocol_oteryn::world_object::UseDisposition;
    let target = object_target(
        owner,
        TilePos {
            x: MIRROR.x,
            y: MIRROR.y,
            floor: MIRROR.floor,
        },
        1,
    )?;
    assert_eq!(
        use_target(owner, admitted, 1, &target).await?,
        UseDisposition::Committed
    );
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest fence")?;
    let state = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await?;
    assert_eq!(
        state
            .tracks()
            .get("oteryn:quest-progress/crystalserver/quest/u8_4/blood_brothers/mirror_counter"),
        Some(&1)
    );
    Ok(())
}
