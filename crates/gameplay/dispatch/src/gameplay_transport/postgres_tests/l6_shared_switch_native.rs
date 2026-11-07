//! Exact retained B3 switch counterparts, with unrelated native lever controls.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    fixture::{Harness, debug},
};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
#[test]
fn l6_native_switch_present_12121() -> TestResult {
    run("l6-tomb-switch-native-present-12121")
}
#[test]
fn l6_native_switch_control_12121() -> TestResult {
    run("l6-tomb-switch-native-control-12121")
}
#[test]
fn l6_native_switch_present_12122() -> TestResult {
    run("l6-tomb-switch-native-present-12122")
}
#[test]
fn l6_native_switch_control_12122() -> TestResult {
    run("l6-tomb-switch-native-control-12122")
}
#[test]
fn l6_native_switch_present_12123() -> TestResult {
    run("l6-tomb-switch-native-present-12123")
}
#[test]
fn l6_native_switch_control_12123() -> TestResult {
    run("l6-tomb-switch-native-control-12123")
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
        .map_err(|_| "native switch fixture panicked")?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}
pub(super) fn position(name: &str) -> TestResult<Position> {
    let (p, _) = case(name)?;
    Ok(Position {
        x: p.0,
        y: p.1,
        floor: -(p.2 as i8),
    })
}
pub(super) fn start(name: &str) -> TestResult<Position> {
    let (_, p) = case(name)?;
    Ok(Position {
        x: p.0,
        y: p.1,
        floor: -(p.2 as i8),
    })
}
type NativePoint = (u16, u16, u8);
type NativeCase = (NativePoint, NativePoint);
type NativeRegionSource = (&'static [u8], (u8, u16, u16));
fn case(name: &str) -> TestResult<NativeCase> {
    if !matches!(name.rsplit('-').next(), Some("12121" | "12122" | "12123")) {
        return Err("invalid native source fixture name".into());
    }
    Ok(
        match (
            name.ends_with("12121"),
            name.ends_with("12122"),
            name.contains("-control-"),
        ) {
            (true, false, false) => ((33382, 32786, 14), (33381, 32786, 14)),
            (true, false, true) => ((33125, 32790, 14), (33125, 32791, 14)),
            (false, true, false) => ((33089, 32514, 13), (33088, 32514, 13)),
            (false, true, true) => ((32821, 32341, 13), (32820, 32341, 13)),
            (false, false, false) => ((33183, 32884, 11), (33182, 32884, 11)),
            (false, false, true) => ((32843, 32649, 11), (32842, 32649, 11)),
            _ => return Err("invalid native source fixture name".into()),
        },
    )
}
fn region(pos: NativePoint) -> TestResult<NativeRegionSource> {
    Ok(match (pos.2, pos.0 / 256, pos.1 / 256) {
        (11, 128, 127) => (
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z11-x128-y127.b3"
            )),
            (11, 128, 127),
        ),
        (11, 129, 128) => (
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z11-x129-y128.b3"
            )),
            (11, 129, 128),
        ),
        (13, 128, 126) => (
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z13-x128-y126.b3"
            )),
            (13, 128, 126),
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
        (14, 129, 128) => (
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z14-x129-y128.b3"
            )),
            (14, 129, 128),
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
        _ => return Err("unselected native switch region".into()),
    })
}
pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
    name: &str,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene};
    let (p, start) = case(name)?;
    let mut points = std::collections::BTreeSet::from([p, start]);
    if name.ends_with("12121") {
        points.extend([
            (33320, 32682, 14),
            (33338, 32702, 14),
            (33349, 32680, 14),
            (33358, 32701, 14),
            (33305, 32734, 14),
            (33357, 32749, 14),
            (33368, 32763, 14),
            (33382, 32786, 14),
        ]);
    }
    if name.ends_with("12122") {
        points.extend([
            (33006, 32563, 13),
            (33055, 32487, 13),
            (33036, 32507, 13),
            (33077, 32507, 13),
            (33130, 32489, 13),
            (33027, 32530, 13),
            (33089, 32514, 13),
            (33104, 32514, 13),
            (33147, 32524, 13),
            (33028, 32588, 13),
            (33123, 32599, 13),
        ]);
    }
    if name.ends_with("12123") {
        points.extend([
            (33176, 32880, 11),
            (33182, 32880, 11),
            (33175, 32884, 11),
            (33183, 32884, 11),
            (33176, 32889, 11),
            (33181, 32889, 11),
        ]);
    }
    let mut specs = Vec::new();
    for at in points {
        let (bytes, r) = region(at)?;
        let ids = match at {
            (32820, 32341, 13) => vec![352],
            (32821, 32341, 13) => vec![351, 2772],
            (32842, 32649, 11) => vec![500],
            (32843, 32649, 11) => vec![500, 2772],
            (33006, 32563, 13) => vec![415, 2772],
            (33027, 32530, 13) => vec![415, 2772],
            (33028, 32588, 13) => vec![415, 2772],
            (33036, 32507, 13) => vec![415, 2772],
            (33055, 32487, 13) => vec![415, 2772],
            (33077, 32507, 13) => vec![415, 2772],
            (33088, 32514, 13) => vec![415],
            (33089, 32514, 13) => vec![415, 2772],
            (33104, 32514, 13) => vec![415, 2772],
            (33123, 32599, 13) => vec![423, 2772],
            (33125, 32790, 14) => vec![415, 2318, 2772],
            (33125, 32791, 14) => vec![415],
            (33130, 32489, 13) => vec![423, 2772],
            (33147, 32524, 13) => vec![423, 2772],
            (33175, 32884, 11) => vec![415, 2342, 2772],
            (33176, 32880, 11) => vec![415, 2343, 2772],
            (33176, 32889, 11) => vec![415, 2342, 2772],
            (33181, 32889, 11) => vec![415, 2342, 2772],
            (33182, 32880, 11) => vec![415, 2342, 2772],
            (33182, 32884, 11) => vec![417],
            (33183, 32884, 11) => vec![415, 2342, 2772],
            (33305, 32734, 14) => vec![415, 2772],
            (33320, 32682, 14) => vec![415, 2772],
            (33338, 32702, 14) => vec![417, 2772],
            (33349, 32680, 14) => vec![415, 2772],
            (33357, 32749, 14) => vec![415, 2772],
            (33358, 32701, 14) => vec![415, 2772],
            (33368, 32763, 14) => vec![415, 2772],
            (33381, 32786, 14) => vec![415],
            (33382, 32786, 14) => vec![415, 2772],
            _ => return Err("unwitnessed switch tile".into()),
        };
        specs.push(NativeCrop {
            bytes,
            region: r,
            bounds: (at.0, at.1, at.0 + 1, at.1 + 1),
            start: (start.0, start.1),
            expected: vec![ExpectedTile {
                position: (at.0, at.1),
                ids,
                counted_entry: None,
            }],
        });
    }
    native_scene(native, specs, &[])
}
pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    trigger: Position,
    name: &str,
) -> TestResult {
    let v: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("POSTGRES_DATABASE_READY l6_native_switch version={v}");
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    let ordinal = if (name.contains("-control-") && name.ends_with("12121"))
        || (!name.contains("-control-") && name.ends_with("12123"))
    {
        2
    } else {
        1
    };
    let key =
        oteryn_world_bundle::bundle::placement_key(trigger.floor, trigger.x, trigger.y, ordinal)
            .ok_or("native lever")?;
    let map = owner.bundle_map().ok_or("native switch map")?;
    let at = crate::map::overlay::TilePos {
        x: trigger.x,
        y: trigger.y,
        floor: trigger.floor,
    };
    let base = map
        .overlay
        .base()
        .tile(trigger.x, trigger.y, trigger.floor)
        .ok_or("native target tile")?;
    let id = base
        .ids()
        .iter()
        .zip(base.depths())
        .filter(|(_, d)| **d == 0)
        .nth(usize::from(ordinal))
        .map(|(id, _)| *id)
        .ok_or("native target ordinal")?;
    assert_eq!(
        map.facts
            .base_entry(at, ordinal, id)
            .ok_or("native target facts")?
            .appearance_id,
        2772
    );

    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: 0,
    })
    .map_err(debug)?;
    let reply = super::quest_world_callbacks::wire(owner, admitted, 1, 2, payload.clone()).await?;
    let disposition =
        oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?;
    let control = name.contains("-control-");
    if !control {
        assert_eq!(
            disposition,
            UseDisposition::Committed,
            "exact authored native counterpart commits source world lease"
        );
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_shared_switch_leases")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        count,
        i64::from(!control),
        "unrelated same-appearance lever cannot enter a quest counter"
    );
    assert_eq!(
        super::quest_world_callbacks::wire(owner, admitted, 1, 2, payload).await?,
        reply
    );
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_shared_switch_leases")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(after, count);
    let quests: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        quests, before,
        "native activation grants no early personal progress"
    );
    Ok(())
}
