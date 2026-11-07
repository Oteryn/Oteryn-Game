//! Real composed caster map producer: full display stacks, no viewer actors.
use super::{
    Dependencies, TestResult,
    fixture::{Harness, debug, run},
    prepare_current_session,
};
use crate::{
    foundation::MovementLocalPosition,
    map::boot::tests::{DOOR_AT, door_world},
};
use oteryn_protocol_oteryn::cast::{Envelope, Message, PublicMedia};
#[test]
fn s17_viewer_map_uses_casters_committed_map_without_changing_actor() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            run(async |admin| {
                let started = std::time::Instant::now();
                let h =
                    Harness::create_with_admission_seed(admin, "s17_view_map", true, false).await?;
                prepare_current_session(&h).await?;
                let result = async {
                    let seal = h.recovery.seal_current().map_err(debug)?;
                    let authority = h
                        .root
                        .open_character_authority(&seal)
                        .await
                        .map_err(debug)?;
                    let world_id = crate::foundation::WorldId::decode(&super::fixture::id(42))
                        .map_err(debug)?;
                    let world = door_world()?;
                    let (controller, activated) =
                        super::death_ml_loss_composed::production_generation_with_pin(
                            world_id,
                            "content-1",
                        )?;
                    let generation = controller.active().ok_or("active generation")?;
                    let pin = crate::foundation::ChannelContentPin::from_activation(
                        world_id,
                        1,
                        generation.identity().server_artifact_digest(),
                        generation.identity().client_artifact_digest(),
                        activated.frame_binding().digest(),
                        world.map_revision_digest(),
                        (
                            i32::from(DOOR_AT.x) - 1,
                            i32::from(DOOR_AT.y),
                            -i16::from(DOOR_AT.floor),
                        ),
                    );
                    let deps = Dependencies::with_content(
                        pin,
                        crate::content::qualify_native_entry_room(world_id).map_err(debug)?,
                        crate::spell::cast::v1_spell_book().map_err(debug)?,
                    )?;
                    let cells = world
                        .movement_cells(deps.room.movement_cells())
                        .map_err(debug)?;
                    let mut owner = deps.owner(&h, &authority);
                    owner.movement_cells = (&cells).into();
                    owner.active_generation = Some(generation);
                    let position = MovementLocalPosition {
                        x: i32::from(DOOR_AT.x) - 1,
                        y: i32::from(DOOR_AT.y),
                        floor: -i16::from(DOOR_AT.floor),
                    };
                    owner
                        .runtime
                        .lock()
                        .await
                        .initialize_pinned_test_position(deps.actor, position)
                        .map_err(debug)?;
                    let (camera, media) = owner
                        .cast_map(deps.admitted)
                        .await
                        .ok_or("production caster map")?;
                    assert!(
                        camera.matches_runtime(
                            oteryn_protocol_oteryn::world_spatial::ActorPosition {
                                x: position.x,
                                y: position.y,
                                floor: position.floor
                            }
                        ),
                        "ordinary positive runtime floor matches the native caster map"
                    );
                    assert!(!media.is_empty());
                    let mut found = false;
                    assert!(matches!(media.first(),Some(PublicMedia::Camera {floor:-7})));
                    for (i, m) in media.iter().skip(1).enumerate() {
                        let PublicMedia::Map { reset, last, tiles } = m else {
                            return Err("unexpected map media".into());
                        };
                        assert_eq!(*reset, i == 0);
                        assert_eq!(*last, i + 2 == media.len());
                        assert!(tiles.len() <= 32);
                        found |= tiles.iter().any(|t| t.appearances.contains(&1629));
                        let envelope = Envelope {
                            generation: 1,
                            message: Message::Media {
                                epoch: 1,
                                sequence: i as u64 + 1,
                                media: m.clone(),
                            },
                        };
                        let bytes = envelope.encode().map_err(debug)?;
                        assert_eq!(Envelope::decode(&bytes, false).map_err(debug)?, envelope);
                        assert!(Envelope::decode(&bytes, true).is_err());
                    }
                    assert!(found, "door appearance is projected from actual map stacks");
                    let after = owner
                        .runtime
                        .lock()
                        .await
                        .borrow_movement_position()
                        .read(deps.actor)
                        .map_err(debug)?
                        .position();
                    assert_eq!(after, position, "viewer projection cannot move caster");
                    eprintln!(
                        "S17 executed real PostgreSQL composed viewer map case: {:.3}s",
                        started.elapsed().as_secs_f64()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                }
                .await;
                h.cleanup().await?;
                result
            })
            .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "S17 map thread panicked")?
        .map_err(Into::into)
}
