use super::*;
use oteryn_client_runtime::world_interaction::{Action, Target};

#[path = "activity_scope.rs"]
mod activity_scope;

#[test]
fn targeting_click_sends_one_command_without_walking_or_attacking() -> Result<(), BatchError> {
    let mut view = view()?;
    let (link, mut commands, outbox) = play_channel();
    outbox.targeting_initialize(true);
    super::super::mailbox::lock(&link.mailbox)
        .world_context
        .begin_targeting(std::num::NonZeroU64::MIN);
    view.primary_click(TileCoord::new(98, 200), &link)?;
    assert!(view.inputs.is_empty());
    assert!(
        matches!(commands.try_recv(), Ok(crate::combat_input::PlayCommand::WorldContext(command)) if command.action == Action::UseOn(std::num::NonZeroU64::MIN) && command.target == Target::Position(ActorPosition {x:98,y:200,floor:0}))
    );
    // While pending, the real primary-click route must consume the click too.
    view.primary_click(TileCoord::new(99, 200), &link)?;
    assert!(view.inputs.is_empty());
    assert!(commands.try_recv().is_err());
    super::super::mailbox::lock(&link.mailbox)
        .world_context
        .pending = false;
    view.primary_click(TileCoord::new(99, 200), &link)?;
    assert!(matches!(view.inputs.front(), Some(Input::TargetWalk(_))));
    Ok(())
}

#[test]
fn escape_cancels_only_active_targeting_and_shutdown_clears_it() {
    let (link, mut commands, outbox) = play_channel();
    let escape = oteryn_input_actions::NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::ESCAPE,
        state: oteryn_input_actions::ButtonState::Pressed,
        modifiers: oteryn_input_actions::Modifiers::NONE,
        repeat: false,
    };
    assert!(!link.use_with_cancel_input(std::slice::from_ref(&escape)));
    super::super::mailbox::lock(&link.mailbox)
        .world_context
        .begin_targeting(std::num::NonZeroU64::MIN);
    assert!(link.use_with_cancel_input(std::slice::from_ref(&escape)));
    assert!(!link.use_with_cancel_input(std::slice::from_ref(&escape)));
    assert!(commands.try_recv().is_err());
    super::super::mailbox::lock(&link.mailbox)
        .world_context
        .begin_targeting(std::num::NonZeroU64::MIN);
    outbox.inventory_end();
    assert!(!link.use_with_cancel_input(std::slice::from_ref(&escape)));
}

#[test]
fn pending_and_self_targets_preserve_targeting_and_send_nothing() -> Result<(), BatchError> {
    let view = view()?;
    let (link, mut commands, outbox) = play_channel();
    outbox.targeting_initialize(true);
    let source = std::num::NonZeroU64::MIN;
    {
        let mut mailbox = super::super::mailbox::lock(&link.mailbox);
        mailbox.world_context.begin_targeting(source);
        mailbox.world_context.pending = true;
    }
    assert!(view.use_with_click(TileCoord::new(98, 200), &link));
    assert!(commands.try_recv().is_err());
    assert_eq!(
        super::super::mailbox::lock(&link.mailbox)
            .world_context
            .targeting,
        Some(source)
    );
    assert!(
        link.world_context_feedback()
            .is_some_and(|text| text.contains("waiting"))
    );
    Ok(())
}

fn carried(handle: u64) -> Result<oteryn_session::ItemEntry, Box<dyn std::error::Error>> {
    Ok(oteryn_session::ItemEntry {
        handle: handle.try_into()?,
        item_definition_ref: 1.try_into()?,
        count: 1.try_into()?,
        sub_type: 0,
    })
}
fn item_map_view() -> Result<PlayView, Box<dyn std::error::Error>> {
    use oteryn_session::world_map::*;
    let mut view = view()?;
    view.set_live_map(&WorldMapViewSnapshot {
        header: MapViewHeader {
            content_generation: [1; 32],
            bundle_digest: [2; 32],
            reset_epoch: 0,
            origin: ActorPosition {
                x: 100,
                y: 200,
                floor: 0,
            },
            first_visible_floor: 0,
        },
        tiles: [100, 101]
            .into_iter()
            .map(|x| MapTile {
                position: ActorPosition {
                    x,
                    y: 200,
                    floor: 0,
                },
                items: vec![MapItem {
                    definition: MapDefinition::Item(NonZeroU32::MIN),
                    count: 1,
                    sub_type: 0,
                    origin: MapOrigin::Handle(std::num::NonZeroU64::MIN),
                    appearance_id: 0,
                }],
                more: false,
                ground_speed: 0,
            })
            .collect(),
    })?;
    Ok(view)
}
#[test]
fn rune_uses_position_on_item_tiles_including_own_tile_and_tools_keep_items()
-> Result<(), Box<dyn std::error::Error>> {
    for (rune, x) in [(true, 101), (true, 100), (false, 101)] {
        let mut view = item_map_view()?;
        let (link, mut commands, outbox) = play_channel();
        outbox.targeting_initialize(true);
        let item = carried(2)?;
        outbox.inventory_initialize(
            Some(oteryn_session::CharacterInventory {
                main_backpack: Some(item),
                ..Default::default()
            }),
            false,
        );
        assert!(link.inventory_select(item.handle));
        assert!(if rune {
            link.inventory_use_rune()
        } else {
            link.inventory_use_with()
        });
        view.primary_click(TileCoord::new(x, 200), &link)?;
        let crate::combat_input::PlayCommand::WorldContext(command) = commands.try_recv()? else {
            return Err("unexpected command".into());
        };
        assert_eq!(
            command.target,
            if rune {
                Target::Position(ActorPosition {
                    x,
                    y: 200,
                    floor: 0,
                })
            } else {
                Target::Item(std::num::NonZeroU64::MIN)
            }
        );
        assert_eq!(
            super::super::mailbox::lock(&link.mailbox)
                .world_context
                .targeting,
            None
        );
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}
#[test]
fn self_target_keeps_mode_and_inventory_destination_consumes_once()
-> Result<(), Box<dyn std::error::Error>> {
    let mut view = item_map_view()?;
    let (link, mut commands, outbox) = play_channel();
    outbox.targeting_initialize(true);
    outbox.inventory_initialize(
        Some(oteryn_session::CharacterInventory {
            main_backpack: Some(carried(1)?),
            entries: vec![carried(2)?],
            equipment: vec![],
        }),
        false,
    );
    assert!(link.inventory_select(std::num::NonZeroU64::MIN));
    assert!(link.inventory_use_with());
    assert!(!link.inventory_use_with());
    view.primary_click(TileCoord::new(101, 200), &link)?;
    assert!(commands.try_recv().is_err());
    assert_eq!(
        super::super::mailbox::lock(&link.mailbox)
            .world_context
            .targeting,
        Some(std::num::NonZeroU64::MIN)
    );
    assert!(link.inventory_select(2.try_into()?));
    assert!(link.inventory_use_with());
    assert!(
        matches!(commands.try_recv()?,crate::combat_input::PlayCommand::WorldContext(command) if command.action==Action::UseOn(std::num::NonZeroU64::MIN) && command.target==Target::Item(2.try_into()?))
    );
    assert!(!link.inventory_use_with());
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn published_source_removal_cancels_targeting() -> Result<(), Box<dyn std::error::Error>> {
    let (link, _, outbox) = play_channel();
    outbox.targeting_initialize(true);
    outbox.inventory_initialize(
        Some(oteryn_session::CharacterInventory {
            main_backpack: Some(carried(1)?),
            ..Default::default()
        }),
        false,
    );
    assert!(link.inventory_select(std::num::NonZeroU64::MIN));
    assert!(link.inventory_use_with());
    let session = Fake {
        serves: Arc::new(AtomicUsize::new(0)),
        fail_step: false,
        fail_serve_after: usize::MAX,
        queued: vec![],
        drained: Arc::new(AtomicUsize::new(0)),
        combats: Arc::new(Mutex::new(vec![])),
        logouts: Arc::default(),
    };
    outbox.targeting_refresh(&session);
    assert_eq!(
        super::super::mailbox::lock(&link.mailbox)
            .world_context
            .targeting,
        None
    );
    assert!(
        link.world_context_feedback()
            .is_some_and(|text| text.contains("disappeared"))
    );
    Ok(())
}

#[test]
fn native_inventory_targeting_keeps_other_panels_alive() -> Result<(), Box<dyn std::error::Error>> {
    let (link, _commands, outbox) = play_channel();
    outbox.inventory_initialize(
        Some(oteryn_session::CharacterInventory {
            main_backpack: Some(carried(1)?),
            ..Default::default()
        }),
        false,
    );
    outbox.targeting_initialize(true);
    assert!(link.inventory_select(1.try_into()?));
    assert!(PlayLink::inventory_use_shared(
        &link.mailbox,
        &link.commands,
        false
    ));
    let mailbox = super::super::mailbox::lock(&link.mailbox);
    assert_eq!(mailbox.world_context.targeting, Some(1.try_into()?));
    assert!(!mailbox.inventory_shutdown);
    assert!(!mailbox.depot_shutdown);
    assert!(!mailbox.market_shutdown);
    assert!(!mailbox.quest_shutdown);
    assert!(!mailbox.wheel_shutdown);
    Ok(())
}

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
