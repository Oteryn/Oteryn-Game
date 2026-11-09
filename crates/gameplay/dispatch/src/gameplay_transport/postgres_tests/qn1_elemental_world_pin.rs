//! Exercise the real ambient owner against the retained Elemental Spheres fire map.
use super::{ComposedFreshAdmission, TestResult};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[path = "qn1_elemental_world_pin/arrival.rs"]
mod arrival;
#[path = "qn1_elemental_world_pin/energy.rs"]
mod energy;
#[path = "qn1_elemental_world_pin/handoff.rs"]
mod handoff;
#[path = "qn1_elemental_world_pin/reset.rs"]
mod reset;
#[path = "qn1_elemental_world_pin/soil.rs"]
mod soil;

#[test]
fn qn1_elemental_world_pass_accepts_native_bundle_pin() -> TestResult {
    run("qn1-elemental-world-pin")
}

#[test]
fn qn1_elemental_live_drop_consumes_corpse_and_projects_native_boss() -> TestResult {
    run("qn1-elemental-arrival")
}

#[test]
fn qn1_elemental_soil_use_consumes_one_replays_and_recovers_decay() -> TestResult {
    run("qn1-elemental-soil")
}

#[test]
fn qn1_elemental_energy_soil_creates_field_without_replacing_altar() -> TestResult {
    run("qn1-elemental-energy-soil")
}

#[test]
fn qn1_elemental_energy_field_retires_once_through_world_reset() -> TestResult {
    run("qn1-elemental-energy-reset")
}

fn run(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            super::quest_world_callbacks::scenario(
                name,
                if name.starts_with("qn1-elemental-energy-") {
                    Position {
                        x: 33094,
                        y: 32189,
                        floor: -13,
                    }
                } else {
                    Position {
                        x: 33199,
                        y: 32103,
                        floor: -13,
                    }
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
    energy: bool,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene_with_appearances};
    native_scene_with_appearances(
        native,
        vec![NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z13-x129-y125.b3"
            )),
            region: (13, 129, 125),
            bounds: if energy {
                (33090, 32187, 33101, 32199)
            } else {
                (33195, 32101, 33205, 32111)
            },
            start: if energy {
                (33093, 32189)
            } else {
                (33198, 32103)
            },
            expected: vec![ExpectedTile {
                position: if energy {
                    (33095, 32194)
                } else {
                    (33199, 32103)
                },
                ids: vec![if energy { 7738 } else { 7739 }],
                counted_entry: None,
            }],
        }],
        &[],
        &[7745],
    )
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &super::fixture::Harness,
    name: &str,
    actor: crate::foundation::ExactActorRef,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
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
    if name.starts_with("qn1-elemental-energy-") {
        drop(runtime);
        return energy::exercise(
            owner,
            harness,
            actor,
            fence,
            admitted,
            name.ends_with("-reset"),
        )
        .await;
    }
    if name == "qn1-elemental-soil" {
        drop(runtime);
        return soil::exercise(owner, harness, actor, fence).await;
    }
    if name == "qn1-elemental-arrival" {
        drop(runtime);
        return arrival::exercise(owner, harness, actor, fence).await;
    }
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
