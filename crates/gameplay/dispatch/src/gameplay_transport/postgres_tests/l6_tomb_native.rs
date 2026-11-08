//! Literal native portal room proofs; no synthetic UID or hidden raw destination.
use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
#[test]
fn l6_tomb_native_missing_3950_uses_source_branch() -> TestResult {
    run(
        "l6-tomb-native-missing-3950",
        Position {
            x: 33183,
            y: 32717,
            floor: -14,
        },
        Position {
            x: 33231,
            y: 32705,
            floor: -8,
        },
    )
}
#[test]
fn l6_tomb_native_present_3950_uses_source_branch() -> TestResult {
    run(
        "l6-tomb-native-present-3950",
        Position {
            x: 33183,
            y: 32717,
            floor: -14,
        },
        Position {
            x: 33182,
            y: 32714,
            floor: -14,
        },
    )
}
#[test]
fn l6_tomb_native_missing_3951_uses_source_branch() -> TestResult {
    run(
        "l6-tomb-native-missing-3951",
        Position {
            x: 33175,
            y: 32937,
            floor: -15,
        },
        Position {
            x: 33282,
            y: 32744,
            floor: -8,
        },
    )
}
#[test]
fn l6_tomb_native_present_3951_uses_source_branch() -> TestResult {
    run(
        "l6-tomb-native-present-3951",
        Position {
            x: 33175,
            y: 32937,
            floor: -15,
        },
        Position {
            x: 33174,
            y: 32934,
            floor: -15,
        },
    )
}
#[test]
fn l6_tomb_native_missing_3952_uses_source_branch() -> TestResult {
    run(
        "l6-tomb-native-missing-3952",
        Position {
            x: 33127,
            y: 32594,
            floor: -15,
        },
        Position {
            x: 33250,
            y: 32832,
            floor: -8,
        },
    )
}
#[test]
fn l6_tomb_native_present_3952_uses_source_branch() -> TestResult {
    run(
        "l6-tomb-native-present-3952",
        Position {
            x: 33127,
            y: 32594,
            floor: -15,
        },
        Position {
            x: 33126,
            y: 32591,
            floor: -15,
        },
    )
}
pub(super) fn run(name: &'static str, trigger: Position, expected: Position) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED {name}");
    let result = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            super::quest_world_callbacks::stateless_scenario(name, trigger, expected)
                .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "native Tomb scenario panicked")?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}
pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
    unique: u16,
) -> TestResult<crate::map::boot::BundleWorld> {
    let recipe = oteryn_gameplay_quests::quest::carried_step::RECIPES
        .iter()
        .find(|r| r.unique == unique)
        .ok_or("source carried recipe")?;
    let policy = native
        .item_policy(recipe.item, "definition-r1")
        .ok_or("source key native policy")?;
    assert_eq!(
        policy.record().semantics.charges,
        crate::content::ReferenceItemField::NotApplicable
    );
    let physical =
        crate::durability::spell_items_abi::QualifiedItemDefinition::from_native_policy(policy)?;
    assert!(
        physical.materializable
            && physical.movable
            && physical.inventory_destination
            && physical.ground_destination
    );
    assert_eq!(physical.stack_maximum, 1);
    assert!(physical.container_capacity.is_none());
    let decay = physical.decay.ok_or("source24h key lifetime")?;
    assert_eq!(decay.duration_millis, 86_400_000);
    assert!(
        decay.target.is_none(),
        "source key expires to0, no replacement item"
    );
    use super::l6_world_pickups::{ExpectedTeleport, ExpectedTile, NativeCrop, native_scene};
    let (bytes, region, portal, success, missing, raw): (&'static [u8], _, _, _, _, _) =
        match unique {
            3950 => (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z14-x129-y127.b3"
                )),
                (14, 129, 127),
                (33183, 32717),
                (33182, 32714),
                (33231, 32705),
                (33231, 32705),
            ),
            3951 => (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z15-x129-y128.b3"
                )),
                (15, 129, 128),
                (33175, 32937),
                (33174, 32934),
                (33282, 32744),
                (33250, 32832),
            ),
            3952 => (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z15-x129-y127.b3"
                )),
                (15, 129, 127),
                (33127, 32594),
                (33126, 32591),
                (33250, 32832),
                (33131, 32566),
            ),
            3953 => (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z15-x129-y127.b3"
                )),
                (15, 129, 127),
                (33146, 32668),
                (33145, 32665),
                (33025, 32868),
                (33206, 32592),
            ),
            3954 => (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z14-x129-y128.b3"
                )),
                (14, 129, 128),
                (33042, 32777),
                (33041, 32774),
                (33133, 32642),
                (33133, 32642),
            ),
            3955 => (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z14-x130-y128.b3"
                )),
                (14, 130, 128),
                (33350, 32830),
                (33349, 32827),
                (33131, 32566),
                (33282, 32744),
            ),
            3956 => (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z14-x129-y128.b3"
                )),
                (14, 129, 128),
                (33187, 33015),
                (33186, 33012),
                (33206, 32592),
                (33250, 32832),
            ),
            _ => return Err("unknown source UID".into()),
        };
    let tile = |position, ids| ExpectedTile {
        position,
        ids,
        counted_entry: None,
    };
    let mut specs = vec![NativeCrop {
        bytes,
        region,
        bounds: (success.0, success.1, portal.0 + 1, portal.1 + 1),
        start: (portal.0, portal.1 - 1),
        expected: vec![
            tile(portal, vec![423, 1949]),
            tile((portal.0, portal.1 - 1), vec![423]),
            tile(success, vec![417]),
        ],
    }];
    for target in std::collections::BTreeSet::from([missing, raw]) {
        let (bytes, region): (&'static [u8], _) = match (target.0 / 256, target.1 / 256) {
            (129, 127) => (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z08-x129-y127.b3"
                )),
                (8, 129, 127),
            ),
            (130, 127) => (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z08-x130-y127.b3"
                )),
                (8, 130, 127),
            ),
            (129, 128) => (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z08-x129-y128.b3"
                )),
                (8, 129, 128),
            ),
            _ => return Err("unknown source destination region".into()),
        };
        specs.push(NativeCrop {
            bytes,
            region,
            bounds: (target.0, target.1, target.0 + 1, target.1 + 1),
            start: target,
            expected: vec![tile(
                target,
                vec![if target == (33133, 32642) { 423 } else { 415 }],
            )],
        });
    }
    let world = native_scene(
        native,
        specs,
        &[ExpectedTeleport {
            position: (portal.0, portal.1, region.0),
            ordinal: 1,
            destination: (raw.0, raw.1, 8),
        }],
    )?;
    let key =
        oteryn_world_bundle::bundle::placement_key(-i8::try_from(region.0)?, portal.0, portal.1, 1)
            .ok_or("native portal key")?;
    assert!(
        !world
            .facts()
            .quest_callback_matches(key, Some(unique), None, None),
        "actual native portal must not acquire a synthetic source UID"
    );
    assert_eq!(
        world.floors().teleport(crate::map::overlay::TilePos {
            x: portal.0,
            y: portal.1,
            floor: -i8::try_from(region.0)?
        }),
        Some(crate::map::overlay::TilePos {
            x: raw.0,
            y: raw.1,
            floor: -8
        }),
        "raw native teleport is retained, including its reachable destination"
    );
    Ok(world)
}
