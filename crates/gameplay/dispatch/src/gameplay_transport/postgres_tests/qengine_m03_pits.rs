//! Live level/Premium admission through the production Pits STEP binding.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};

pub(super) const NAME: &str = "qengine-rewards-m03-pits";
pub(super) const ENTRY: Position = Position {
    x: 32791,
    y: 32331,
    floor: -10,
};
const QUEST: &str = "oteryn:quest.the_pits_of_inferno_quest";

#[test]
fn m03_pits_native_step_enforces_live_premium_level_and_replay_postgres() -> TestResult {
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, ENTRY, false)
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED {NAME} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
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
            bounds: (32789, 32330, 32794, 32334),
            start: (32791, 32332),
            expected: vec![
                ExpectedTile {
                    position: (32791, 32331),
                    ids: vec![354],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (32791, 32332),
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
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT version()")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("M03_PITS_STEP_POSTGRES {version}");
    owner.admit_premium(super::fixture::id(40), admitted.game_session_id);
    assert!(
        owner
            .session_premium_gate(admitted.game_session_id)
            .await
            .premium_current()
    );
    assert_eq!(footprint(h).await?, (0, 0));
    let origin = position(owner, admitted).await?;
    step(owner, admitted, 1, StepDirection::North).await?;
    assert_eq!(
        position(owner, admitted).await?,
        origin,
        "level gate returns the actor"
    );
    assert_eq!(footprint(h).await?, (0, 0));
    super::qengine_outfit_stories::earn_level(owner, admitted, authority, 80).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let premium = owner.premium_coordinator.take();
    let denied = step(owner, admitted, 2, StepDirection::North).await;
    owner.premium_coordinator = premium;
    denied?;
    assert_eq!(
        position(owner, admitted).await?,
        origin,
        "absent Premium owner fails closed"
    );
    assert_eq!(footprint(h).await?, (0, 0));
    assert!(
        owner
            .session_premium_gate(admitted.game_session_id)
            .await
            .premium_current(),
        "restored delivered Premium is current"
    );
    let level: i64 = sqlx::query_scalar("SELECT level FROM game_character_progression_state")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        level, 80,
        "the existing XP writer earned the required level"
    );
    let accepted = step(owner, admitted, 3, StepDirection::North).await?;
    assert_eq!(position(owner, admitted).await?, (32791, 32331, 10));
    assert_eq!(footprint(h).await?, (1, 1));
    let replay = step(owner, admitted, 3, StepDirection::North).await?;
    assert_eq!(
        replay, accepted,
        "literal STEP replay returns its original result"
    );
    assert_eq!(position(owner, admitted).await?, (32791, 32331, 10));
    assert_eq!(footprint(h).await?, (1, 1));
    let premium = owner.premium_coordinator.take();
    let exited = step(owner, admitted, 4, StepDirection::South).await;
    exited?;
    assert_eq!(
        position(owner, admitted).await?,
        origin,
        "egress does not require Premium"
    );
    let denied = step(owner, admitted, 5, StepDirection::North).await;
    owner.premium_coordinator = premium;
    denied?;
    assert_eq!(
        position(owner, admitted).await?,
        origin,
        "earned progress never bypasses the live gate"
    );
    assert_eq!(footprint(h).await?, (1, 1));
    let value:i64=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE quest_key=$1 AND track_key='oteryn:quest-progress/chosen-source/the_pits_of_inferno_quest/s1'").bind(QUEST).fetch_one(&h.pool).await?;
    assert_eq!(value, 1);
    let completed: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1 AND completes",
    )
    .bind(QUEST)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(completed, 0, "entrance is not quest completion");
    Ok(())
}
async fn position(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<(i32, i32, i16)> {
    let runtime = owner.runtime.lock().await;
    let p = runtime
        .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position();
    Ok((p.x, p.y, p.floor))
}
async fn footprint(h: &Harness) -> Result<(i64, i64), sqlx::Error> {
    sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_tracks WHERE quest_key=$1),(SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1)").bind(QUEST).fetch_one(&h.pool).await
}
async fn step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    id: u64,
    direction: StepDirection,
) -> TestResult<Vec<u8>> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        id,
        1,
        oteryn_protocol_oteryn::world_spatial::encode_step_intent(direction),
    )
    .await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_spatial::decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    Ok(reply)
}
