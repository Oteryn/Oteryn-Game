//! Four retained fragment approaches share one bounded stage counter.
use super::super::quest_world_callbacks::{local, wire};
use super::*;
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};

const QUEST: &str = "oteryn:quest.koshei_the_deathless_quest";
const TRACK: &str = "oteryn:quest-progress/chosen-source/koshei_the_deathless_quest/s1";
const POINTS: [(u16, u16, i8, StepDirection); 4] = [
    (33195, 32458, -7, StepDirection::West),
    (33304, 32277, -10, StepDirection::East),
    (33211, 32593, -7, StepDirection::East),
    (33054, 32468, -11, StepDirection::West),
];

#[test]
fn m01b_koshei_four_native_visits_require_level_and_cannot_be_farmed() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result =
        super::super::kill_reward_e2e::scenario_thread::run("m01b-koshei-fragments", || {
            super::super::quest_world_callbacks::scenario(
                "m01b/koshei-the-deathless/s1/koshei_amulet_part1",
                Position {
                    x: 33195,
                    y: 32458,
                    floor: -7,
                },
                false,
            )
        });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_koshei_fragments duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene};
    let bytes: [&'static [u8]; 4] = [
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z07-x129-y126.b3"
        )),
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z10-x130-y126.b3"
        )),
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z07-x129-y127.b3"
        )),
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z11-x129-y126.b3"
        )),
    ];
    let appearances = [408, 410, 231, 409];
    let crops = POINTS
        .iter()
        .enumerate()
        .map(|(i, &(x, y, floor, direction))| {
            let start_x = if direction == StepDirection::West {
                x + 1
            } else {
                x - 1
            };
            NativeCrop {
                bytes: bytes[i],
                region: ((-floor) as u8, x / 256, y / 256),
                bounds: (x - 1, y - 1, x + 2, y + 1),
                start: (start_x, y),
                expected: vec![ExpectedTile {
                    position: (x, y),
                    ids: vec![appearances[i]],
                    counted_entry: None,
                }],
            }
        })
        .collect();
    native_scene(native, crops, &[])
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("Koshei actor")?;
    // Arrange durable levels, not the cached client/runtime level: the World
    // predicate must read the actual PostgreSQL progression owner.
    seed_level(h, 39).await?;
    let mut command = 1;
    step(owner, admitted, command, StepDirection::West).await?;
    assert_eq!(counts(h).await?, (0, 0, 0));
    seed_level(h, 40).await?;
    // Raising the level cannot retroactively turn the old refused visit into credit.
    step(owner, admitted, command, StepDirection::West).await?;
    assert_eq!(counts(h).await?, (0, 0, 0));
    for (index, &(x, y, floor, direction)) in POINTS.iter().enumerate() {
        let start = Position {
            x: if direction == StepDirection::West {
                x + 1
            } else {
                x - 1
            },
            y,
            floor,
        };
        relocate(owner, actor, admitted.game_session_id, start).await?;
        let prior = i64::try_from(index)?;
        assert_eq!(
            counts(h).await?,
            (prior, prior, 0),
            "fixture positioning grants no exploration credit"
        );
        command += 1;
        step(owner, admitted, command, direction).await?;
        let expected = i64::try_from(index + 1)?;
        assert_eq!(counts(h).await?, (expected, expected, 0));
        step(owner, admitted, command, direction).await?;
        assert_eq!(
            counts(h).await?,
            (expected, expected, 0),
            "replay cannot farm a fragment"
        );
        let reverse = if direction == StepDirection::West {
            StepDirection::East
        } else {
            StepDirection::West
        };
        command += 1;
        step(owner, admitted, command, reverse).await?;
        command += 1;
        step(owner, admitted, command, direction).await?;
        assert_eq!(
            counts(h).await?,
            (expected, expected, 0),
            "revisiting one fragment cannot replace another"
        );
    }
    assert_eq!(
        counts(h).await?,
        (4, 4, 0),
        "exploration does not finish the whole quest"
    );
    Ok(())
}

// Administrator arrangement only, matching grizzly_durability::seed. SET LOCAL
// ends at commit; all subsequent native commands use the normal fenced writer.
pub(super) async fn seed_level(h: &Harness, level: i64) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_character_progression_state SET level=$2 WHERE character_id=encode($1,'hex')::uuid")
        .bind(super::super::fixture::id(41).as_slice()).bind(level).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn counts(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key LIKE 'oteryn:quest-callback/m01b/koshei-the-deathless/s1/%'), (SELECT coalesce(max(value),0) FROM game_character_quest_tracks WHERE track_key=$1), (SELECT count(*) FROM game_character_quest_states WHERE quest_key=$2 AND completed_character_revision IS NOT NULL)")
        .bind(TRACK).bind(QUEST).fetch_one(&h.pool).await?)
}

pub(super) async fn step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: u64,
    direction: StepDirection,
) -> TestResult {
    if command == admitted.continuity.next_command_id {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
    }
    let reply = wire(
        owner,
        admitted,
        command,
        1,
        oteryn_protocol_oteryn::world_spatial::encode_step_intent(direction),
    )
    .await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_spatial::decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    Ok(())
}

// Same fixture arrangement used by qengine_access::relocate. Disconnected source
// floors are positioned by the server; each credit still requires a real STEP.
pub(super) async fn relocate(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
    at: Position,
) -> TestResult {
    let map = owner.bundle_map().ok_or("map")?;
    let closed = crate::map::floor::BundleDoors(map.facts.doors());
    let mut runtime = owner.runtime.lock().await;
    let expected = runtime.read_actor_position(actor).map_err(debug)?;
    let proof = crate::map::floor::prepare_quest_callback_teleport(
        &runtime,
        &owner.movement_cells,
        actor,
        session,
        expected,
        local(at),
        false,
        &closed,
    )
    .map_err(debug)?;
    runtime.commit_source_step(proof).map_err(debug)?;
    Ok(())
}
