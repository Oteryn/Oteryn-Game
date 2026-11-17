//! Authored native altar counterpart: real drops through the existing floor crafting owner.
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::content::native_gameplay::NativeGameplayState;
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

// Global spoiler curid 5073 oldid 1174796 puts the altar at the north table.
const ALTAR: Position = Position {
    x: 33198,
    y: 32876,
    floor: -11,
};

pub(super) fn start() -> Position {
    Position {
        y: ALTAR.y + 1,
        ..ALTAR
    }
}

pub(super) fn run(name: &'static str) -> TestResult {
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
                "/../../../content/world/placements/region-z11-x129-y128.b3"
            )),
            region: (11, 129, 128),
            bounds: (ALTAR.x, ALTAR.y, ALTAR.x + 1, ALTAR.y + 2),
            start: (start().x, start().y),
            expected: vec![
                ExpectedTile {
                    position: (ALTAR.x, ALTAR.y),
                    ids: vec![419, 2344],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (start().x, start().y),
                    ids: vec![10145],
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
        start()
    } else {
        ALTAR
    };
    super::l6_floor_assembly::exercise_at(owner, admitted, harness, fence, name, altar).await
}
