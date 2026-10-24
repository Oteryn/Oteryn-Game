// --- ITEM-CLIENT-1: capability 4 (domains 9 and 11, command 9, the USE item target and the
// item handle on domain-1 entities).

const ITEM_SELECTED: &[u32] = &[4, 6];

fn handle(value: u64) -> ItemHandle {
    ItemHandle::new(value).unwrap_or(ItemHandle::MIN)
}

fn entry(handle_value: u64, definition: u32, count: u32) -> ItemEntry {
    ItemEntry {
        handle: handle(handle_value),
        item_definition_ref: std::num::NonZeroU32::new(definition)
            .unwrap_or(std::num::NonZeroU32::MIN),
        count: std::num::NonZeroU32::new(count).unwrap_or(std::num::NonZeroU32::MIN),
        sub_type: 0,
    }
}

fn loot_corpse(marker: u8, handle_value: u64, position: ActorPosition) -> WorldSpatialEntity {
    let mut entity = corpse(marker, position);
    entity.detail = EntityDetail::Object {
        item_definition_ref: 9,
        quantity: 1,
        item_handle: Some(handle(handle_value)),
    };
    entity
}

fn item_snapshot(others: Vec<WorldSpatialEntity>) -> Result<(u32, Vec<u8>), BoxError> {
    let mut entities = vec![own(at(0, 0))];
    entities.extend(others);
    let payload = world_spatial_entities::encode_world_spatial_entities_snapshot_with_item_handles(
        &WorldSpatialEntitiesSnapshot {
            content_generation: CONTENT_GENERATION,
            actor_position: at(0, 0),
            own_identity: OWN,
            entities,
        },
    )
    .map_err(|error| format!("{error:?}"))?;
    Ok((SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2, payload))
}

fn inventory_domain(
    revision: u64,
    view: &CharacterInventory,
) -> Result<(u32, u64, u32, Vec<u8>), BoxError> {
    Ok((
        STATE_DOMAIN_CHARACTER_INVENTORY,
        revision,
        item_view::SNAPSHOT_TYPE_CHARACTER_INVENTORY_V1,
        item_view::encode_character_inventory(view).map_err(|error| format!("{error:?}"))?,
    ))
}

fn container_domain(
    revision: u64,
    view: &OpenContainer,
) -> Result<(u32, u64, u32, Vec<u8>), BoxError> {
    Ok((
        STATE_DOMAIN_OPEN_CONTAINER,
        revision,
        item_view::SNAPSHOT_TYPE_OPEN_CONTAINER_V1,
        item_view::encode_open_container(view).map_err(|error| format!("{error:?}"))?,
    ))
}

fn inventory_push(
    sequence: u64,
    base: u64,
    new: u64,
    view: &CharacterInventory,
) -> Result<Step, BoxError> {
    Ok(Step::Send(encode_state_delta(
        1,
        sequence,
        STATE_DOMAIN_CHARACTER_INVENTORY,
        base,
        new,
        item_view::DELTA_TYPE_CHARACTER_INVENTORY_V1,
        &item_view::encode_character_inventory(view).map_err(|error| format!("{error:?}"))?,
    )?))
}

fn container_push(
    sequence: u64,
    base: u64,
    new: u64,
    view: &OpenContainer,
) -> Result<Step, BoxError> {
    Ok(Step::Send(encode_state_delta(
        1,
        sequence,
        STATE_DOMAIN_OPEN_CONTAINER,
        base,
        new,
        item_view::DELTA_TYPE_OPEN_CONTAINER_V1,
        &item_view::encode_open_container(view).map_err(|error| format!("{error:?}"))?,
    )?))
}

fn move_result(
    sequence: u64,
    id: u64,
    status: CommandStatus,
    outcome: ItemMoveOutcome,
) -> Result<Step, BoxError> {
    Ok(Step::Send(encode_command_result(
        1,
        sequence,
        id,
        status,
        &item_view::encode_item_move_result(outcome).map_err(|error| format!("{error:?}"))?,
    )?))
}

fn use_result(sequence: u64, id: u64, disposition: UseDisposition) -> Result<Step, BoxError> {
    Ok(Step::Send(encode_command_result(
        1,
        sequence,
        id,
        CommandStatus::Accepted,
        &world_object::encode_use_result(disposition).unwrap_or_default(),
    )?))
}

fn item_admission() -> Result<Admission<'static>, BoxError> {
    entity_admission()
}

/// Joins with capabilities 4 and 6 selected, `snapshot` for domain 1 and `raw` domains; plays
/// `script`, where `Expect` reads one command and checks its type and payload.
enum ItemStep {
    Send(Step),
    Expect(u32, Vec<u8>),
}

fn item_peer(
    snapshot: (u32, Vec<u8>),
    raw: Vec<(u32, u64, u32, Vec<u8>)>,
    script: Vec<ItemStep>,
) -> (DuplexStream, tokio::task::JoinHandle<Result<(), BoxError>>) {
    let (client, mut server) = tokio::io::duplex(256 * 1024);
    let peer = tokio::spawn(async move {
        join_peer_with(
            &mut server,
            CLIENT_SUPPORTED_CAPABILITIES,
            ITEM_SELECTED,
            &[],
            snapshot,
            &raw,
        )
        .await?;
        for step in script {
            match step {
                ItemStep::Send(step) => play_step(&mut server, step).await?,
                ItemStep::Expect(command_type, payload) => {
                    let frame = read_frame(&mut server).await?;
                    let envelope = decode_wire_envelope(&frame)?;
                    let command = envelope.client_command(1)?;
                    assert_eq!(command.command_type, command_type);
                    assert_eq!(command.payload, payload.as_slice());
                }
            }
        }
        let mut rest = Vec::new();
        let _ = server.read_to_end(&mut rest).await;
        Ok::<(), BoxError>(())
    });
    (client, peer)
}

#[test]
fn the_advertised_set_carries_capability_4_with_its_requirement() {
    assert!(CLIENT_SUPPORTED_CAPABILITIES.contains(&item_view::CAPABILITY_ITEM_VIEW_MOVE_V1));
    assert!(CLIENT_SUPPORTED_CAPABILITIES.contains(&CAPABILITY_WORLD_SPATIAL_ENTITIES));
    // Capabilities 12 and 14 (equipment, Ground, the container tree) are not advertised.
    assert!(!CLIENT_SUPPORTED_CAPABILITIES.contains(&12));
    assert!(!CLIENT_SUPPORTED_CAPABILITIES.contains(&14));
}

#[test]
fn the_join_snapshot_sets_the_inventory_and_open_container_or_leaves_them_empty()
-> Result<(), BoxError> {
    block_on(async {
        let inventory = CharacterInventory {
            main_backpack: Some(entry(1, 2854, 1)),
            entries: vec![entry(2, 3031, 5), entry(3, 3035, 1)],
            equipment: vec![],
        };
        let open = OpenContainer {
            container_handle: Some(handle(40)),
            entries: vec![entry(41, 3031, 2)],
        };
        let (client, peer) = item_peer(
            item_snapshot(vec![loot_corpse(1, 40, at(1, 0))])?,
            vec![
                inventory_domain(3, &inventory)?,
                container_domain(4, &open)?,
            ],
            vec![],
        );
        let session = Session::admit(client, item_admission()?).await?;
        assert_eq!(session.inventory(), Some(&inventory));
        assert_eq!(session.open_container(), Some(&open));
        let stored = session.world_entities().ok_or("capability 6 selected")?;
        assert!(stored.iter().any(|entity| matches!(
            entity.detail,
            EntityDetail::Object {
                item_handle: Some(found),
                ..
            } if found == handle(40)
        )));
        drop(session);
        peer.await??;

        // Selected but never sent: empty, not absent.
        let (client, peer) = item_peer(item_snapshot(vec![])?, vec![], vec![]);
        let session = Session::admit(client, item_admission()?).await?;
        assert_eq!(session.inventory(), Some(&CharacterInventory::default()));
        assert_eq!(session.open_container(), Some(&OpenContainer::default()));
        drop(session);
        peer.await??;

        // Unselected: no item state at all, and the plain entity codec.
        let (client, peer) = entity_peer(entity_snapshot(vec![]), &[6], vec![]);
        let session = Session::admit(client, item_admission()?).await?;
        assert!(session.inventory().is_none());
        assert!(session.open_container().is_none());
        drop(session);
        peer.await??;
        Ok(())
    })?
}

#[test]
fn item_snapshots_of_another_type_or_without_the_capability_are_refused() -> Result<(), BoxError> {
    block_on(async {
        let (client, peer) = item_peer(
            item_snapshot(vec![])?,
            vec![(STATE_DOMAIN_CHARACTER_INVENTORY, 3, 2, vec![])],
            vec![],
        );
        let result = Session::admit(client, item_admission()?).await;
        drop(peer);
        assert!(matches!(
            result,
            Err(SessionError::UnregisteredSnapshotType {
                domain_id: STATE_DOMAIN_CHARACTER_INVENTORY,
                snapshot_type: 2
            })
        ));

        // Domain 11 named without capability 4.
        let (client, mut server) = tokio::io::duplex(64 * 1024);
        let peer = tokio::spawn(async move {
            join_peer_with(
                &mut server,
                &[13],
                &[13],
                &[],
                (SNAPSHOT_TYPE_WORLD_SPATIAL_V1, spatial(0)),
                &[container_domain(3, &OpenContainer::default())
                    .map_err(|error| format!("{error:?}"))?],
            )
            .await
        });
        let result = Session::admit(client, admission()?).await;
        let _ = peer.await?;
        assert!(matches!(
            result,
            Err(SessionError::UnselectedDomain {
                domain_id: STATE_DOMAIN_OPEN_CONTAINER
            })
        ));
        Ok(())
    })?
}

#[test]
fn a_duplicate_item_handle_in_a_snapshot_is_refused() -> Result<(), BoxError> {
    block_on(async {
        // Inventory: handles 0x51 and 0x52, then 0x52 patched to a second 0x51.
        let mut payload = item_view::encode_character_inventory(&CharacterInventory {
            main_backpack: Some(entry(1, 2854, 1)),
            entries: vec![entry(0x51, 1, 1), entry(0x52, 1, 1)],
            equipment: vec![],
        })
        .map_err(|error| format!("{error:?}"))?;
        assert_eq!(payload.iter().filter(|byte| **byte == 0x52).count(), 1);
        for byte in &mut payload {
            if *byte == 0x52 {
                *byte = 0x51;
            }
        }
        let (client, peer) = item_peer(
            item_snapshot(vec![])?,
            vec![(
                STATE_DOMAIN_CHARACTER_INVENTORY,
                3,
                item_view::SNAPSHOT_TYPE_CHARACTER_INVENTORY_V1,
                payload,
            )],
            vec![],
        );
        let result = Session::admit(client, item_admission()?).await;
        drop(peer);
        assert!(matches!(result, Err(SessionError::ItemView(_))));

        // Entities: two corpses with the same handle.
        let (kind, mut payload) = item_snapshot(vec![
            loot_corpse(1, 0x51, at(1, 0)),
            loot_corpse(2, 0x52, at(2, 0)),
        ])?;
        assert_eq!(payload.iter().filter(|byte| **byte == 0x52).count(), 1);
        for byte in &mut payload {
            if *byte == 0x52 {
                *byte = 0x51;
            }
        }
        let (client, peer) = item_peer((kind, payload), vec![], vec![]);
        let result = Session::admit(client, item_admission()?).await;
        drop(peer);
        assert!(matches!(result, Err(SessionError::WorldSpatial(_))));
        Ok(())
    })?
}

#[test]
fn field_10_is_required_with_capability_4_and_refused_without_it() -> Result<(), BoxError> {
    block_on(async {
        // Capability 4 selected, a corpse without a handle: the plain-codec snapshot.
        let (client, peer) = item_peer(entity_snapshot(vec![corpse(1, at(1, 0))]), vec![], vec![]);
        let result = Session::admit(client, item_admission()?).await;
        drop(peer);
        assert!(matches!(result, Err(SessionError::WorldSpatial(_))));

        // Capability 4 not selected, a corpse with a handle.
        let (kind, payload) = item_snapshot(vec![loot_corpse(1, 40, at(1, 0))])?;
        let (client, peer) = entity_peer((kind, payload), &[6], vec![]);
        let result = Session::admit(client, item_admission()?).await;
        drop(peer);
        assert!(matches!(result, Err(SessionError::WorldSpatial(_))));
        Ok(())
    })?
}

#[test]
fn capability_11_dispositions_and_the_gate_id_need_the_negotiated_capability() {
    use UseDisposition as D;
    for (disposition, gate) in [
        (D::Sealed, Some(9)),
        (D::Locked, None),
        (D::NotPossible, None),
    ] {
        assert!(check_use_result_negotiated(disposition, gate, true, false).is_ok());
        assert!(matches!(
            check_use_result_negotiated(disposition, gate, false, true),
            Err(SessionError::WorldObject(_))
        ));
    }
    for disposition in [D::Committed, D::NothingToUse, D::NoTarget, D::Rejected] {
        assert!(check_use_result_negotiated(disposition, None, false, false).is_ok());
    }
}

#[test]
fn rune_use_dispositions_18_and_19_need_capability_21() {
    use UseDisposition as D;
    for disposition in [D::TargetIllegal, D::ProtectionZone] {
        assert!(check_use_result_negotiated(disposition, None, false, true).is_ok());
        assert!(matches!(
            check_use_result_negotiated(disposition, None, true, false),
            Err(SessionError::WorldObject(_))
        ));
    }
}

#[test]
fn a_sealed_use_result_without_capability_11_fails_closed_and_others_carry_no_gate_id()
-> Result<(), BoxError> {
    block_on(async {
        let sealed = world_object::encode_use_result_with_gate(UseDisposition::Sealed, Some(9))
            .map_err(|error| format!("{error:?}"))?;
        let (client, peer) = item_peer(
            item_snapshot(vec![loot_corpse(1, 40, at(1, 0))])?,
            vec![],
            vec![
                ItemStep::Expect(
                    world_object::COMMAND_TYPE_USE_INTENT,
                    world_object::encode_use_item_intent(handle(40)),
                ),
                ItemStep::Send(use_result(41, 7, UseDisposition::NothingToUse)?),
                ItemStep::Expect(
                    world_object::COMMAND_TYPE_USE_INTENT,
                    world_object::encode_use_item_intent(handle(40)),
                ),
                ItemStep::Send(Step::Send(encode_command_result(
                    1,
                    42,
                    8,
                    CommandStatus::Accepted,
                    &sealed,
                )?)),
            ],
        );
        let mut session = Session::admit(client, item_admission()?).await?;
        let used = session.use_item(handle(40)).await?;
        assert_eq!(used.disposition, UseDisposition::NothingToUse);
        assert_eq!(used.gate_message_id, None);
        // Capability 11 is not selected: the SEALED result is a negotiation fault.
        assert!(matches!(
            session.use_item(handle(40)).await,
            Err(SessionError::WorldObject(_))
        ));
        drop(session);
        peer.await??;
        Ok(())
    })?
}

#[test]
fn opening_a_corpse_and_looting_one_entry_follows_the_result_with_both_deltas()
-> Result<(), BoxError> {
    block_on(async {
        let before = CharacterInventory {
            main_backpack: Some(entry(1, 2854, 1)),
            entries: vec![entry(2, 3031, 5)],
            equipment: vec![],
        };
        let after = CharacterInventory {
            main_backpack: Some(entry(1, 2854, 1)),
            entries: vec![entry(2, 3031, 5), entry(41, 3035, 1)],
            equipment: vec![],
        };
        let opened = OpenContainer {
            container_handle: Some(handle(40)),
            entries: vec![entry(41, 3035, 1), entry(42, 3031, 2)],
        };
        let looted = OpenContainer {
            container_handle: Some(handle(40)),
            entries: vec![entry(42, 3031, 2)],
        };
        let (client, peer) = item_peer(
            item_snapshot(vec![loot_corpse(1, 40, at(1, 0))])?,
            vec![
                inventory_domain(3, &before)?,
                container_domain(4, &OpenContainer::default())?,
            ],
            vec![
                ItemStep::Expect(
                    world_object::COMMAND_TYPE_USE_INTENT,
                    world_object::encode_use_item_intent(handle(40)),
                ),
                ItemStep::Send(use_result(41, 7, UseDisposition::Committed)?),
                ItemStep::Send(container_push(42, 4, 5, &opened)?),
                ItemStep::Expect(
                    COMMAND_TYPE_ITEM_MOVE_INTENT,
                    item_view::encode_item_move_intent(&ItemMoveIntent {
                        source: handle(41),
                        destination: ItemMoveDestination::MainBackpack,
                    })
                    .map_err(|error| format!("{error:?}"))?,
                ),
                ItemStep::Send(move_result(
                    43,
                    8,
                    CommandStatus::Accepted,
                    ItemMoveOutcome::Moved,
                )?),
                ItemStep::Send(inventory_push(44, 3, 4, &after)?),
                ItemStep::Send(container_push(45, 5, 6, &looted)?),
            ],
        );
        let mut session = Session::admit(client, item_admission()?).await?;
        let used = session.use_item(handle(40)).await?;
        assert_eq!(used.disposition, UseDisposition::Committed);
        let moved = session
            .move_item(&ItemMoveIntent {
                source: handle(41),
                destination: ItemMoveDestination::MainBackpack,
            })
            .await?;
        assert_eq!(moved.outcome, ItemMoveOutcome::Moved);
        assert_eq!(moved.status, CommandStatus::Accepted);
        // The deltas follow the results and go through the store and the queue.
        session
            .service_liveness(Duration::from_millis(200))
            .await
            .ok();
        let events = session.take_events();
        assert_eq!(events.len(), 3, "{events:?}");
        assert!(matches!(&events[0], SessionEvent::OpenContainer(delta) if delta.value == opened));
        assert!(matches!(&events[1], SessionEvent::Inventory(delta) if delta.value == after));
        assert!(matches!(&events[2], SessionEvent::OpenContainer(delta) if delta.value == looted));
        assert_eq!(session.inventory(), Some(&after));
        assert_eq!(session.open_container(), Some(&looted));
        drop(session);
        peer.await??;
        Ok(())
    })?
}

#[test]
fn a_stale_move_result_changes_nothing_locally() -> Result<(), BoxError> {
    block_on(async {
        let inventory = CharacterInventory {
            main_backpack: Some(entry(1, 2854, 1)),
            entries: vec![],
            equipment: vec![],
        };
        let open = OpenContainer {
            container_handle: Some(handle(40)),
            entries: vec![entry(41, 3035, 1)],
        };
        let (client, peer) = item_peer(
            item_snapshot(vec![])?,
            vec![
                inventory_domain(3, &inventory)?,
                container_domain(4, &open)?,
            ],
            vec![
                ItemStep::Send(Step::ReadCommand),
                ItemStep::Send(move_result(
                    41,
                    7,
                    CommandStatus::Accepted,
                    ItemMoveOutcome::Stale,
                )?),
            ],
        );
        let mut session = Session::admit(client, item_admission()?).await?;
        let moved = session
            .move_item(&ItemMoveIntent {
                source: handle(41),
                destination: ItemMoveDestination::MainBackpack,
            })
            .await?;
        assert_eq!(moved.outcome, ItemMoveOutcome::Stale);
        assert_eq!(session.inventory(), Some(&inventory));
        assert_eq!(session.open_container(), Some(&open));
        assert!(session.take_events().is_empty());
        // The session stays usable.
        assert!(session.ensure_usable().is_ok());
        drop(session);
        peer.await??;
        Ok(())
    })?
}

#[test]
fn item_commands_are_refused_before_sending_without_capability_4() -> Result<(), BoxError> {
    block_on(async {
        let (client, peer) = entity_peer(entity_snapshot(vec![]), &[6], vec![]);
        let mut session = Session::admit(client, item_admission()?).await?;
        assert!(matches!(
            session.use_item(handle(1)).await,
            Err(SessionError::CapabilityNotSelected { capability: 4 })
        ));
        assert!(matches!(
            session
                .move_item(&ItemMoveIntent {
                    source: handle(1),
                    destination: ItemMoveDestination::MainBackpack,
                })
                .await,
            Err(SessionError::CapabilityNotSelected { capability: 4 })
        ));
        // Equipment and Ground need capability 12: refused by the codec.
        let (client, peer2) = item_peer(item_snapshot(vec![])?, vec![], vec![]);
        let mut with_item = Session::admit(client, item_admission()?).await?;
        assert!(matches!(
            with_item
                .move_item(&ItemMoveIntent {
                    source: handle(1),
                    destination: ItemMoveDestination::Container(handle(2)),
                })
                .await,
            Err(SessionError::ItemView(_))
        ));
        drop((session, with_item));
        peer.await??;
        peer2.await??;
        Ok(())
    })?
}

#[test]
fn a_pushed_item_delta_with_a_wrong_revision_or_a_duplicate_handle_fails_closed()
-> Result<(), BoxError> {
    block_on(async {
        let mut duplicate = item_view::encode_open_container(&OpenContainer {
            container_handle: Some(handle(0x40)),
            entries: vec![entry(0x51, 1, 1), entry(0x52, 1, 1)],
        })
        .map_err(|error| format!("{error:?}"))?;
        for byte in &mut duplicate {
            if *byte == 0x52 {
                *byte = 0x51;
            }
        }
        let stale_base = container_push(41, 9, 10, &OpenContainer::default())?;
        let dup_push = Step::Send(encode_state_delta(
            1,
            41,
            STATE_DOMAIN_OPEN_CONTAINER,
            4,
            5,
            item_view::DELTA_TYPE_OPEN_CONTAINER_V1,
            &duplicate,
        )?);
        for (push, check) in [
            (
                stale_base,
                (|error: &SessionError| matches!(error, SessionError::StateRevisionMismatch { .. }))
                    as fn(&SessionError) -> bool,
            ),
            (dup_push, |error: &SessionError| {
                matches!(error, SessionError::ItemView(_))
            }),
        ] {
            let (client, peer) = item_peer(
                item_snapshot(vec![])?,
                vec![container_domain(4, &OpenContainer::default())?],
                vec![ItemStep::Send(push)],
            );
            let mut session = Session::admit(client, item_admission()?).await?;
            let error = session
                .service_liveness(Duration::from_millis(200))
                .await
                .err()
                .ok_or("the push must fail closed")?;
            assert!(check(&error), "{error:?}");
            assert_eq!(session.open_container(), Some(&OpenContainer::default()));
            assert!(matches!(
                session.use_item(handle(1)).await,
                Err(SessionError::SessionUnusable)
            ));
            drop(session);
            peer.await??;
        }
        Ok(())
    })?
}
