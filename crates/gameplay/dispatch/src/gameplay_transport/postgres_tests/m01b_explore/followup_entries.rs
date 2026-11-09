//! Later native surveys retain real first visits and separate action/kill evidence.
use super::koshei::{relocate, step};
use super::*;
use oteryn_protocol_oteryn::world_spatial::StepDirection;

#[derive(Clone, Copy)]
struct Route {
    name: &'static str,
    tail: &'static str,
    start: Position,
    first: Position,
    first_direction: StepDirection,
    entry: StepDirection,
    exit: StepDirection,
    required: i64,
}

const ROUTES: [Route; 3] = [
    Route {
        name: "m01b/dragon-corpse/s3/dragoncorpse",
        tail: "dragon_corpse_quest",
        start: Position {
            x: 32178,
            y: 32223,
            floor: -9,
        },
        first: Position {
            x: 32148,
            y: 32207,
            floor: -7,
        },
        first_direction: StepDirection::West,
        entry: StepDirection::South,
        exit: StepDirection::North,
        required: 1,
    },
    Route {
        name: "m01b/dragon-tower/s3/backpack1",
        tail: "dragon_tower_quest",
        start: Position {
            x: 33073,
            y: 32170,
            floor: -2,
        },
        first: Position {
            x: 33074,
            y: 32169,
            floor: -7,
        },
        first_direction: StepDirection::East,
        entry: StepDirection::North,
        exit: StepDirection::South,
        required: 10,
    },
    Route {
        name: "m01b/edron-goblin/s3/edrongoblin1",
        tail: "edron_goblin_quest",
        start: Position {
            x: 33096,
            y: 31801,
            floor: -10,
        },
        first: Position {
            x: 33109,
            y: 31850,
            floor: -7,
        },
        first_direction: StepDirection::West,
        entry: StepDirection::North,
        exit: StepDirection::South,
        required: 10,
    },
];

fn route(name: &str) -> TestResult<Route> {
    ROUTES
        .into_iter()
        .find(|r| r.name == name)
        .ok_or_else(|| "unbound M01b follow-up".into())
}

#[test]
fn m01b_followup_dragon_corpse_crosses_native_fire_after_first_visit_and_use() -> TestResult {
    run(ROUTES[0])
}

#[test]
fn m01b_followup_dragon_tower_requires_first_visit_and_ten_kills() -> TestResult {
    run(ROUTES[1])
}

#[test]
fn m01b_followup_edron_throne_requires_first_visit_ten_kills_and_premium() -> TestResult {
    run(ROUTES[2])
}

fn run(r: Route) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::super::kill_reward_e2e::scenario_thread::run(r.tail, move || {
        super::super::quest_world_callbacks::scenario(r.name, target(r), false)
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_followup_{} duration={:?} success={}",
        r.tail,
        started.elapsed(),
        result.is_ok()
    );
    result
}

fn target(r: Route) -> Position {
    match r.entry {
        StepDirection::South => Position {
            y: r.start.y + 1,
            ..r.start
        },
        _ => Position {
            y: r.start.y - 1,
            ..r.start
        },
    }
}

pub(super) fn start(name: &str) -> Option<Position> {
    ROUTES.into_iter().find(|r| r.name == name).map(|r| r.start)
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
    name: &str,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene};
    let r = route(name)?;
    let tile = |position, ids| ExpectedTile {
        position,
        ids,
        counted_entry: None,
    };
    let crops = match r.tail {
        "dragon_corpse_quest" => vec![
            NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z09-x125-y125.b3"
                )),
                region: (9, 125, 125),
                bounds: (32176, 32222, 32180, 32225),
                start: (r.start.x, r.start.y),
                expected: vec![
                    tile((32179, 32224), vec![355, 4025]),
                    tile((32177, 32224), vec![355, 2125]),
                    tile((32177, 32223), vec![355]),
                    tile((32178, 32224), vec![355]),
                    tile((32178, 32223), vec![355]),
                ],
            },
            NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z07-x125-y125.b3"
                )),
                region: (7, 125, 125),
                bounds: (32147, 32207, 32149, 32208),
                start: (r.first.x, r.first.y),
                expected: vec![
                    tile((32147, 32207), vec![4409]),
                    tile((32148, 32207), vec![4406]),
                ],
            },
        ],
        "dragon_tower_quest" => vec![
            NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z02-x129-y125.b3"
                )),
                region: (2, 129, 125),
                bounds: (33071, 32168, 33074, 32171),
                start: (r.start.x, r.start.y),
                expected: vec![
                    tile((33072, 32169), vec![408, 2469]),
                    tile((33073, 32169), vec![408]),
                    tile((33073, 32170), vec![408]),
                ],
            },
            NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z07-x129-y125.b3"
                )),
                region: (7, 129, 125),
                bounds: (33074, 32169, 33076, 32170),
                start: (r.first.x, r.first.y),
                expected: vec![tile((33075, 32169), vec![408])],
            },
        ],
        "edron_goblin_quest" => vec![
            NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z10-x129-y124.b3"
                )),
                region: (10, 129, 124),
                bounds: (33094, 31799, 33097, 31802),
                start: (r.start.x, r.start.y),
                expected: vec![
                    tile((33095, 31800), vec![353, 2469]),
                    tile((33095, 31801), vec![353, 2469]),
                    tile((33096, 31800), vec![353]),
                    tile((33096, 31801), vec![353]),
                ],
            },
            NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z07-x129-y124.b3"
                )),
                region: (7, 129, 124),
                bounds: (33108, 31849, 33110, 31851),
                start: (r.first.x, r.first.y),
                expected: vec![tile((33108, 31850), vec![355])],
            },
        ],
        _ => return Err("unbound follow-up scene".into()),
    };
    native_scene(native, crops, &[])
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    callback: &Callback,
) -> TestResult {
    let r = route(
        callback
            .key
            .strip_prefix("oteryn:quest-callback/")
            .ok_or("follow-up key")?,
    )?;
    let actor = admitted.runtime_actor.ok_or("follow-up actor")?;
    step(owner, admitted, 1, r.entry).await?;
    assert_eq!(
        counts(h, r).await?,
        (0, 0, 0),
        "later visit needs a real first survey"
    );
    relocate(owner, actor, admitted.game_session_id, r.first).await?;
    step(owner, admitted, 2, r.first_direction).await?;
    let first:i64=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(super::super::fixture::id(41).as_slice()).bind(format!("{}s1",prefix(r))).fetch_one(&h.pool).await?;
    assert_eq!(first, 1, "first exploration is earned by real native STEP");
    relocate(owner, actor, admitted.game_session_id, r.start).await?;
    step(owner, admitted, 3, r.entry).await?;
    assert_eq!(
        counts(h, r).await?,
        (0, 0, 0),
        "first visit cannot replace action or kill evidence"
    );
    seed_evidence(h, r).await?;
    step(owner, admitted, 3, r.entry).await?;
    assert_eq!(
        counts(h, r).await?,
        (0, 0, 0),
        "new evidence cannot retro-credit a replay"
    );
    let mut command = 4;
    if callback.admission.as_ref().is_some_and(|a| a.premium) {
        relocate(owner, actor, admitted.game_session_id, r.start).await?;
        let premium = owner
            .premium_coordinator
            .take()
            .ok_or("Current Premium provider")?;
        let visit = step(owner, admitted, command, r.entry).await;
        owner.premium_coordinator = Some(premium);
        visit?;
        assert_eq!(counts(h, r).await?, (0, 0, 0));
        step(owner, admitted, command, r.entry).await?;
        assert_eq!(counts(h, r).await?, (0, 0, 0));
        command += 1;
    }
    let successful_direction = if r.tail == "dragon_corpse_quest" {
        relocate(
            owner,
            actor,
            admitted.game_session_id,
            Position {
                x: 32177,
                y: 32223,
                floor: -9,
            },
        )
        .await?;
        step(owner, admitted, command, StepDirection::South).await?;
        assert_eq!(
            counts(h, r).await?,
            (0, 0, 0),
            "the retained fire field is not the final corpse approach"
        );
        command += 1;
        StepDirection::East
    } else {
        relocate(owner, actor, admitted.game_session_id, r.start).await?;
        r.entry
    };
    step(owner, admitted, command, successful_direction).await?;
    assert_eq!(counts(h, r).await?, (1, 1, 0));
    step(owner, admitted, command, successful_direction).await?;
    assert_eq!(counts(h, r).await?, (1, 1, 0));
    step(owner, admitted, command + 1, r.exit).await?;
    step(owner, admitted, command + 2, r.entry).await?;
    assert_eq!(
        counts(h, r).await?,
        (1, 1, 0),
        "revisits cannot farm progress or complete reward sets"
    );
    Ok(())
}

fn prefix(r: Route) -> String {
    format!("oteryn:quest-progress/chosen-source/{}/", r.tail)
}

// Arrange only the separate action/kill owner's completed condition; never an
// EXPLORE counter, visit receipt or live root revision.
async fn seed_evidence(h: &Harness, r: Route) -> TestResult {
    let before = counts(h, r).await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,$3,$4,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid ON CONFLICT(character_id,track_key) DO UPDATE SET value=EXCLUDED.value,committed_character_revision=EXCLUDED.committed_character_revision")
        .bind(super::super::fixture::id(41).as_slice()).bind(format!("{}s2",prefix(r))).bind(format!("oteryn:quest.{}",r.tail)).bind(r.required).execute(&mut *tx).await?;
    tx.commit().await?;
    assert_eq!(counts(h, r).await?, before);
    Ok(())
}

async fn counts(h: &Harness, r: Route) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid AND transition_key=$2),(SELECT coalesce(max(value),0) FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$3),(SELECT count(*) FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$4 AND completed_character_revision IS NOT NULL)")
        .bind(super::super::fixture::id(41).as_slice()).bind(format!("oteryn:quest-callback/{}",r.name)).bind(format!("{}s3",prefix(r))).bind(format!("oteryn:quest.{}",r.tail)).fetch_one(&h.pool).await?)
}
