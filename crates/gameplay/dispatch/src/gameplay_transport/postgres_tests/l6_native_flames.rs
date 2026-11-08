//! Seven real native STEP flags, then the source all-flags gate. No SQL flag setup.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    fixture::{Harness, debug},
};
use crate::content::native_gameplay::NativeGameplayState;
use crate::foundation::{ExactActorRef, GameSessionId};
use oteryn_gameplay_quests::quest::world_callbacks::{NativeSelector, Position};
use oteryn_protocol_oteryn::world_spatial::*;
use std::collections::BTreeMap;
const PREFIX: &str = "oteryn:quest-progress/quest/u7_4/the_ancient_tombs/";
const GATE: Position = Position {
    x: 33163,
    y: 32695,
    floor: -14,
};
const ORIGIN: Position = Position {
    x: 33163,
    y: 32696,
    floor: -14,
};
const GOAL: Position = Position {
    x: 33163,
    y: 32694,
    floor: -14,
};
const START_IDS: &[u32] = &[417, 417, 417, 417, 417, 423, 417];
fn run(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED {name}");
    let result = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            super::quest_world_callbacks::scenario(name, GATE, false).map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "native flame scenario panicked")?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}
#[test]
fn l6_native_flames_missing_last() -> TestResult {
    run("l6-native-flames-missing-last")
}
#[test]
fn l6_native_flames_missing_middle() -> TestResult {
    run("l6-native-flames-missing-middle")
}
#[test]
fn l6_native_flames_all_seven() -> TestResult {
    run("l6-native-flames-all")
}
#[test]
fn l6_native_flames_unordered() -> TestResult {
    run("l6-native-flames-unordered")
}
#[test]
fn l6_native_flames_repeat_and_replay() -> TestResult {
    run("l6-native-flames-repeat")
}
fn bindings() -> TestResult<Vec<NativeSelector>> {
    let value: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/quests/native-counterparts.json"
    )))?;
    let rows = value["counterparts"].as_array().ok_or("counterpart rows")?;
    (1..=7)
        .map(|i| {
            let key = format!("oteryn:quest-callback/ancient-tombs-morguthis-flame-{i}");
            let row = rows
                .iter()
                .find(|v| v["callback"].as_str() == Some(key.as_str()))
                .ok_or("flame metadata")?;
            Ok(serde_json::from_value(row["native"].clone())?)
        })
        .collect()
}
pub(super) fn start() -> TestResult<Position> {
    let b = bindings()?;
    let p = b.first().ok_or("first marker")?.position;
    Ok(Position { x: p.x - 1, ..p })
}
pub(super) fn world(native: &NativeGameplayState) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene};
    let mut points = BTreeMap::new();
    let bindings = bindings()?;
    for (b, id) in bindings.iter().zip(START_IDS) {
        points.insert((b.position.x, b.position.y), u32::from(b.appearance));
        points.insert((b.position.x - 1, b.position.y), *id);
    }
    for (p, id) in [(GATE, 417), (ORIGIN, 423), (GOAL, 423)] {
        assert!(
            points.insert((p.x, p.y), id).is_none(),
            "marker/gate footprints do not overlap"
        );
    }
    let start = start()?;
    let crops = points
        .into_iter()
        .map(|((x, y), id)| NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z14-x129-y127.b3"
            )),
            region: (14, 129, 127),
            bounds: (x, y, x + 1, y + 1),
            start: (start.x, start.y),
            expected: vec![ExpectedTile {
                position: (x, y),
                ids: vec![id],
                counted_entry: None,
            }],
        })
        .collect();
    native_scene(native, crops, &[])
}
fn tile(p: Position) -> crate::map::overlay::TilePos {
    crate::map::overlay::TilePos {
        x: p.x,
        y: p.y,
        floor: p.floor,
    }
}
async fn relocate(
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
async fn step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    ordinal: u64,
    direction: StepDirection,
) -> TestResult {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(1))
        .ok_or("step clock")?;
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        ordinal,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(direction),
    )
    .await?;
    assert_eq!(
        decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    Ok(())
}
async fn value(h: &Harness, track: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),-1)",
    )
    .bind(track)
    .fetch_one(&h.pool)
    .await?)
}
async fn snapshot(h: &Harness) -> TestResult<Vec<serde_json::Value>> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_object('tracks',(SELECT jsonb_agg(to_jsonb(t) ORDER BY track_key) FROM game_character_quest_tracks t),'receipts',(SELECT jsonb_agg(to_jsonb(r) ORDER BY committed_character_revision) FROM game_character_quest_receipts r))").fetch_all(&h.pool).await?)
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
    eprintln!("POSTGRES_DATABASE_READY l6_native_flames version={version}");
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let bs = bindings()?;
    let mut order: Vec<usize> = (0..7).collect();
    if name.ends_with("missing-last") {
        order.pop();
    } else if name.ends_with("missing-middle") {
        order.retain(|i| *i != 3);
    } else if name.ends_with("unordered") {
        order.reverse();
    } else if name.ends_with("repeat") {
        order.insert(1, 0);
    }
    let mut visited = [false; 7];
    let mut expected_effects = Vec::new();
    for i in order.iter().copied() {
        let p = bs[i].position;
        relocate(
            owner,
            actor,
            admitted.game_session_id,
            Position { x: p.x - 1, ..p },
        )
        .await?;
        let ordinal = admitted.continuity.next_command_id;
        step(owner, admitted, ordinal, StepDirection::East).await?;
        visited[i] = true;
        assert_eq!(
            value(h, &format!("{PREFIX}morguthis_blue_flame_storage{}", i + 1)).await?,
            1,
            "literal native STEP commits the source flame flag"
        );
        expected_effects.push((u64::try_from(expected_effects.len() + 1)?, tile(p), 13));
        assert_eq!(
            owner
                .bundle_map()
                .ok_or("map")?
                .facts
                .movement_effects_since(0),
            expected_effects,
            "every new flame entry emits source blue13, including already-set reentry"
        );
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
    }
    for (i, set) in visited.iter().enumerate() {
        assert_eq!(
            value(h, &format!("{PREFIX}morguthis_blue_flame_storage{}", i + 1)).await?,
            if *set { 1 } else { -1 }
        );
    }
    let before = snapshot(h).await?;
    relocate(owner, actor, admitted.game_session_id, ORIGIN).await?;
    let ordinal = admitted.continuity.next_command_id;
    step(owner, admitted, ordinal, StepDirection::North).await?;
    let open = visited.iter().all(|set| *set);
    let destination = if open { GOAL } else { ORIGIN };
    let actual = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    assert_eq!(actual, super::quest_world_callbacks::local(destination));
    let treasure = value(h, &format!("{PREFIX}morguthis_treasure")).await?;
    assert_eq!(treasure, if open { 2 } else { -1 });
    if !open {
        assert_eq!(
            snapshot(h).await?,
            before,
            "one missing flag cannot partly write other flags, treasure or receipt"
        );
    }
    expected_effects.push((
        u64::try_from(expected_effects.len() + 1)?,
        tile(destination),
        11,
    ));
    assert_eq!(
        owner
            .bundle_map()
            .ok_or("map")?
            .facts
            .movement_effects_since(0),
        expected_effects
    );
    let after = snapshot(h).await?;
    step(owner, admitted, ordinal, StepDirection::North).await?;
    assert_eq!(snapshot(h).await?, after);
    assert_eq!(
        owner
            .bundle_map()
            .ok_or("map")?
            .facts
            .movement_effects_since(0),
        expected_effects
    );
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(receipts, i64::try_from(order.len())? + i64::from(open));
    Ok(())
}
