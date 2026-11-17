#[test]
fn soulpit_native_core_obelisk_scope_and_exit_walkthrough() -> Result<(), Box<dyn std::error::Error>>
{
    use oteryn_session::activity_scope::{
        ActivityScope, ActivityScopeSnapshot, encode_activity_scope,
    };
    use oteryn_session::world_map::*;
    // Crystal 201189f data-global/scripts/quests/soulpit/soulpit_fight.lua:37-106:
    // use a carried core on the source obelisk. soulpit_arena_exit.lua:8-16 uses the exit.
    let mut view = item_map_view()?;
    let (link, mut commands, outbox) = play_channel();
    outbox.targeting_initialize(true);
    let core = carried(2)?;
    outbox.inventory_initialize(
        Some(oteryn_session::CharacterInventory {
            main_backpack: Some(core),
            ..Default::default()
        }),
        false,
    );
    let source_map = WorldMapViewSnapshot {
        header: MapViewHeader {
            content_generation: [1; 32],
            bundle_digest: [2; 32],
            reset_epoch: 7,
            origin: ActorPosition {
                x: 32375,
                y: 31158,
                floor: -8,
            },
            first_visible_floor: -8,
        },
        tiles: vec![MapTile {
            position: ActorPosition {
                x: 32375,
                y: 31157,
                floor: -8,
            },
            items: vec![MapItem {
                definition: MapDefinition::Item(NonZeroU32::MIN),
                count: 1,
                sub_type: 0,
                origin: MapOrigin::BaseOrdinal {
                    ordinal: 0,
                    object_revision: 4,
                },
                appearance_id: 0,
            }],
            more: false,
            ground_speed: 0,
        }],
    };
    view.set_live_map(&source_map)?;
    assert!(link.inventory_select(core.handle));
    assert!(link.inventory_use_with());
    view.primary_click(TileCoord::new(32375, 31157), &link)?;
    let crate::combat_input::PlayCommand::WorldContext(command) = commands.try_recv()? else {
        return Err("expected native use-with".into());
    };
    assert_eq!(command.action, Action::UseOn(core.handle));
    assert!(matches!(command.target, Target::Object(_)));
    assert_eq!(
        command
            .selection
            .as_ref()
            .and_then(|selection| selection.binding),
        Some(([1; 32], [2; 32], 7))
    );
    assert!(view.inputs.is_empty());
    let mut id = [1; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    let mut successor = id;
    successor[15] = 2;
    let payload = encode_activity_scope(&ActivityScopeSnapshot {
        handoff: id,
        source_session: id,
        session: successor,
        scope: ActivityScope::Instance {
            world: id,
            instance: id,
        },
        scope_generation: 1,
        connection_generation: 1,
        lease_generation: 2,
        selection: vec![117],
        requires_ack: false,
    })
    .map_err(|e| format!("{e:?}"))?;
    let exit_position = ActorPosition {
        x: 32375,
        y: 31153,
        floor: -8,
    };
    let mut destination = source_map.clone();
    destination.header.origin = ActorPosition {
        x: 32375,
        y: 31151,
        floor: -8,
    };
    destination.header.reset_epoch = 8;
    destination.tiles[0].position = exit_position;
    link.request(StepDir::North);
    view.inputs.push_back(Input::TargetWalk(ActorPosition {
        x: 32374,
        y: 31158,
        floor: -8,
    }));
    outbox.push(vec![
        oteryn_session::SessionEvent::GatedSnapshotReplaced(oteryn_session::AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value: oteryn_session::GatedSnapshot {
                domain_id: 99,
                snapshot_type: 1,
                revision: 1,
                payload,
            },
        }),
        oteryn_session::SessionEvent::WorldMapSnapshot(oteryn_session::AppliedDelta {
            server_sequence: 0,
            base_revision: 0,
            new_revision: 1,
            value: destination,
        }),
    ]);
    view.tick(&link).map_err(|e| format!("{e:?}"))?;
    assert!(
        outbox.death_command(&mut commands).is_err(),
        "source queue was canceled"
    );
    assert!(
        view.action_line
            .is_some_and(|line| line.contains("Entered activity"))
    );
    assert!(
        !super::super::mailbox::lock(&link.mailbox)
            .world_context
            .pending
    );
    assert_eq!(
        super::super::mailbox::lock(&link.mailbox)
            .world_context
            .targeting,
        None
    );
    assert!(view.inputs.is_empty());
    let menu = view
        .world_context(TileCoord::new(exit_position.x, exit_position.y), &link)
        .ok_or("native exit menu")?;
    assert!(view.world_context_choose(&link, &menu, Action::Use));
    let crate::combat_input::PlayCommand::WorldContext(command) = commands.try_recv()? else {
        return Err("expected native exit use".into());
    };
    assert_eq!(command.action, Action::Use);
    assert!(matches!(command.target, Target::Object(_)));
    assert_eq!(
        command.selection.and_then(|selection| selection.binding),
        Some(([1; 32], [2; 32], 8))
    );
    assert!(commands.try_recv().is_err());
    let mut returned = successor;
    returned[15] = 3;
    let payload = encode_activity_scope(&ActivityScopeSnapshot {
        handoff: id,
        source_session: successor,
        session: returned,
        scope: ActivityScope::Channel {
            world: id,
            channel: id,
        },
        scope_generation: 1,
        connection_generation: 1,
        lease_generation: 3,
        selection: vec![117],
        requires_ack: false,
    })
    .map_err(|e| format!("{e:?}"))?;
    outbox.push(vec![
        oteryn_session::SessionEvent::GatedSnapshotReplaced(oteryn_session::AppliedDelta {
            server_sequence: 0,
            base_revision: 0,
            new_revision: 1,
            value: oteryn_session::GatedSnapshot {
                domain_id: 99,
                snapshot_type: 1,
                revision: 1,
                payload,
            },
        }),
        oteryn_session::SessionEvent::WorldMapSnapshot(oteryn_session::AppliedDelta {
            server_sequence: 0,
            base_revision: 0,
            new_revision: 1,
            value: source_map,
        }),
    ]);
    view.tick(&link).map_err(|e| format!("{e:?}"))?;
    assert_eq!(view.action_line, Some("Returned to the channel."));
    assert!(
        !view.world_context_choose(&link, &menu, Action::Use),
        "old instance exit cannot replay in the channel"
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}
