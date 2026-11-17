//! Native entry restore uses qualified non-House evidence without a map capability.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_last_position::{
    LastPositionCell, LastPositionKey, LastPositionRecord, StoredLastPosition,
};
use crate::foundation::{ChannelContentPin, MovementLocalPosition, WorldId};
use crate::gameplay_transport::last_position::AdmissionPosition;

#[test]
fn native_entry_owner_restores_safe_saved_cell_and_keeps_world_and_frame_guards() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "native saved restore scenario panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "native_entry_saved_restore", true, false)
                .await?;
        prepare_current_session(&harness).await?;
        {
            let seal = harness.recovery.seal_current().map_err(debug)?;
            let authority = harness
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let world = WorldId::decode(&id(42)).map_err(debug)?;
            for mismatch in [None, Some("world"), Some("frame")] {
                let room = crate::content::qualify_native_entry_room(world).map_err(debug)?;
                let pin = ChannelContentPin::from_activation(
                    world,
                    1,
                    room.compiled().server_digest(),
                    room.compiled().client_digest(),
                    room.frame_binding().digest(),
                    room.map_revision_digest(),
                    (0, 0, 0),
                );
                let deps = Dependencies::with_content(
                    pin,
                    room,
                    crate::spell::cast::v1_spell_book().map_err(debug)?,
                )?;
                let owner = deps.owner(&harness, &authority);
                // Supply the immutable stored result at the exact production placement
                // boundary. The node-boot case separately proves logout persistence/readback.
                let mut record = LastPositionRecord {
                    world_id: world,
                    cell: LastPositionCell {
                        x: 1,
                        y: 0,
                        floor: 0,
                    },
                    coordinate_frame_digest: deps.room.frame_binding().digest(),
                    map_revision_digest: deps.room.map_revision_digest(),
                    server_artifact_digest: deps.room.compiled().server_digest(),
                    vitals: None,
                    rest: None,
                };
                match mismatch {
                    Some("world") => record.world_id = WorldId::decode(&id(99)).map_err(debug)?,
                    Some("frame") => record.coordinate_frame_digest[0] ^= 1,
                    _ => {}
                }
                let decided = AdmissionPosition::Stored(StoredLastPosition {
                    record,
                    key: LastPositionKey {
                        character_lease_generation: 1,
                        write_sequence: 1,
                    },
                });
                let mut runtime = deps.runtime.lock().await;
                owner
                    .initialize_admitted_position(
                        &mut runtime,
                        deps.actor,
                        deps.admitted.game_session_id,
                        &decided,
                    )
                    .await
                    .map_err(debug)?;
                let observed = runtime
                    .read_actor_position(deps.actor)
                    .map_err(debug)?
                    .position();
                let expected = MovementLocalPosition {
                    x: if mismatch.is_none() { 1 } else { 0 },
                    y: 0,
                    floor: 0,
                };
                assert_eq!(observed, expected, "saved placement with {mismatch:?}");
                eprintln!(
                    "NATIVE_ENTRY_SAVED_RESTORE_CASE mismatch={mismatch:?} observed={observed:?} expected={expected:?}"
                );
            }
        }
        harness.cleanup().await?;
        eprintln!("NATIVE_ENTRY_SAVED_RESTORE_COMPLETED");
        Ok(())
    })
}
