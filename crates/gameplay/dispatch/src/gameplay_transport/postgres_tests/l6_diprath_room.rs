//! Composed native shared-or-personal entry. Sealed pocket setup, never SQL flags.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    fixture::{Harness, debug},
};
use crate::content::native_gameplay::NativeGameplayState;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
};
use oteryn_protocol_oteryn::world_spatial::*;
const KEY: &str = "oteryn:quest-callback/ancient-tombs-diprath-room";
const TRACK: &str = "oteryn:quest-track/ancient-tombs/diprath-treasure";
const ORIGIN: Position = Position {
    x: 33092,
    y: 32589,
    floor: -15,
};
const TRIGGER: Position = Position {
    x: 33093,
    y: 32589,
    floor: -15,
};
const GOAL: Position = Position {
    x: 33092,
    y: 32590,
    floor: -15,
};
#[derive(Clone, Copy)]
struct Switch {
    at: (u16, u16),
    start: (u16, u16),
    ground: u32,
}
const SWITCHES: &[Switch] = &[
    Switch {
        at: (33006, 32563),
        start: (33007, 32563),
        ground: 415,
    },
    Switch {
        at: (33036, 32507),
        start: (33037, 32507),
        ground: 415,
    },
    Switch {
        at: (33077, 32507),
        start: (33076, 32507),
        ground: 415,
    },
    Switch {
        at: (33027, 32530),
        start: (33027, 32531),
        ground: 415,
    },
    Switch {
        at: (33089, 32514),
        start: (33088, 32514),
        ground: 415,
    },
    Switch {
        at: (33104, 32514),
        start: (33103, 32514),
        ground: 415,
    },
    Switch {
        at: (33147, 32524),
        start: (33146, 32524),
        ground: 423,
    },
];
pub(super) fn start() -> TestResult<Position> {
    super::l6_native_signs::start()
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
            super::quest_world_callbacks::scenario(name, TRIGGER, false).map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "Diprath room scenario panicked")?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}
#[test]
fn l6_diprath_room_closed_six_switches_seven_signs() -> TestResult {
    run("l6-diprath-room-closed")
}
#[test]
fn l6_diprath_room_shared_seven_without_signs() -> TestResult {
    run("l6-diprath-room-shared")
}
#[test]
fn l6_diprath_room_personal_eight_without_switches() -> TestResult {
    run("l6-diprath-room-personal")
}
pub(super) fn world(native: &NativeGameplayState) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene};
    let start = start()?;
    let mut crops = super::l6_native_signs::crops()?;
    let source15 = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/world/placements/region-z15-x129-y127.b3"
    ))
    .as_slice();
    for s in SWITCHES {
        for (p, ids) in [(s.at, vec![s.ground, 2772]), (s.start, vec![s.ground])] {
            let (bytes, region) = super::l6_shared_gate_native::region((p.0, p.1, 13))?;
            crops.push(NativeCrop {
                bytes,
                region,
                bounds: (p.0, p.1, p.0 + 1, p.1 + 1),
                start: (start.x, start.y),
                expected: vec![ExpectedTile {
                    position: p,
                    ids,
                    counted_entry: None,
                }],
            });
        }
    }
    for (p, id) in [(ORIGIN, 418), (TRIGGER, 423), (GOAL, 418)] {
        crops.push(NativeCrop {
            bytes: source15,
            region: (15, 129, 127),
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
async fn use_switch(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    s: Switch,
) -> TestResult {
    super::l6_native_signs::relocate(
        owner,
        admitted.runtime_actor.ok_or("actor")?,
        admitted.game_session_id,
        Position {
            x: s.start.0,
            y: s.start.1,
            floor: -13,
        },
    )
    .await?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(1))
        .ok_or("switch clock")?;
    let map = owner.bundle_map().ok_or("map")?;
    let key =
        oteryn_world_bundle::bundle::placement_key(-13, s.at.0, s.at.1, 1).ok_or("switch key")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    })
    .map_err(debug)?;
    let ordinal = admitted.continuity.next_command_id;
    let reply = super::quest_world_callbacks::wire(owner, admitted, ordinal, 2, payload).await?;
    assert_eq!(
        decode_use_result(&reply).map_err(debug)?,
        UseDisposition::Committed
    );
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    Ok(())
}
async fn snapshot(h: &Harness) -> TestResult<serde_json::Value> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_object('tracks',(SELECT jsonb_agg(to_jsonb(t) ORDER BY track_key) FROM game_character_quest_tracks t),'receipts',(SELECT jsonb_agg(to_jsonb(r) ORDER BY committed_character_revision) FROM game_character_quest_receipts r),'leases',(SELECT jsonb_agg(to_jsonb(l) ORDER BY to_jsonb(l)::text) FROM game_shared_switch_leases l))").fetch_one(&h.pool).await?)
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
    eprintln!("POSTGRES_DATABASE_READY l6_diprath_room version={version}");
    let (switches, signs, open) = if name.ends_with("-shared") {
        (7, 0, true)
    } else if name.ends_with("-personal") {
        (0, 8, true)
    } else if name.ends_with("-closed") {
        (6, 7, false)
    } else {
        return Err("Diprath room case".into());
    };
    for s in SWITCHES.iter().copied().take(switches) {
        use_switch(owner, admitted, s).await?;
    }
    super::l6_native_signs::activate_signs(owner, admitted, signs).await?;
    let leases: i64 = sqlx::query_scalar("SELECT count(*) FROM game_shared_switch_leases")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(leases, i64::try_from(switches)?);
    for i in 0..8 {
        let value:i64=sqlx::query_scalar("SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),-1)").bind(format!("oteryn:quest-progress/quest/u7_4/the_ancient_tombs/diprath_sign{}",i+1)).fetch_one(&h.pool).await?;
        assert_eq!(value, if i < signs { 1 } else { -1 });
    }
    super::l6_native_signs::relocate(
        owner,
        admitted.runtime_actor.ok_or("actor")?,
        admitted.game_session_id,
        ORIGIN,
    )
    .await?;
    let before = snapshot(h).await?;
    let effects_before = owner
        .bundle_map()
        .ok_or("map")?
        .facts
        .movement_effects_since(0)
        .len();
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(1))
        .ok_or("step clock")?;
    let ordinal = admitted.continuity.next_command_id;
    let payload = encode_step_intent(StepDirection::East);
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        ordinal,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        payload.clone(),
    )
    .await?;
    assert_eq!(
        decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    let expected = if open { GOAL } else { ORIGIN };
    let actual = owner
        .runtime
        .lock()
        .await
        .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position();
    assert_eq!(
        actual,
        super::quest_world_callbacks::local(expected),
        "shared threshold OR personal completion controls the exact room landing"
    );
    let value: i64 = sqlx::query_scalar(
        "SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),-1)",
    )
    .bind(TRACK)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(value, if open { 3 } else { -1 });
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(KEY)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(receipts, i64::from(open));
    let after = snapshot(h).await?;
    if !open {
        assert_eq!(
            after, before,
            "neither alternative permits any partial Quest or shared state writes"
        );
    }
    let effects = owner
        .bundle_map()
        .ok_or("map")?
        .facts
        .movement_effects_since(0);
    assert_eq!(effects.len(), effects_before + 1);
    let last = effects.last().ok_or("room cue")?;
    assert_eq!(
        (last.1, last.2),
        (
            crate::map::overlay::TilePos {
                x: expected.x,
                y: expected.y,
                floor: expected.floor
            },
            11
        )
    );
    assert_eq!(
        super::quest_world_callbacks::wire(
            owner,
            admitted,
            ordinal,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            payload
        )
        .await?,
        reply
    );
    assert_eq!(snapshot(h).await?, after);
    assert_eq!(
        owner
            .bundle_map()
            .ok_or("map")?
            .facts
            .movement_effects_since(0),
        effects
    );
    Ok(())
}
