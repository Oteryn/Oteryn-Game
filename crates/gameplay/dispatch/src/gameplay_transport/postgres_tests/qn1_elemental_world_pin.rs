//! Exercise the real ambient owner against the retained Elemental Spheres fire map.
use super::{ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn qn1_elemental_world_pass_accepts_native_bundle_pin() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            super::quest_world_callbacks::scenario(
                "qn1-elemental-world-pin",
                Position {
                    x: 33199,
                    y: 32103,
                    floor: -13,
                },
                false,
            )
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "elemental World pass scenario panicked")??;
    assert!(started.elapsed().as_secs_f64() > 0.01);
    eprintln!(
        "QN1_ELEMENTAL_WORLD_PIN_PG_EXECUTED duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene};
    native_scene(
        native,
        vec![NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z13-x129-y125.b3"
            )),
            region: (13, 129, 125),
            bounds: (33195, 32101, 33205, 32111),
            start: (33198, 32103),
            expected: vec![ExpectedTile {
                position: (33199, 32103),
                ids: vec![7739],
                counted_entry: None,
            }],
        }],
        &[],
    )
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &super::fixture::Harness,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&harness.pool)
        .await?;
    eprintln!("QN1_ELEMENTAL_WORLD_PIN_PG_READY PostgreSQL {version}");
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native generation")?;
    assert!(
        native
            .item_policy("oteryn:item.tibia.i8141", "definition-r1")
            .is_some()
    );
    let map = owner.bundle_map().ok_or("fire-area map")?;
    let pin = owner.runtime.lock().await.content_pin().clone();
    use sha2::{Digest, Sha256};
    let digest: [u8; 32] =
        Sha256::digest(crate::map::overlay::map_revision(map.overlay.base()).as_bytes()).into();
    assert_eq!(pin.map_revision_digest(), digest);
    assert_ne!(pin.map_revision_digest(), map.overlay.base().digest());
    assert!(
        owner
            .boss_spawn_pass(&mut Default::default())
            .await
            .map_err(|_| "elemental World pass refused native pin")?
    );
    Ok(())
}
