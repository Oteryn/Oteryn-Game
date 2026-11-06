//! Source zones.lua filters map-valid tiles before selecting a uniform position.
//! Unknown cells and the five physical blockers below are independent refusals.
use super::*;

fn sparse_room(owner: &mut Owner) -> MovementLocalPosition {
    let valid = MovementLocalPosition {
        x: 32375,
        y: 31145,
        floor: 8,
    };
    let clear = SpawnTile {
        ground: true,
        solid: false,
        projectile: false,
        immovable_solid: false,
        immovable_path: false,
    };
    owner.scene.known_cells = Some(
        [
            ((valid.x, valid.y, valid.floor), clear),
            (
                (32374, 31145, 8),
                SpawnTile {
                    ground: false,
                    ..clear
                },
            ),
            (
                (32376, 31145, 8),
                SpawnTile {
                    solid: true,
                    ..clear
                },
            ),
            (
                (32375, 31144, 8),
                SpawnTile {
                    projectile: true,
                    ..clear
                },
            ),
            (
                (32375, 31146, 8),
                SpawnTile {
                    immovable_solid: true,
                    ..clear
                },
            ),
            (
                (32375, 31147, 8),
                SpawnTile {
                    immovable_path: true,
                    ..clear
                },
            ),
        ]
        .into_iter()
        .collect(),
    );
    valid
}

#[test]
fn sparse_source_zone_selects_from_qualified_cells_before_uniform_draw() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(debug)?
        .block_on(async {
            let mut owner = Owner::fixture_with_capacity(false, 7)?;
            let valid = sparse_room(&mut owner);
            assert!(owner.scene.permits_spawn(valid));
            for (x, y) in [
                (32374, 31145),
                (32376, 31145),
                (32375, 31144),
                (32375, 31146),
                (32375, 31147),
                (32362, 31132),
            ] {
                assert!(
                    !owner
                        .scene
                        .permits_spawn(MovementLocalPosition { x, y, floor: 8 })
                );
            }
            drive_soulpit_deadlines(&mut owner, 4_250_000)
                .await
                .map_err(debug)?;
            let actors = owner
                .carrier
                .soulpit_actors(&owner.continuity, owner.binding.handoff_id)
                .map_err(debug)?;
            assert_eq!(
                actors.len(),
                1,
                "the source selects its one qualified cell, then native occupancy skips duplicates"
            );
            assert_eq!(actors[0].position, valid);
            assert_eq!(owner.stored.encounter.snapshot().monsters_alive, 1);
            Ok(())
        })
}

#[test]
fn uncertain_spawn_ack_reuses_physical_selection_after_all_map_cells_become_blocked() -> TestResult
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(debug)?
        .block_on(async {
            let mut owner = Owner::fixture_with_capacity(false, 7)?;
            sparse_room(&mut owner);
            owner.fail_spawn_ack = true;
            assert!(matches!(
                drive_soulpit_deadlines(&mut owner, 4_250_000).await,
                Err(SoulpitRuntimeError::AdapterUnavailable)
            ));
            let before = owner
                .carrier
                .soulpit_actors(&owner.continuity, owner.binding.handoff_id)
                .map_err(debug)?;
            assert_eq!(before.len(), 1);
            for tile in owner
                .scene
                .known_cells
                .as_mut()
                .ok_or("qualified room")?
                .values_mut()
            {
                tile.ground = false;
            }
            drive_soulpit_deadlines(&mut owner, 4_300_000)
                .await
                .map_err(debug)?;
            let after = owner
                .carrier
                .soulpit_actors(&owner.continuity, owner.binding.handoff_id)
                .map_err(debug)?;
            assert_eq!(
                before.iter().map(|actor| actor.actor).collect::<Vec<_>>(),
                after.iter().map(|actor| actor.actor).collect::<Vec<_>>()
            );
            assert_eq!(
                after.len(),
                1,
                "an uncertain ack cannot allocate another physical wave"
            );
            assert_eq!(owner.registrations.len(), 2);
            assert_eq!(owner.registrations[0], owner.registrations[1]);
            assert_eq!(owner.stored.encounter.snapshot().monsters_alive, 1);
            Ok(())
        })
}
