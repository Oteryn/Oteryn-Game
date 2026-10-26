#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use crate::foundation::{
    ChannelContentPin, ChannelId, GameSessionId, MovementLocalPosition, NodeId, WorldId,
};
use crate::gameplay_transport::connection::{
    AdmissionRefusal, AdmittedSession, FirstEntryOutcome, FreshAdmissionAttempt, SessionContinuity,
};
use crate::gameplay_transport::item_view::InventoryItems;
use crate::gameplay_transport::world_spatial::{ActorPosition, WorldSpatialObservation};
use std::cell::RefCell;
use std::num::{NonZeroU32, NonZeroU64};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

type TestResult = Result<(), Box<dyn std::error::Error>>;
fn id(n: u8) -> [u8; 16] {
    [1, 0, 0, 0, 0, n, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, n]
}
fn index() -> crate::content::item_ref::ItemDefinitionIndex {
    crate::content::item_ref::ItemDefinitionIndex::decode(br#"{"schema":"OTERYN_NATIVE_ITEM_KEYS/v1","records":[["oteryn:item.tibia.i2854","definition-r1"],["oteryn:item.tibia.i3003","definition-r1"],["oteryn:item.tibia.i646","definition-r1"]]}"#).unwrap()
}
fn inventory() -> crate::durability::item_use::ItemUseInventory {
    use crate::durability::item_transfer::{BackpackEntry, CharacterBackpack, InventoryItem};
    let item = |n, key: &str| InventoryItem {
        item_instance_id: id(n),
        quantity: 1,
        definition: TypedDefinitionRef {
            family: "Item".into(),
            production_key: key.into(),
            revision_ref: "definition-r1".into(),
        },
    };
    crate::durability::item_use::ItemUseInventory {
        content_revision: "content-r1".into(),
        ruleset_revision: "ruleset-r1".into(),
        equipment: vec![],
        nested: vec![],
        backpack: Some(CharacterBackpack {
            backpack: item(5, "oteryn:item.tibia.i2854"),
            entries: vec![BackpackEntry {
                item: item(6, "oteryn:item.tibia.i3003"),
                placement_ordinal: 1,
            }],
        }),
    }
}

struct Owner {
    runtime: RefCell<ChannelRuntimeV1>,
    states: RefCell<ChannelSpellStates>,
    cells: NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    inventory: RefCell<crate::durability::item_use::ItemUseInventory>,
}
impl Owner {
    fn new() -> Self {
        use oteryn_world_bundle::sector::{Attrs, Item, Tile};
        use oteryn_world_bundle::writer::{
            self as writer, BuildClass, Extent, Family, Identity, Manifest, PaletteEntry, Sector,
            Terrain, TerrainKind,
        };
        let manifest = Manifest {
            subareas: Default::default(),
            format: writer::FORMAT.into(),
            min_reader_version: writer::VERSION,
            projection_class: "server".into(),
            compiler_version: "engineering".into(),
            build_class: BuildClass::NonProduction,
            identity: Identity {
                project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
                world_schema_version: "world-schema-1".into(),
                content_revision: "rev-1".into(),
                ..Identity::default()
            },
            world: Extent {
                min_x: 0,
                min_y: 0,
                max_x: 32,
                max_y: 32,
                floors: vec![-8, -7],
            },
            palette: [100, 386]
                .into_iter()
                .enumerate()
                .map(|(n, id)| PaletteEntry {
                    key: format!("oteryn:terrain.tibia.i{id}"),
                    family: Family::Terrain,
                    id: n as u32,
                    terrain: Some(Terrain {
                        kind: TerrainKind::Ground,
                        walkable: Some(true),
                        ground_speed: Some(150),
                    }),
                })
                .collect(),
            draft_areas: vec![],
            skipped_provisional_keys: vec![],
            dropped_teleports: vec![],
            spawns: Default::default(),
            npcs: Default::default(),
        };
        let tile = |x, y, palette| Tile {
            x,
            y,
            flags: 0,
            house: 0,
            zones: vec![],
            items: vec![Item {
                palette,
                depth: 0,
                attrs: Attrs::default(),
            }],
        };
        let mut sectors = [
            Sector {
                floor: -8,
                sx: 0,
                sy: 0,
                tiles: vec![
                    tile(10, 10, 1),
                    tile(11, 10, 0),
                    tile(12, 10, 0),
                    tile(13, 10, 0),
                ],
            },
            Sector {
                floor: -7,
                sx: 0,
                sy: 0,
                tiles: vec![
                    tile(10, 9, 0),
                    tile(9, 10, 0),
                    tile(10, 10, 0),
                    tile(11, 10, 0),
                    tile(10, 11, 0),
                ],
            },
        ];
        sectors[1]
            .tiles
            .iter_mut()
            .find(|tile| (tile.x, tile.y) == (10, 11))
            .unwrap()
            .flags = 1;
        let bytes = writer::write(&manifest, &sectors, &Default::default()).unwrap();
        let pins = crate::map::BundlePins {
            digest: writer::read(&bytes).unwrap().digest,
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "rev-1".into(),
            production: false,
        };
        let world = WorldId::decode(&id(1)).unwrap();
        let channel = ChannelId::decode(&id(2)).unwrap();
        let room = crate::content::qualify_native_entry_room(world).unwrap();
        let booted = crate::map::boot::boot(
            &bytes,
            &crate::map::boot::BootPins {
                map_revision: format!(
                    "sha256:{}",
                    pins.digest
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<String>()
                ),
                bundle: pins,
                start: crate::map::overlay::TilePos {
                    x: 10,
                    y: 10,
                    floor: -8,
                },
            },
            world,
            channel,
            |_| None,
        )
        .unwrap();
        let cells = booted.movement_cells(room.movement_cells()).unwrap();
        let pin = ChannelContentPin::from_activation(
            world,
            1,
            room.compiled().server_digest(),
            room.compiled().client_digest(),
            booted.frame_binding_digest(world),
            booted.map_revision_digest(),
            (10, 10, 8),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            channel,
            NodeId::decode(&id(3)).unwrap(),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            pin,
        )
        .unwrap();
        let session = GameSessionId::decode(&id(4)).unwrap();
        let reservation = runtime.reserve_fresh_session(session).unwrap();
        let actor = runtime.commit_fresh_session(reservation).unwrap();
        runtime.initialize_first_entry_position(actor).unwrap();
        let mut states = ChannelSpellStates::default();
        states
            .initialize(
                &runtime,
                actor,
                session,
                crate::gameplay_transport::actor_spell::tests::FACTS,
                (0, 0),
                SemanticTimeMicros::from_micros(0),
            )
            .unwrap();
        Self {
            runtime: RefCell::new(runtime),
            states: RefCell::new(states),
            cells,
            actor,
            session,
            inventory: RefCell::new(inventory()),
        }
    }
    fn intent(&self, x: u16, revision: u64) -> UseIntent {
        let mut runtime = self.runtime.borrow_mut();
        let at =
            crate::gameplay_transport::bundle_map_position(&mut runtime, &self.cells, self.actor)
                .unwrap();
        let mut placement = at.map.overlay.base().digest().to_vec();
        placement.extend_from_slice(
            &oteryn_world_bundle::bundle::placement_key(-8, x, 10, 0)
                .unwrap()
                .to_be_bytes(),
        );
        UseIntent {
            target: UseTarget::ItemByDefinition(NonZeroU32::new(2).unwrap()),
            use_with: None,
            use_with_position: None,
            use_with_map_item: Some(MapUseTarget::Object(
                oteryn_protocol_oteryn::world_object::WorldObjectTarget {
                    placement,
                    expected_revision: revision,
                },
            )),
        }
    }
    fn position(&self) -> MovementLocalPosition {
        self.runtime
            .borrow()
            .read_actor_position(self.actor)
            .unwrap()
            .position()
    }
    fn apply(&self, intent: &UseIntent, target: Option<ItemKey>) -> UseDisposition {
        rope_in_channel(
            &mut self.runtime.borrow_mut(),
            &mut self.states.borrow_mut(),
            &self.cells,
            self.actor,
            UseCommand {
                game_session_id: self.session,
                command_id: 1,
                item_fence: None,
            },
            intent,
            target,
            0,
            SemanticTimeMicros::from_micros(100),
            None,
        )
    }
}
impl FreshAdmissionAuthority for Owner {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Unavailable)
    }
    async fn observe(&self, _: ExactActorRef) -> Option<WorldSpatialObservation> {
        let p = self.position();
        Some(WorldSpatialObservation {
            content_generation: self.runtime.borrow().content_pin().client_artifact_digest(),
            actor_position: ActorPosition {
                x: p.x,
                y: p.y,
                floor: -p.floor,
            },
        })
    }
    async fn observe_character_inventory(
        &self,
        _: ExactActorRef,
        _: GameSessionId,
    ) -> Option<InventoryItems> {
        crate::gameplay_transport::item_view::inventory_from_backpack(
            self.inventory.borrow().backpack.as_ref(),
            |d| index().definition_ref(&d.family, &d.production_key, &d.revision_ref),
        )
    }
    async fn use_craft(
        &self,
        _: ExactActorRef,
        _: UseCommand,
        intent: UseIntent,
        source: Option<ItemKey>,
        target: Option<ItemKey>,
    ) -> Option<UseOutcome> {
        let disposition =
            if carried_rope(&self.inventory.borrow(), &index(), &intent, source).is_some() {
                self.apply(&intent, target)
            } else {
                UseDisposition::NothingToUse
            };
        Some(UseOutcome {
            disposition,
            ..UseOutcome::rejected()
        })
    }
}

#[test]
fn tools_rope_spot_owner_reach_stale_nonspot_custody_and_fallback() {
    let owner = Owner::new();
    let intent = owner.intent(10, 0);
    assert_eq!(
        owner.apply(&owner.intent(10, 1), None),
        UseDisposition::StaleState
    );
    assert_eq!(
        owner.apply(&owner.intent(13, 0), None),
        UseDisposition::TooFar
    );
    assert_eq!(
        owner.apply(&owner.intent(11, 0), None),
        UseDisposition::NothingToUse
    );
    let mut inventory = inventory();
    assert!(carried_rope(&inventory, &index(), &intent, None).is_some());
    let mut handled = intent.clone();
    handled.target = UseTarget::Item(NonZeroU64::new(2).unwrap());
    assert!(
        carried_rope(
            &inventory,
            &index(),
            &handled,
            Some(ItemKey::Instance(id(6)))
        )
        .is_some()
    );
    assert!(
        carried_rope(
            &inventory,
            &index(),
            &handled,
            Some(ItemKey::Instance(id(7)))
        )
        .is_none()
    );
    let entry = inventory.backpack.as_mut().unwrap().entries.remove(0);
    inventory.equipment.push(entry.item.clone());
    assert!(carried_rope(&inventory, &index(), &intent, None).is_none());
    inventory.equipment.clear();
    inventory.nested.push((id(5), entry));
    assert!(
        carried_rope(&inventory, &index(), &intent, None).is_none(),
        "unreachable bag tree"
    );
    let mut bag = inventory.backpack.as_ref().unwrap().backpack.clone();
    bag.item_instance_id = id(7);
    inventory.backpack.as_mut().unwrap().entries.push(
        crate::durability::item_transfer::BackpackEntry {
            item: bag,
            placement_ordinal: 1,
        },
    );
    inventory.nested[0].0 = id(7);
    assert!(
        carried_rope(&inventory, &index(), &intent, None).is_some(),
        "closed carried bag"
    );
    let nested_rope = &mut inventory.nested[0].1.item.definition;
    nested_rope.production_key = "oteryn:item.tibia.i646".into();
    let mut alternate = intent.clone();
    alternate.target = UseTarget::ItemByDefinition(NonZeroU32::new(3).unwrap());
    assert!(carried_rope(&inventory, &index(), &alternate, None).is_some());
    inventory.nested[0].1.item.definition.revision_ref = "wrong-revision".into();
    assert!(carried_rope(&inventory, &index(), &alternate, None).is_none());
    assert_eq!(owner.apply(&intent, None), UseDisposition::Committed);
    assert_eq!(
        owner.position(),
        MovementLocalPosition {
            x: 10,
            y: 11,
            floor: 7
        }
    );
    let runtime = owner.runtime.borrow();
    let states = owner.states.borrow();
    assert_eq!(
        states
            .get(&runtime, owner.actor, owner.session)
            .unwrap()
            .cooldowns
            .group_ready_at(oteryn_gameplay_spell_rules::cast::STAIRHOP_COOLDOWN_KEY)
            .unwrap()
            .get(),
        100 + oteryn_gameplay_spell_rules::cast::STAIRHOP_MICROS
    );
}

#[test]
fn tools_rope_spot_wire_dispatch_moves_and_emits_spatial_delta() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            use crate::gameplay_transport::capabilities::{
                OfferedCapability, SelectedCapabilities,
            };
            for (selected_tool_capability, use_handle) in
                [(true, false), (true, true), (false, false)]
            {
                let owner = Owner::new();
                let offered = [
                    OfferedCapability {
                        id: 4,
                        requires: &[],
                    },
                    OfferedCapability {
                        id: 6,
                        requires: &[],
                    },
                    OfferedCapability {
                        id: 11,
                        requires: &[15],
                    },
                    OfferedCapability {
                        id: 15,
                        requires: &[4],
                    },
                ];
                let admitted = AdmittedSession {
                    game_session_id: owner.session,
                    world_id: WorldId::decode(&id(1))?,
                    channel_id: ChannelId::decode(&id(2))?,
                    runtime_actor: Some(owner.actor),
                    first_entry: FirstEntryOutcome::Positioned,
                    controller: None,
                    item_fence: None,
                    continuity: SessionContinuity {
                        selected_capabilities: SelectedCapabilities::select(
                            &offered,
                            if selected_tool_capability {
                                &[4, 11, 15]
                            } else {
                                &[4, 15]
                            },
                        )
                        .unwrap(),
                        ..SessionContinuity::FRESH
                    },
                };
                let mut intent = owner.intent(10, 0);
                if use_handle {
                    // Snapshot allocation: main backpack first, then its one rope entry.
                    intent.target = UseTarget::Item(NonZeroU64::new(2).unwrap());
                }
                let payload =
                    oteryn_protocol_oteryn::world_object::encode_item_use_intent(&intent).unwrap();
                let frame = oteryn_protocol_oteryn::encode_client_command(
                    1,
                    &oteryn_protocol_oteryn::ClientCommandValue {
                        command_id: 1,
                        command_type: 2,
                        payload: &payload,
                    },
                )?;
                let (mut server, mut client) = tokio::io::duplex(1 << 20);
                client.write_u32(frame.len() as u32).await?;
                client.write_all(&frame).await?;
                client.shutdown().await?;
                let end = crate::gameplay_transport::connection::serve_admitted(
                    &mut server,
                    admitted,
                    &owner,
                    crate::gameplay_transport::connection::IDLE_LIVENESS,
                )
                .await;
                assert!(
                    matches!(
                    end,
                    crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenDisconnected(
                        _
                    )
                ),
                    "{end:?}"
                );
                drop(server);
                let mut bytes = vec![];
                client.read_to_end(&mut bytes).await?;
                let mut cursor = 0;
                let mut committed = false;
                let mut spatial = false;
                while cursor < bytes.len() {
                    let n = u32::from_be_bytes(bytes[cursor..cursor + 4].try_into()?) as usize;
                    let frame = crate::foundation::decode_wire_envelope(
                        &bytes[cursor + 4..cursor + 4 + n],
                    )?;
                    if frame.message_type() == crate::foundation::MessageType::CommandResult {
                        let result =
                            oteryn_protocol_oteryn::decode_command_result(frame.payload())?;
                        assert_eq!(
                            oteryn_protocol_oteryn::world_object::decode_use_result(result.payload)
                                .unwrap(),
                            if selected_tool_capability {
                                UseDisposition::Committed
                            } else {
                                UseDisposition::Rejected
                            }
                        );
                        committed = true;
                    }
                    if frame.message_type() == crate::foundation::MessageType::StateDelta {
                        let delta = oteryn_protocol_oteryn::decode_state_delta(frame.payload())?;
                        if delta.domain_id == 1 {
                            assert!(committed, "result precedes relocation");
                            spatial = true;
                        }
                    }
                    cursor += n + 4;
                }
                assert!(committed);
                assert_eq!(spatial, selected_tool_capability);
                assert_eq!(
                    owner.position(),
                    if selected_tool_capability {
                        MovementLocalPosition {
                            x: 10,
                            y: 11,
                            floor: 7,
                        }
                    } else {
                        MovementLocalPosition {
                            x: 10,
                            y: 10,
                            floor: 8,
                        }
                    }
                );
            }
            Ok(())
        })
}

#[test]
fn tools_rope_spot_occupied_fallback_no_landing_and_stale_proof() {
    let owner = Owner::new();
    let mut runtime = owner.runtime.borrow_mut();
    let second_session = GameSessionId::decode(&id(9)).unwrap();
    let reservation = runtime.reserve_fresh_session(second_session).unwrap();
    let other = runtime.commit_fresh_session(reservation).unwrap();
    runtime
        .initialize_pinned_test_position(
            other,
            MovementLocalPosition {
                x: 10,
                y: 11,
                floor: 7,
            },
        )
        .unwrap();
    let expected = runtime.read_actor_position(owner.actor).unwrap();
    let spot = LogicalCell { x: 10, y: 10, z: 8 };
    let mut closed = std::collections::BTreeSet::from([
        LogicalCell { x: 10, y: 9, z: 7 },
        LogicalCell { x: 11, y: 10, z: 7 },
        LogicalCell { x: 9, y: 10, z: 7 },
    ]);
    assert!(matches!(
        crate::map::floor::prepare_bundle_rope(
            &runtime,
            &owner.cells,
            owner.actor,
            owner.session,
            expected,
            spot,
            false,
            &closed
        ),
        Err(crate::movement::MovementError::Blocked)
    ));
    assert_eq!(runtime.read_actor_position(owner.actor).unwrap(), expected);
    closed.clear();
    let proof = crate::map::floor::prepare_bundle_rope(
        &runtime,
        &owner.cells,
        owner.actor,
        owner.session,
        expected,
        spot,
        false,
        &closed,
    )
    .unwrap();
    runtime.commit_source_step(proof).unwrap();
    assert_eq!(
        runtime.read_actor_position(owner.actor).unwrap().position(),
        MovementLocalPosition {
            x: 10,
            y: 9,
            floor: 7
        }
    );
    assert!(
        crate::map::floor::prepare_bundle_rope(
            &runtime,
            &owner.cells,
            owner.actor,
            owner.session,
            expected,
            spot,
            false,
            &closed
        )
        .is_err()
    );
}

#[test]
fn tools_rope_spot_hidden_target_epoch_and_pz_landing() {
    let owner = Owner::new();
    let intent = owner.intent(10, 0);
    let mut runtime = owner.runtime.borrow_mut();
    let at =
        crate::gameplay_transport::bundle_map_position(&mut runtime, &owner.cells, owner.actor)
            .unwrap();
    let mut overlay = crate::map::overlay::ChannelOverlay::new(
        at.map.overlay.base().clone(),
        WorldId::decode(&id(1)).unwrap(),
        ChannelId::decode(&id(2)).unwrap(),
    );
    overlay
        .hide(
            crate::map::overlay::TilePos {
                x: 10,
                y: 10,
                floor: -8,
            },
            0,
            crate::map::overlay::Admission::Refusable,
        )
        .unwrap();
    let before = runtime.read_actor_position(owner.actor).unwrap();
    let command = UseCommand {
        game_session_id: owner.session,
        command_id: 1,
        item_fence: None,
    };
    let now = SemanticTimeMicros::from_micros(100);
    let mut states = owner.states.borrow_mut();
    assert_eq!(
        rope_in_channel(
            &mut runtime,
            &mut states,
            &owner.cells,
            owner.actor,
            command,
            &intent,
            None,
            0,
            now,
            Some(&overlay)
        ),
        UseDisposition::StaleState
    );
    let mut handled = intent;
    handled.use_with_map_item = Some(MapUseTarget::Item(NonZeroU64::new(9).unwrap()));
    let key = ItemKey::MapBase {
        bundle_digest: at.map.overlay.base().digest(),
        placement_key: oteryn_world_bundle::bundle::placement_key(-8, 10, 10, 0).unwrap(),
        reset_epoch: 1,
    };
    assert_eq!(
        rope_in_channel(
            &mut runtime,
            &mut states,
            &owner.cells,
            owner.actor,
            command,
            &handled,
            Some(key),
            0,
            now,
            None
        ),
        UseDisposition::StaleState
    );
    assert_eq!(runtime.read_actor_position(owner.actor).unwrap(), before);
    assert!(
        states
            .get(&runtime, owner.actor, owner.session)
            .unwrap()
            .cooldowns
            .group_ready_at(oteryn_gameplay_spell_rules::cast::STAIRHOP_COOLDOWN_KEY)
            .is_none()
    );
    let closed = std::collections::BTreeSet::new();
    let proof = crate::map::floor::prepare_bundle_rope(
        &runtime,
        &owner.cells,
        owner.actor,
        owner.session,
        before,
        LogicalCell { x: 10, y: 10, z: 8 },
        true,
        &closed,
    )
    .unwrap();
    runtime.commit_source_step(proof).unwrap();
    assert_eq!(
        runtime.read_actor_position(owner.actor).unwrap().position(),
        MovementLocalPosition {
            x: 10,
            y: 9,
            floor: 7
        }
    );
}
