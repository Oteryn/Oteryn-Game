//! Exercise the real ambient owner against the retained Elemental Spheres fire map.
use super::{ComposedFreshAdmission, TestResult};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
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
    // This metadata is a projector fixture, not a durable lease. The real Root
    // claim/retirement cases separately prove the clock and receipt boundary.
    let mut runtime = owner.runtime.lock().await;
    // Channel boot installs these same artifact-qualified creature policies.
    native
        .install_companion_policies(&mut runtime)
        .map_err(super::fixture::debug)?;
    let binding = runtime.binding();
    let recipe = &oteryn_gameplay_quests::quest::elemental_arrival::RECIPES[1];
    let clock = crate::durability::item_loop_move::ElementalSummonClock {
        intent: crate::durability::item_loop_move::ElementalSummonIntent {
            transaction_id: [9; 16],
            altar: recipe.altar,
            creature_key: recipe.creature.into(),
            position: recipe.summon,
        },
        spawn_key: format!("qn1.elemental:{}", "09".repeat(16)),
        cycle: 0,
        owner_generation: binding.scope_generation(),
    };
    let cells = owner.movement_cells();
    crate::gameplay_transport::world_respawns::elemental::project(
        &mut runtime,
        binding,
        &pin,
        &[clock],
        |at| {
            cells.index().lookup(
                cells.scope(),
                crate::content::LogicalCell {
                    x: at.x,
                    y: at.y,
                    z: i32::from(at.floor),
                },
            ) == Ok(crate::content::CollisionClass::Walkable)
        },
    )
    .map_err(super::fixture::debug)?;
    let spawn = runtime
        .boss_spawns()
        .first()
        .ok_or("fire overlord must project on the native floor")?;
    assert_eq!(spawn.position.floor, 13);
    let actor = spawn.actor.ok_or("fire overlord native actor")?;
    assert!(runtime.contains_live_creature(actor));
    Ok(())
}
