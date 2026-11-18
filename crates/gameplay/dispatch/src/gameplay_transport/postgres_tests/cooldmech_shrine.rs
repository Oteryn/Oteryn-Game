//! The Shrine of Wonders guard through native movement dispatch.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};

pub(super) const NAME: &str = "cooldmech-shrine";
pub(super) const ORIGIN: Position = Position {
    x: 32791,
    y: 32331,
    floor: -10,
};
const ENTRY: Position = Position { y: 32330, ..ORIGIN };
const LANDING: Position = Position { y: 32327, ..ORIGIN };

#[test]
fn cooldmech_shrine_step_requires_live_premium_and_carried_book_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, ENTRY, false)
    })
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_crop};
    native_crop(
        native,
        NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z10-x128-y126.b3"
            )),
            region: (10, 128, 126),
            bounds: (32789, 32326, 32794, 32333),
            start: (ORIGIN.x, ORIGIN.y),
            expected: vec![
                ExpectedTile {
                    position: (ENTRY.x, ENTRY.y),
                    ids: vec![599],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (LANDING.x, LANDING.y),
                    ids: vec![354],
                    counted_entry: None,
                },
            ],
        },
    )
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
) -> TestResult {
    owner.admit_premium(id(40), admitted.game_session_id);
    assert!(
        owner
            .session_premium_gate(admitted.game_session_id)
            .await
            .premium_current()
    );
    step(owner, admitted, 1, StepDirection::North, ORIGIN).await?;
    let mut ordinal = 101;
    super::qengine_voucher_stories::mint(owner, admitted, 2836, 1, &mut ordinal).await?;
    let premium = owner.premium_coordinator.take();
    let result = step(owner, admitted, 2, StepDirection::North, ORIGIN).await;
    owner.premium_coordinator = premium;
    result?;
    step(owner, admitted, 3, StepDirection::North, LANDING).await?;
    step(owner, admitted, 4, StepDirection::South, ORIGIN).await?;
    assert_eq!(
        super::qengine_voucher_stories::quantity(harness, 2836).await?,
        1
    );
    Ok(())
}

async fn step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: u64,
    direction: StepDirection,
    expected: Position,
) -> TestResult {
    owner.clock_origin -= std::time::Duration::from_secs(2);
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
    let at = owner
        .runtime
        .lock()
        .await
        .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
        .map_err(debug)?;
    assert_eq!(at.position(), super::quest_world_callbacks::local(expected));
    Ok(())
}
