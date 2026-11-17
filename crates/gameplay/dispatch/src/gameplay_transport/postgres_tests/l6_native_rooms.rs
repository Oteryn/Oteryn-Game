//! Exact native interaction pockets; no claim of traversing the intervening maze.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    fixture::{Harness, debug},
};
use crate::content::native_gameplay::NativeGameplayState;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
    world_callbacks::{Callback, Position},
};
use oteryn_protocol_oteryn::world_spatial::*;
use std::sync::Arc;
#[derive(Clone, Copy)]
struct Case {
    suffix: &'static str,
    trigger: Position,
    ground: u32,
    start_ground: u32,
    goal: Position,
    goal_ground: u32,
    before: i64,
}
const CASES: &[Case] = &[
    Case {
        suffix: "morguthis-entry",
        trigger: Position {
            x: 33258,
            y: 32707,
            floor: -13,
        },
        ground: 415,
        start_ground: 415,
        goal: Position {
            x: 33259,
            y: 32707,
            floor: -13,
        },
        goal_ground: 415,
        before: 2,
    },
    Case {
        suffix: "omruc-specific",
        trigger: Position {
            x: 33205,
            y: 32956,
            floor: -14,
        },
        ground: 415,
        start_ground: 415,
        goal: Position {
            x: 33205,
            y: 32955,
            floor: -14,
        },
        goal_ground: 415,
        before: 0,
    },
    Case {
        suffix: "omruc-room",
        trigger: Position {
            x: 33206,
            y: 33002,
            floor: -14,
        },
        ground: 1236,
        start_ground: 1245,
        goal: Position {
            x: 33207,
            y: 33002,
            floor: -14,
        },
        goal_ground: 1236,
        before: 1,
    },
    Case {
        suffix: "rahemos-levers",
        trigger: Position {
            x: 33122,
            y: 32760,
            floor: -14,
        },
        ground: 415,
        start_ground: 415,
        goal: Position {
            x: 33124,
            y: 32760,
            floor: -14,
        },
        goal_ground: 415,
        before: 2,
    },
    Case {
        suffix: "rahemos-room",
        trigger: Position {
            x: 33085,
            y: 32781,
            floor: -14,
        },
        ground: 423,
        start_ground: 423,
        goal: Position {
            x: 33086,
            y: 32781,
            floor: -14,
        },
        goal_ground: 423,
        before: 0,
    },
    Case {
        suffix: "vashresamun-specific",
        trigger: Position {
            x: 33194,
            y: 32666,
            floor: -15,
        },
        ground: 423,
        start_ground: 417,
        goal: Position {
            x: 33194,
            y: 32665,
            floor: -15,
        },
        goal_ground: 423,
        before: 0,
    },
    Case {
        suffix: "vashresamun-specific-preserve",
        trigger: Position {
            x: 33194,
            y: 32666,
            floor: -15,
        },
        ground: 423,
        start_ground: 417,
        goal: Position {
            x: 33194,
            y: 32665,
            floor: -15,
        },
        goal_ground: 423,
        before: 3,
    },
    Case {
        suffix: "vashresamun-room",
        trigger: Position {
            x: 33128,
            y: 32657,
            floor: -15,
        },
        ground: 460,
        start_ground: 454,
        goal: Position {
            x: 33129,
            y: 32657,
            floor: -15,
        },
        goal_ground: 459,
        before: 1,
    },
];
fn spec(name: &str) -> TestResult<Case> {
    CASES
        .iter()
        .find(|c| name.ends_with(c.suffix))
        .copied()
        .ok_or_else(|| "native room case".into())
}
pub(super) fn start(name: &str) -> TestResult<Position> {
    let c = spec(name)?;
    Ok(Position {
        x: c.trigger.x - 1,
        ..c.trigger
    })
}
pub(super) fn callback(name: &str) -> TestResult<Callback> {
    let c = spec(name)?;
    oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .find(|v| {
            v.key.ends_with(&format!(
                "/ancient-tombs-{}",
                c.suffix.strip_suffix("-preserve").unwrap_or(c.suffix)
            ))
        })
        .ok_or_else(|| "source room callback".into())
}
fn run(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED {name}");
    let result = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            super::quest_world_callbacks::scenario(
                name,
                spec(name).map_err(|e| e.to_string())?.trigger,
                false,
            )
            .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "native room scenario panicked")?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}
#[test]
fn l6_native_room_morguthis_entry() -> TestResult {
    run("l6-native-room-morguthis-entry")
}
#[test]
fn l6_native_room_omruc_specific() -> TestResult {
    run("l6-native-room-omruc-specific")
}
#[test]
fn l6_native_room_omruc_room() -> TestResult {
    run("l6-native-room-omruc-room")
}
#[test]
fn l6_native_room_rahemos_levers() -> TestResult {
    run("l6-native-room-rahemos-levers")
}
#[test]
fn l6_native_room_rahemos_room() -> TestResult {
    run("l6-native-room-rahemos-room")
}

#[test]
fn l6_native_room_vashresamun_specific() -> TestResult {
    run("l6-native-room-vashresamun-specific")
}
#[test]
fn l6_native_room_vashresamun_specific_preserve() -> TestResult {
    run("l6-native-room-vashresamun-specific-preserve")
}
#[test]
fn l6_native_room_vashresamun_room() -> TestResult {
    run("l6-native-room-vashresamun-room")
}

type RegionSource = (&'static [u8], (u8, u16, u16));
fn region(p: Position) -> TestResult<RegionSource> {
    let path = (p.floor, p.x / 256, p.y / 256);
    match path {
        (-15, 129, 127) => Ok((
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z15-x129-y127.b3"
            )),
            (15, 129, 127),
        )),
        (-13, 129, 127) => Ok((
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z13-x129-y127.b3"
            )),
            (13, 129, 127),
        )),
        (-14, 129, 127) => Ok((
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z14-x129-y127.b3"
            )),
            (14, 129, 127),
        )),
        (-14, 129, 128) => Ok((
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z14-x129-y128.b3"
            )),
            (14, 129, 128),
        )),
        _ => Err("native room region".into()),
    }
}
pub(super) fn world(
    native: &NativeGameplayState,
    name: &str,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene};
    let c = spec(name)?;
    let start = start(name)?;
    let mut crops = Vec::new();
    for (p, id) in [
        (start, c.start_ground),
        (c.trigger, c.ground),
        (c.goal, c.goal_ground),
    ] {
        let (bytes, region) = region(p)?;
        crops.push(NativeCrop {
            bytes,
            region,
            bounds: (p.x, p.y, p.x + 1, p.y + 1),
            start: (start.x, start.y),
            expected: vec![ExpectedTile {
                position: (p.x, p.y),
                ids: vec![id],
                counted_entry: None,
            }],
        });
    }
    native_scene(native, crops, &[])
}
fn tile(p: Position) -> crate::map::overlay::TilePos {
    crate::map::overlay::TilePos {
        x: p.x,
        y: p.y,
        floor: p.floor,
    }
}
async fn step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    ordinal: u64,
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
        encode_step_intent(StepDirection::East),
    )
    .await?;
    assert_eq!(
        decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    Ok(())
}
async fn position(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<oteryn_gameplay_entities::MovementLocalPosition> {
    Ok(owner
        .runtime
        .lock()
        .await
        .read_actor_position(admitted.runtime_actor.ok_or("native actor")?)
        .map_err(debug)?
        .position())
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
    eprintln!("POSTGRES_DATABASE_READY l6_native_room version={version}");
    let c = spec(name)?;
    let callback = callback(name)?;
    let origin = start(name)?;
    let initial = snapshot(h).await?;
    let closed = admitted.continuity.next_command_id;
    step(owner, admitted, closed).await?;
    assert_eq!(
        position(owner, admitted).await?,
        super::quest_world_callbacks::local(origin),
        "missing DefaultStart must restore the real native approach"
    );
    assert_eq!(
        snapshot(h).await?,
        initial,
        "native guard refusal writes no Quest state or receipt"
    );
    assert_eq!(
        owner
            .bundle_map()
            .ok_or("map")?
            .facts
            .movement_effects_since(0),
        vec![(1, tile(origin), 11)]
    );
    step(owner, admitted, closed).await?;
    assert_eq!(snapshot(h).await?, initial);
    assert_eq!(
        owner
            .bundle_map()
            .ok_or("map")?
            .facts
            .movement_effects_since(0)
            .len(),
        1
    );
    seed(owner, admitted, &callback, c.before).await?;
    let opened = admitted.continuity.next_command_id;
    step(owner, admitted, opened).await?;
    assert_eq!(
        position(owner, admitted).await?,
        super::quest_world_callbacks::local(c.goal),
        "eligible room uses literal source landing"
    );
    let state = &callback.storage[1];
    let value: i64 =
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(&state.track)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(value, c.before.max(state.set));
    let eligibility: i64 =
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(&callback.storage[0].track)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(eligibility, 1);
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(&callback.key)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(receipts, 1);
    assert_eq!(
        owner
            .bundle_map()
            .ok_or("map")?
            .facts
            .movement_effects_since(0),
        vec![(1, tile(origin), 11), (2, tile(c.goal), 11)]
    );
    let committed = snapshot(h).await?;
    step(owner, admitted, opened).await?;
    assert_eq!(
        position(owner, admitted).await?,
        super::quest_world_callbacks::local(c.goal)
    );
    assert_eq!(snapshot(h).await?, committed);
    assert_eq!(
        owner
            .bundle_map()
            .ok_or("map")?
            .facts
            .movement_effects_since(0)
            .len(),
        2
    );
    Ok(())
}
async fn seed(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    callback: &Callback,
    before: i64,
) -> TestResult {
    let canonical = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
    let tracks = callback
        .storage
        .iter()
        .map(|s| canonical.track(&s.track).cloned().ok_or("source track"))
        .collect::<Result<Vec<_>, _>>()?;
    let key = "oteryn:quest-transition/native-room-fixture-ready";
    let setup = QuestTransition {
world_change: None,
        key: key.into(),
        quest: callback.quest.clone(),
        effects: callback
            .storage
            .iter()
            .enumerate()
            .map(|(i, s)| QuestEffect {
                track: s.track.clone(),
                from: QuestComparison::Any,
                effect: QuestEffectKind::Set(if i == 0 { 1 } else { before }),
            })
            .collect(),
        completes: false,
        experience: None,
    };
    let catalogue = Arc::new(
        QuestStateCatalogue::new(
            canonical.content_revision(),
            tracks,
            vec![
                setup,
                oteryn_gameplay_quests::quest::world_callbacks::primary_transition(callback)?,
            ],
        )
        .map_err(debug)?,
    );
    owner.quest_catalogue = Some(Arc::clone(&catalogue));
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let outcome = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: key.into(),
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(9000).map_err(debug)?,
                )),
            },
            catalogue,
        )
        .await
        .map_err(debug)?;
    assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)));
    drop(slot);
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    Ok(())
}
async fn snapshot(h: &Harness) -> TestResult<Vec<serde_json::Value>> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_object('tracks',(SELECT jsonb_agg(to_jsonb(t) ORDER BY track_key) FROM game_character_quest_tracks t),'receipts',(SELECT jsonb_agg(to_jsonb(r) ORDER BY committed_character_revision) FROM game_character_quest_receipts r))").fetch_all(&h.pool).await?)
}
