//! Literal native USE progresses only the exact ordered source sign. No SQL flags.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    fixture::{Harness, debug},
};
use crate::content::native_gameplay::NativeGameplayState;
use crate::foundation::{ExactActorRef, GameSessionId};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
};
#[derive(Clone, Copy)]
struct Sign {
    at: Position,
    start: Position,
}
const SIGNS: &[Sign] = &[
    Sign {
        at: Position {
            x: 33031,
            y: 32532,
            floor: -13,
        },
        start: Position {
            x: 33030,
            y: 32532,
            floor: -13,
        },
    },
    Sign {
        at: Position {
            x: 33033,
            y: 32536,
            floor: -13,
        },
        start: Position {
            x: 33032,
            y: 32536,
            floor: -13,
        },
    },
    Sign {
        at: Position {
            x: 33044,
            y: 32549,
            floor: -13,
        },
        start: Position {
            x: 33045,
            y: 32549,
            floor: -13,
        },
    },
    Sign {
        at: Position {
            x: 33044,
            y: 32554,
            floor: -13,
        },
        start: Position {
            x: 33045,
            y: 32554,
            floor: -13,
        },
    },
    Sign {
        at: Position {
            x: 33078,
            y: 32566,
            floor: -13,
        },
        start: Position {
            x: 33079,
            y: 32566,
            floor: -13,
        },
    },
    Sign {
        at: Position {
            x: 33086,
            y: 32566,
            floor: -13,
        },
        start: Position {
            x: 33085,
            y: 32566,
            floor: -13,
        },
    },
    Sign {
        at: Position {
            x: 33078,
            y: 32572,
            floor: -13,
        },
        start: Position {
            x: 33079,
            y: 32572,
            floor: -13,
        },
    },
    Sign {
        at: Position {
            x: 33086,
            y: 32572,
            floor: -13,
        },
        start: Position {
            x: 33085,
            y: 32572,
            floor: -13,
        },
    },
];
const CONTROL: Sign = Sign {
    at: Position {
        x: 33060,
        y: 32555,
        floor: -14,
    },
    start: Position {
        x: 33061,
        y: 32555,
        floor: -14,
    },
};
const PREFIX: &str = "oteryn:quest-progress/quest/u7_4/the_ancient_tombs/diprath_sign";
fn run(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED {name}");
    let result = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            super::quest_world_callbacks::scenario(name, start().map_err(|e| e.to_string())?, false)
                .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "native sign scenario panicked")?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}
#[test]
fn l6_native_signs_ordered() -> TestResult {
    run("l6-native-signs-ordered")
}
#[test]
fn l6_native_signs_out_of_order() -> TestResult {
    run("l6-native-signs-out-of-order")
}
#[test]
fn l6_native_signs_missing_middle() -> TestResult {
    run("l6-native-signs-missing-middle")
}
#[test]
fn l6_native_signs_repeat_and_replay() -> TestResult {
    run("l6-native-signs-repeat")
}
#[test]
fn l6_native_signs_unrelated_control() -> TestResult {
    run("l6-native-signs-control")
}
pub(super) fn start() -> TestResult<Position> {
    Ok(SIGNS.first().ok_or("first sign")?.start)
}
pub(super) fn world(native: &NativeGameplayState) -> TestResult<crate::map::boot::BundleWorld> {
    super::l6_world_pickups::native_scene(native, crops()?, &[])
}
pub(super) fn crops() -> TestResult<Vec<super::l6_world_pickups::NativeCrop>> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop};
    let start = start()?;
    let mut crops = Vec::new();
    for s in SIGNS.iter().chain(std::iter::once(&CONTROL)) {
        let (bytes, region) = if s.at.floor == -13 {
            (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z13-x129-y127.b3"
                ))
                .as_slice(),
                (13, 129, 127),
            )
        } else {
            (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z14-x129-y127.b3"
                ))
                .as_slice(),
                (14, 129, 127),
            )
        };
        for (p, ids) in [(s.start, vec![415]), (s.at, vec![415, 2190])] {
            crops.push(NativeCrop {
                bytes,
                region,
                bounds: (p.x, p.y, p.x + 1, p.y + 1),
                start: (start.x, start.y),
                expected: vec![ExpectedTile {
                    position: (p.x, p.y),
                    ids,
                    counted_entry: None,
                }],
            });
        }
    }
    Ok(crops)
}
pub(super) async fn relocate(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    p: Position,
) -> TestResult {
    let map = owner.bundle_map().ok_or("map")?;
    let doors = crate::map::floor::BundleDoors(map.facts.doors());
    let mut runtime = owner.runtime.lock().await;
    let expected = runtime.read_actor_position(actor).map_err(debug)?;
    let at = super::quest_world_callbacks::local(p);
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
async fn use_sign(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    s: Sign,
) -> TestResult<(UseDisposition, u64, Vec<u8>, Vec<u8>)> {
    relocate(
        owner,
        admitted.runtime_actor.ok_or("actor")?,
        admitted.game_session_id,
        s.start,
    )
    .await?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(1))
        .ok_or("use clock")?;
    let map = owner.bundle_map().ok_or("map")?;
    let key = oteryn_world_bundle::bundle::placement_key(s.at.floor, s.at.x, s.at.y, 1)
        .ok_or("native object key")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    })
    .map_err(debug)?;
    let ordinal = admitted.continuity.next_command_id;
    let reply =
        super::quest_world_callbacks::wire(owner, admitted, ordinal, 2, payload.clone()).await?;
    let disposition = decode_use_result(&reply).map_err(debug)?;
    Ok((disposition, ordinal, payload, reply))
}
pub(super) async fn activate_signs(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    count: usize,
) -> TestResult {
    if count > SIGNS.len() {
        return Err("sign count".into());
    }
    for sign in SIGNS.iter().copied().take(count) {
        let (result, _, _, _) = use_sign(owner, admitted, sign).await?;
        assert_eq!(result, UseDisposition::Committed);
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
    }
    Ok(())
}
async fn snapshot(h: &Harness) -> TestResult<Vec<serde_json::Value>> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_object('tracks',(SELECT jsonb_agg(to_jsonb(t) ORDER BY track_key) FROM game_character_quest_tracks t),'receipts',(SELECT jsonb_agg(to_jsonb(r) ORDER BY committed_character_revision) FROM game_character_quest_receipts r))").fetch_all(&h.pool).await?)
}
async fn progress(h: &Harness, i: usize) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),-1)",
    )
    .bind(format!("{PREFIX}{}", i + 1))
    .fetch_one(&h.pool)
    .await?)
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("POSTGRES_DATABASE_READY l6_native_signs version={version}");
    let initial = snapshot(h).await?;
    if name.ends_with("-control") {
        let (result, _, _, _) = use_sign(owner, admitted, CONTROL).await?;
        assert_ne!(result, UseDisposition::Committed);
        assert_eq!(
            snapshot(h).await?,
            initial,
            "unrelated same-appearance native object cannot grant any sign progress"
        );
        return Ok(());
    }
    if name.ends_with("out-of-order") {
        let (result, _, _, _) = use_sign(owner, admitted, SIGNS[7]).await?;
        assert_eq!(result, UseDisposition::Rejected);
        assert_eq!(snapshot(h).await?, initial);
    }
    let limit = if name.ends_with("missing-middle") {
        3
    } else {
        8
    };
    for (i, s) in SIGNS.iter().copied().take(limit).enumerate() {
        let (result, ordinal, payload, reply) = use_sign(owner, admitted, s).await?;
        assert_eq!(
            result,
            UseDisposition::Committed,
            "exact native USE commits source ordered sign"
        );
        assert_eq!(progress(h, i).await?, 1);
        for future in i + 1..8 {
            assert_eq!(
                progress(h, future).await?,
                -1,
                "using a sign never grants a future step"
            );
        }
        let committed = snapshot(h).await?;
        assert_eq!(
            super::quest_world_callbacks::wire(owner, admitted, ordinal, 2, payload).await?,
            reply
        );
        assert_eq!(
            snapshot(h).await?,
            committed,
            "receipt replay cannot repeat the flag or revision"
        );
        if name.ends_with("-repeat") {
            let (again, _, _, _) = use_sign(owner, admitted, s).await?;
            assert_eq!(again, UseDisposition::Rejected);
            assert_eq!(
                snapshot(h).await?,
                committed,
                "already-set sign keeps the source reentry refusal"
            );
        }
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
    }
    if limit == 3 {
        let before = snapshot(h).await?;
        let (result, _, _, _) = use_sign(owner, admitted, SIGNS[4]).await?;
        assert_eq!(result, UseDisposition::Rejected);
        assert_eq!(snapshot(h).await?, before);
        for i in 3..8 {
            assert_eq!(progress(h, i).await?, -1);
        }
    }
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(receipts, i64::try_from(limit)?);
    let leases: i64 = sqlx::query_scalar("SELECT count(*) FROM game_shared_switch_leases")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        leases, 0,
        "personal ordered signs cannot activate shared switches"
    );
    Ok(())
}
