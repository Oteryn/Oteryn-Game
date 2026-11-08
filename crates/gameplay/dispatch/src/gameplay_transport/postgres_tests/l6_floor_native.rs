//! Authored native altar counterpart: real drops through the existing floor crafting owner.
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::content::native_gameplay::NativeGameplayState;
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

const ALTAR: Position = Position {
    x: 33162,
    y: 32694,
    floor: -14,
};
const CONTROL: Position = Position {
    x: 33163,
    y: 32695,
    floor: -14,
};

pub(super) fn start() -> Position {
    Position {
        x: 33163,
        y: 32694,
        floor: -14,
    }
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
            super::quest_world_callbacks::scenario(name, ALTAR, false).map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "native altar scenario panicked")?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}
#[test]
fn l6_native_altar_assembly() -> TestResult {
    run("l6-floor-assembly-native")
}
#[test]
fn l6_native_altar_missing() -> TestResult {
    run("l6-floor-assembly-native-missing")
}
#[test]
fn l6_native_altar_unbound() -> TestResult {
    run("l6-floor-assembly-native-unbound")
}
#[test]
fn l6_native_altar_ruby_exact_one() -> TestResult {
    run("l6-floor-assembly-native-ruby-stack")
}
#[test]
fn l6_native_altar_ruby_unbound() -> TestResult {
    run("l6-floor-assembly-native-ruby-unbound")
}

pub(super) fn world(native: &NativeGameplayState) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene};
    native_scene(
        native,
        vec![NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z14-x129-y127.b3"
            )),
            region: (14, 129, 127),
            bounds: (33162, 32694, 33164, 32696),
            start: (33163, 32694),
            expected: vec![
                ExpectedTile {
                    position: (33162, 32694),
                    ids: vec![417],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (33163, 32694),
                    ids: vec![423],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (33162, 32695),
                    ids: vec![417, 2190],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (33163, 32695),
                    ids: vec![417],
                    counted_entry: None,
                },
            ],
        }],
        &[],
    )
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    fence: CurrentCharacterGameplayFence,
    name: &str,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&harness.pool)
        .await?;
    eprintln!("POSTGRES_DATABASE_READY l6_native_altar version={version}");
    let altar = if name.ends_with("-unbound") {
        CONTROL
    } else {
        ALTAR
    };
    super::l6_floor_assembly::exercise_at(owner, admitted, harness, fence, name, altar).await
}
