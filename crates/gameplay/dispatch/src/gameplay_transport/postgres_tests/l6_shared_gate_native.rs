//! Native counterpart threshold transports, real native USE and STEP, no synthetic IDs.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    fixture::{Harness, debug},
};
use crate::foundation::{ExactActorRef, GameSessionId};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use oteryn_protocol_oteryn::world_spatial::{
    StepDirection, StepDisposition, decode_step_result, encode_step_intent,
};
type Point = (u16, u16, u8);
type RegionSource = (&'static [u8], (u8, u16, u16));
struct GateCase {
    action: u16,
    required: u16,
    trigger: Point,
    goal: Point,
    levers: &'static [Lever],
}
#[derive(Clone, Copy)]
struct Lever {
    at: Point,
    start: Point,
    ordinal: u8,
}
const LEVERS_12121: &[Lever] = &[
    Lever {
        at: (33320, 32682, 14),
        start: (33321, 32682, 14),
        ordinal: 1,
    },
    Lever {
        at: (33338, 32702, 14),
        start: (33337, 32702, 14),
        ordinal: 1,
    },
    Lever {
        at: (33349, 32680, 14),
        start: (33350, 32680, 14),
        ordinal: 1,
    },
    Lever {
        at: (33358, 32701, 14),
        start: (33359, 32701, 14),
        ordinal: 1,
    },
    Lever {
        at: (33305, 32734, 14),
        start: (33306, 32734, 14),
        ordinal: 1,
    },
    Lever {
        at: (33357, 32749, 14),
        start: (33356, 32749, 14),
        ordinal: 1,
    },
    Lever {
        at: (33368, 32763, 14),
        start: (33369, 32763, 14),
        ordinal: 1,
    },
    Lever {
        at: (33382, 32786, 14),
        start: (33381, 32786, 14),
        ordinal: 1,
    },
];
const LEVERS_12122: &[Lever] = &[
    Lever {
        at: (33006, 32563, 13),
        start: (33007, 32563, 13),
        ordinal: 1,
    },
    Lever {
        at: (33055, 32487, 13),
        start: (33054, 32487, 13),
        ordinal: 1,
    },
    Lever {
        at: (33036, 32507, 13),
        start: (33037, 32507, 13),
        ordinal: 1,
    },
    Lever {
        at: (33077, 32507, 13),
        start: (33076, 32507, 13),
        ordinal: 1,
    },
    Lever {
        at: (33130, 32489, 13),
        start: (33129, 32489, 13),
        ordinal: 1,
    },
    Lever {
        at: (33027, 32530, 13),
        start: (33027, 32531, 13),
        ordinal: 1,
    },
    Lever {
        at: (33089, 32514, 13),
        start: (33088, 32514, 13),
        ordinal: 1,
    },
    Lever {
        at: (33104, 32514, 13),
        start: (33103, 32514, 13),
        ordinal: 1,
    },
    Lever {
        at: (33147, 32524, 13),
        start: (33146, 32524, 13),
        ordinal: 1,
    },
    Lever {
        at: (33028, 32588, 13),
        start: (33027, 32588, 13),
        ordinal: 1,
    },
    Lever {
        at: (33123, 32599, 13),
        start: (33122, 32599, 13),
        ordinal: 1,
    },
];
const LEVERS_12123: &[Lever] = &[
    Lever {
        at: (33176, 32880, 11),
        start: (33177, 32880, 11),
        ordinal: 2,
    },
    Lever {
        at: (33182, 32880, 11),
        start: (33181, 32880, 11),
        ordinal: 2,
    },
    Lever {
        at: (33175, 32884, 11),
        start: (33176, 32884, 11),
        ordinal: 2,
    },
    Lever {
        at: (33183, 32884, 11),
        start: (33182, 32884, 11),
        ordinal: 2,
    },
    Lever {
        at: (33176, 32889, 11),
        start: (33177, 32889, 11),
        ordinal: 2,
    },
    Lever {
        at: (33181, 32889, 11),
        start: (33180, 32889, 11),
        ordinal: 2,
    },
];
#[test]
fn l6_native_gate_closed_50135() -> TestResult {
    run("l6-tomb-gate-native-closed-50135")
}
#[test]
fn l6_native_gate_open_50135() -> TestResult {
    run("l6-tomb-gate-native-open-50135")
}
#[test]
fn l6_native_gate_closed_50137() -> TestResult {
    run("l6-tomb-gate-native-closed-50137")
}
#[test]
fn l6_native_gate_open_50137() -> TestResult {
    run("l6-tomb-gate-native-open-50137")
}
#[test]
fn l6_native_gate_closed_50138() -> TestResult {
    run("l6-tomb-gate-native-closed-50138")
}
#[test]
fn l6_native_gate_open_50138() -> TestResult {
    run("l6-tomb-gate-native-open-50138")
}
fn run(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let trigger = position(name)?;
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED {name}");
    let result = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            super::quest_world_callbacks::scenario(name, trigger, false).map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "native gate fixture panicked")?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}
fn position_of(p: Point) -> Position {
    Position {
        x: p.0,
        y: p.1,
        floor: -(p.2 as i8),
    }
}
pub(super) fn position(name: &str) -> TestResult<Position> {
    Ok(position_of(spec(name)?.trigger))
}
pub(super) fn start(name: &str) -> TestResult<Position> {
    let p = spec(name)?.trigger;
    Ok(position_of((p.0 - 1, p.1, p.2)))
}
fn spec(name: &str) -> TestResult<GateCase> {
    Ok(match name.rsplit('-').next().ok_or("gate name")? {
        "50135" => GateCase {
            action: 12121,
            required: 8,
            trigger: (33381, 32786, 14),
            goal: (33367, 32805, 14),
            levers: LEVERS_12121,
        },
        "50137" => GateCase {
            action: 12122,
            required: 7,
            trigger: (33088, 32514, 13),
            goal: (33083, 32568, 14),
            levers: LEVERS_12122,
        },
        "50138" => GateCase {
            action: 12123,
            required: 5,
            trigger: (33182, 32884, 11),
            goal: (33198, 32885, 11),
            levers: LEVERS_12123,
        },
        _ => return Err("unselected native threshold transport".into()),
    })
}
pub(super) fn region(pos: Point) -> TestResult<RegionSource> {
    Ok(match (pos.2, pos.0 / 256, pos.1 / 256) {
        (11, 129, 128) => (
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z11-x129-y128.b3"
            )),
            (11, 129, 128),
        ),
        (13, 128, 127) => (
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z13-x128-y127.b3"
            )),
            (13, 128, 127),
        ),
        (13, 129, 126) => (
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z13-x129-y126.b3"
            )),
            (13, 129, 126),
        ),
        (13, 129, 127) => (
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z13-x129-y127.b3"
            )),
            (13, 129, 127),
        ),
        (14, 129, 127) => (
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z14-x129-y127.b3"
            )),
            (14, 129, 127),
        ),
        (14, 130, 127) => (
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z14-x130-y127.b3"
            )),
            (14, 130, 127),
        ),
        (14, 130, 128) => (
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z14-x130-y128.b3"
            )),
            (14, 130, 128),
        ),
        _ => return Err("unselected native transport region".into()),
    })
}
pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
    name: &str,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene};
    let case = spec(name)?;
    let origin = (case.trigger.0 - 1, case.trigger.1, case.trigger.2);
    let mut points = std::collections::BTreeSet::from([origin, case.trigger, case.goal]);
    for lever in case.levers {
        points.insert(lever.at);
        points.insert(lever.start);
    }
    let mut crops = Vec::new();
    for at in points {
        let (bytes, r) = region(at)?;
        let ids = match at {
            (33006, 32563, 13) => vec![415, 2772],
            (33007, 32563, 13) => vec![415],
            (33027, 32530, 13) => vec![415, 2772],
            (33027, 32531, 13) => vec![415],
            (33027, 32588, 13) => vec![415],
            (33028, 32588, 13) => vec![415, 2772],
            (33036, 32507, 13) => vec![415, 2772],
            (33037, 32507, 13) => vec![415],
            (33054, 32487, 13) => vec![415],
            (33055, 32487, 13) => vec![415, 2772],
            (33076, 32507, 13) => vec![415],
            (33077, 32507, 13) => vec![415, 2772],
            (33083, 32568, 14) => vec![415],
            (33087, 32514, 13) => vec![415],
            (33088, 32514, 13) => vec![415],
            (33089, 32514, 13) => vec![415, 2772],
            (33103, 32514, 13) => vec![415],
            (33104, 32514, 13) => vec![415, 2772],
            (33122, 32599, 13) => vec![423],
            (33123, 32599, 13) => vec![423, 2772],
            (33129, 32489, 13) => vec![423],
            (33130, 32489, 13) => vec![423, 2772],
            (33146, 32524, 13) => vec![423],
            (33147, 32524, 13) => vec![423, 2772],
            (33174, 32884, 11) => vec![838],
            (33176, 32884, 11) => vec![417],
            (33175, 32884, 11) => vec![415, 2342, 2772],
            (33176, 32880, 11) => vec![415, 2343, 2772],
            (33176, 32889, 11) => vec![415, 2342, 2772],
            (33177, 32880, 11) => vec![415],
            (33177, 32889, 11) => vec![415],
            (33180, 32889, 11) => vec![415],
            (33181, 32880, 11) => vec![415],
            (33181, 32884, 11) => vec![423],
            (33181, 32889, 11) => vec![415, 2342, 2772],
            (33182, 32880, 11) => vec![415, 2342, 2772],
            (33182, 32884, 11) => vec![417],
            (33183, 32884, 11) => vec![415, 2342, 2772],
            (33198, 32885, 11) => vec![417],
            (33305, 32734, 14) => vec![415, 2772],
            (33306, 32734, 14) => vec![415],
            (33320, 32682, 14) => vec![415, 2772],
            (33321, 32682, 14) => vec![415],
            (33337, 32702, 14) => vec![417],
            (33338, 32702, 14) => vec![417, 2772],
            (33349, 32680, 14) => vec![415, 2772],
            (33350, 32680, 14) => vec![415],
            (33356, 32749, 14) => vec![415],
            (33357, 32749, 14) => vec![415, 2772],
            (33358, 32701, 14) => vec![415, 2772],
            (33359, 32701, 14) => vec![415],
            (33367, 32805, 14) => vec![415],
            (33368, 32763, 14) => vec![415, 2772],
            (33369, 32763, 14) => vec![415],
            (33380, 32786, 14) => vec![423],
            (33381, 32786, 14) => vec![415],
            (33382, 32786, 14) => vec![415, 2772],
            _ => return Err("unwitnessed native transport tile".into()),
        };
        crops.push(NativeCrop {
            bytes,
            region: r,
            bounds: (at.0, at.1, at.0 + 1, at.1 + 1),
            start: (origin.0, origin.1),
            expected: vec![ExpectedTile {
                position: (at.0, at.1),
                ids,
                counted_entry: None,
            }],
        });
    }
    native_scene(native, crops, &[])
}
// Setup between isolated native interaction pockets; every action under test is literal USE/STEP.
async fn relocate(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    at: Point,
) -> TestResult {
    let map = owner.bundle_map().ok_or("native transport map")?;
    let doors = crate::map::floor::BundleDoors(map.facts.doors());
    let mut runtime = owner.runtime.lock().await;
    let expected = runtime.read_actor_position(actor).map_err(debug)?;
    let at = super::quest_world_callbacks::local(position_of(at));
    if expected.position() == at {
        return Ok(());
    }
    let proof = crate::map::floor::prepare_quest_callback_teleport(
        &runtime,
        &owner.movement_cells,
        actor,
        session,
        expected,
        at,
        false,
        &doors,
    )
    .map_err(debug)?;
    runtime.commit_source_step(proof).map_err(debug)?;
    Ok(())
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    let v: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("POSTGRES_DATABASE_READY l6_native_gate version={v}");
    let actor = admitted.runtime_actor.ok_or("native actor")?;
    let case = spec(name)?;
    let open = name.contains("-open-");
    let count = if open {
        case.required
    } else {
        case.required - 1
    };
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    let mut ordinal = 1;
    for lever in case.levers.iter().take(usize::from(count)) {
        relocate(owner, actor, admitted.game_session_id, lever.start).await?;
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(1))
            .ok_or("native USE clock")?;
        let map = owner.bundle_map().ok_or("native map")?;
        let key = oteryn_world_bundle::bundle::placement_key(
            -(lever.at.2 as i8),
            lever.at.0,
            lever.at.1,
            lever.ordinal,
        )
        .ok_or("native lever key")?;
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget {
            placement,
            expected_revision: 0,
        })
        .map_err(debug)?;
        let reply =
            super::quest_world_callbacks::wire(owner, admitted, ordinal, 2, payload).await?;
        ordinal += 1;
        assert_eq!(
            oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
            UseDisposition::Committed,
            "native threshold contributions require actual USE"
        );
    }
    let leases:i64=sqlx::query_scalar("SELECT count(*) FROM game_shared_switch_leases l JOIN game_shared_switch_recipes p USING(recipe) WHERE p.action_id=$1").bind(i32::from(case.action)).fetch_one(&h.pool).await?;
    assert_eq!(leases, i64::from(count));
    let origin = (case.trigger.0 - 1, case.trigger.1, case.trigger.2);
    relocate(owner, actor, admitted.game_session_id, origin).await?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(1))
        .ok_or("native STEP clock")?;
    let payload = encode_step_intent(StepDirection::East);
    let reply =
        super::quest_world_callbacks::wire(owner, admitted, ordinal, 1, payload.clone()).await?;
    assert_eq!(
        decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    let expected = position_of(if open { case.goal } else { origin });
    let at = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    assert_eq!(
        at,
        super::quest_world_callbacks::local(expected),
        "exact native threshold transport or origin restoration"
    );
    assert_eq!(
        super::quest_world_callbacks::wire(owner, admitted, ordinal, 1, payload).await?,
        reply
    );
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        at,
        "replay cannot transport twice"
    );
    let effects = owner
        .bundle_map()
        .ok_or("native effect map")?
        .facts
        .movement_effects_since(0);
    assert_eq!(effects.len(), 1);
    assert_eq!(
        (effects[0].1, effects[0].2),
        (
            crate::map::overlay::TilePos {
                x: expected.x,
                y: expected.y,
                floor: expected.floor
            },
            11
        )
    );
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        before, after,
        "threshold transport is not personal quest progress"
    );
    Ok(())
}
