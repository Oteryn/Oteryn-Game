// Real command decoding/routing and production rotation executor; no database writes.
use super::*;
use crate::content::native_gameplay::NativeGameplayState;
use crate::map::boot::{BundleCollisionIndexExt, BundleMap};
use crate::map::overlay::TilePos;
use crate::map::view::{MapFacts, placement_key};
use oteryn_protocol_oteryn::world_map::MapDefinition;
use oteryn_protocol_oteryn::world_object::WorldObjectTarget;

struct Furniture<'a> {
    map: &'a BundleMap,
    native: &'a NativeGameplayState,
    actor: Cell<TilePos>,
}

impl FreshAdmissionAuthority for Furniture<'_> {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Unavailable)
    }

    async fn observe(&self, _: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(at(0))
    }

    async fn use_object(
        &self,
        _: ExactActorRef,
        _: UseCommand,
        target: WorldObjectTarget,
    ) -> UseOutcome {
        UseOutcome {
            disposition: crate::gameplay_transport::item_transform::rotate(
                self.map,
                self.native,
                [1; 32],
                self.actor.get(),
                &target,
            )
            .unwrap_or(UseDisposition::NothingToUse),
            ..UseOutcome::rejected()
        }
    }
}

#[test]
fn item_transform_ground_rotation_real_dispatch_cycle_stale_reach_and_bindings()
-> Result<(), Box<dyn Error>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            run(async {
                let world_id = WorldId::decode(&WORLD)?;
                let channel_id = ChannelId::decode(&CHANNEL)?;
                let (controller, _) =
                    crate::content::native_gameplay::test_support::activated_with_loot_tables(
                        world_id, None,
                    );
                let native = controller
                    .active()
                    .ok_or("active")?
                    .native_gameplay()
                    .ok_or("native")?;
                let index = native.item_index().ok_or("index")?;
                let chair = "oteryn:item.tibia.i2358";
                let reference = |key: &str| {
                    index
                        .definition_ref("Item", key, "definition-r1")
                        .ok_or("reference")
                };
                let (bytes, load) = crate::map::boot::tests::bundle_with_item_ids(
                    (chair, reference(chair)?.get() - 1),
                    (
                        "oteryn:item.tibia.i2359",
                        reference("oteryn:item.tibia.i2359")?.get() - 1,
                    ),
                );
                let boot = |bytes: &[u8], load: crate::map::BundlePins| {
                    let base = crate::map::load(bytes, &load).map_err(|e| format!("{e:?}"))?;
                    crate::map::boot::boot(
                        bytes,
                        &crate::map::boot::BootPins {
                            map_revision: crate::map::overlay::map_revision(&base),
                            bundle: load,
                            start: TilePos {
                                x: 6,
                                y: 0,
                                floor: -7,
                            },
                        },
                        world_id,
                        channel_id,
                        |key| {
                            let policy = native.item_policy(key, "definition-r1")?.record();
                            Some(crate::map::facts::ItemDefinition {
                                reference: reference(key).ok()?,
                                solid: policy.attributes.blocks_movement,
                                blocks_projectile: policy.attributes.blocks_projectile?,
                                pickupable: false,
                                door: false,
                            })
                        },
                    )
                    .map_err(|e| format!("{e:?}"))
                };
                let world = boot(&bytes, load.clone())?;
                let room = crate::content::qualify_native_entry_room(world_id)
                    .map_err(|e| format!("{e:?}"))?;
                let cells = world
                    .movement_cells(room.movement_cells())
                    .map_err(|e| format!("{e:?}"))?;
                let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(
                    collision,
                ) = cells.index()
                else {
                    return Err("bundle".into());
                };
                let map = collision.map();
                let pos = TilePos {
                    x: 6,
                    y: 0,
                    floor: -7,
                };
                let key = placement_key(pos, 1).ok_or("key")?;
                let before = map.facts.pinned();
                let mut placement = map.overlay.base().digest().to_vec();
                placement.extend_from_slice(&key.to_be_bytes());
                let authority = Furniture {
                    map,
                    native,
                    actor: Cell::new(TilePos { x: 7, ..pos }),
                };
                let use_frame =
                    |revision, placement: &[u8]| use_command(1, 1, 2, placement, revision);
                // Every request traverses serve_admitted -> command registry -> USE decoder ->
                // use_object -> the same Content/BundleFacts executor as ComposedFreshAdmission.
                let execute = async |revision,
                                     placement: &[u8],
                                     expected|
                       -> Result<(), Box<dyn Error>> {
                    let (_, frames) =
                        drive_session(&authority, positioned()?, &[use_frame(revision, placement)])
                            .await?;
                    let result = frames
                        .iter()
                        .find_map(|frame| {
                            let envelope = crate::foundation::decode_wire_envelope(frame).ok()?;
                            (envelope.message_type()
                                == crate::foundation::MessageType::CommandResult)
                                .then(|| {
                                    oteryn_protocol_oteryn::decode_command_result(
                                        envelope.payload(),
                                    )
                                    .ok()
                                })
                                .flatten()
                                .and_then(|result| {
                                    oteryn_protocol_oteryn::world_object::decode_use_result(
                                        result.payload,
                                    )
                                    .ok()
                                })
                        })
                        .ok_or("USE result")?;
                    assert_eq!(result, expected);
                    Ok(())
                };
                execute(0, &placement, UseDisposition::Committed).await?;
                assert_eq!(map.facts.object_revision(key), 1);
                let current = map
                    .facts
                    .base_entry(pos, 1, reference(chair)?.get() - 1)
                    .ok_or("chair")?;
                assert_eq!(
                    current.definition,
                    MapDefinition::Item(reference("oteryn:item.tibia.i2359")?)
                );
                assert_eq!(current.appearance_id, 2359);
                // Domain 17 carries the replacement definition, appearance and revision.
                let mut view = crate::gameplay_transport::world_map::SessionMapView::default();
                let mut items = crate::gameplay_transport::item_view::SessionItemView::resume(
                    crate::gameplay_transport::item_view::ItemViewContinuity::default(),
                )
                .with_map_view();
                let facts = map.facts.pinned();
                let source = crate::gameplay_transport::world_map::MapViewSource {
                    overlay: &map.overlay,
                    facts: &facts,
                    content_generation: [1; 32],
                    reset_epoch: 0,
                };
                let snapshot = view
                    .snapshot(
                        &mut items,
                        &source,
                        crate::gameplay_transport::world_spatial::ActorPosition {
                            x: 6,
                            y: 0,
                            floor: -7,
                        },
                    )
                    .map_err(|e| format!("{e:?}"))?;
                let snapshot =
                    oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(&snapshot.payload)
                        .map_err(|e| format!("{e:?}"))?;
                let wire_chair = snapshot
                    .tiles
                    .iter()
                    .find(|tile| {
                        tile.position.x == 6 && tile.position.y == 0 && tile.position.floor == -7
                    })
                    .and_then(|tile| tile.items.get(1))
                    .ok_or("wire chair")?;
                assert_eq!(wire_chair.definition, current.definition);
                assert_eq!(wire_chair.appearance_id, 2359);
                assert_eq!(
                    wire_chair.origin,
                    oteryn_protocol_oteryn::world_map::MapOrigin::BaseOrdinal {
                        ordinal: 1,
                        object_revision: 1
                    }
                );
                let restarted = boot(&bytes, load.clone())?;
                assert_eq!(restarted.facts().object_revision(key), 0);
                // An already-pinned observer retains a matching old definition/revision pair.
                assert_eq!(before.object_revision(key), 0);
                assert_eq!(
                    before
                        .base_entry(pos, 1, reference(chair)?.get() - 1)
                        .ok_or("old")?
                        .appearance_id,
                    2358
                );
                execute(0, &placement, UseDisposition::StaleState).await?;
                execute(2, &placement, UseDisposition::StaleState).await?;
                authority.actor.set(TilePos { x: 9, ..pos });
                execute(1, &placement, UseDisposition::TooFar).await?;
                authority.actor.set(TilePos { floor: -6, ..pos });
                execute(1, &placement, UseDisposition::TooFar).await?;
                authority.actor.set(pos);
                let mut foreign = placement.clone();
                foreign[0] ^= 1;
                execute(1, &foreign, UseDisposition::StaleState).await?;
                for revision in 1..4 {
                    execute(revision, &placement, UseDisposition::Committed).await?;
                }
                assert_eq!(map.facts.object_revision(key), 4);
                assert_eq!(
                    map.facts
                        .base_entry(pos, 1, reference(chair)?.get() - 1)
                        .ok_or("cycle")?
                        .appearance_id,
                    2358
                );
                // House custody and source bindings are refused through the same dispatch too.
                for mode in 0..3 {
                    let mut bundle =
                        oteryn_world_bundle::bundle::read(&bytes).map_err(|e| format!("{e:?}"))?;
                    let tile = bundle.sectors[0]
                        .tiles
                        .iter_mut()
                        .find(|t| t.x == 6)
                        .ok_or("tile")?;
                    match mode {
                        0 => tile.house = 1,
                        1 => tile.items[1].attrs.unique = Some(100),
                        _ => tile.items[1].attrs.action = Some(100),
                    }
                    let modified = oteryn_world_bundle::writer::write(
                        &bundle.manifest,
                        &bundle.sectors,
                        &bundle.spawns,
                    )
                    .map_err(|e| format!("{e:?}"))?;
                    let mut pins = load.clone();
                    pins.digest = oteryn_world_bundle::bundle::read(&modified)
                        .map_err(|e| format!("{e:?}"))?
                        .digest;
                    let world = boot(&modified, pins)?;
                    let cells = world
                        .movement_cells(room.movement_cells())
                        .map_err(|e| format!("{e:?}"))?;
                    let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(
                        collision,
                    ) = cells.index()
                    else {
                        return Err("bundle".into());
                    };
                    let map = collision.map();
                    let authority = Furniture {
                        map,
                        native,
                        actor: Cell::new(pos),
                    };
                    let mut placement = map.overlay.base().digest().to_vec();
                    placement.extend_from_slice(&key.to_be_bytes());
                    let (_, frames) =
                        drive_session(&authority, positioned()?, &[use_frame(0, &placement)])
                            .await?;
                    let payload = encode_use_result(UseDisposition::NothingToUse)
                        .map_err(|e| format!("{e:?}"))?;
                    assert!((0..64).any(|sequence| {
                        encode_command_result(1, 1, sequence, CommandStatus::Accepted, &payload)
                            .is_ok_and(|result| frames.contains(&result))
                    }));
                    assert_eq!(map.facts.object_revision(key), 0);
                }
                Ok(())
            })
            .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "rotation test panic")?
        .map_err(Into::into)
}
