use super::*;
use oteryn_session::EntityKind;

#[test]
fn click_on_empty_ground_queues_a_server_target() -> Result<(), BatchError> {
    let mut view = view()?;
    view.click(TileCoord::new(98, 200))?;
    assert_eq!(
        view.inputs.front(),
        Some(&Input::TargetWalk(ActorPosition {
            x: 98,
            y: 200,
            floor: 0
        }))
    );
    // Target walking never generates a client step.
    assert_eq!(view.next_step(), None);
    // A click on the marker selects it and sets no goal.
    let mut other = self::view()?;
    other.click(TileCoord::new(102, 199))?;
    assert_eq!(other.next_step(), None);
    Ok(())
}

#[test]
fn only_a_creature_click_queues_an_attack() -> Result<(), BatchError> {
    let actor = |kind, x, id: u8| WorldSpatialEntity {
        motion: None,
        display_name: None,
        kind,
        entity: EntityRef {
            identity: [id; 16],
            generation: 1,
        },
        position: ActorPosition {
            x,
            y: 200,
            floor: 0,
        },
        detail: oteryn_session::EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    };
    let creature = actor(EntityKind::Creature, 101, 1);
    let mut view = view()?.with_entities(vec![creature, actor(EntityKind::Npc, 99, 2)])?;
    // The NPC is still drawn: player/marker plus a glyph and two health quads per entity.
    assert_eq!(view.scene().sprites().len(), 2 + 2 * 3);
    view.click(TileCoord::new(99, 200))?;
    assert!(view.inputs.is_empty());
    view.click(TileCoord::new(101, 200))?;
    assert_eq!(
        view.inputs.iter().copied().collect::<Vec<_>>(),
        vec![Input::Attack(creature.entity)]
    );
    Ok(())
}

#[test]
fn creature_click_dispatches_attack_and_reduces_the_report() -> Result<(), BatchError> {
    let target = oteryn_session::EntityRef {
        identity: [1; 16],
        generation: 1,
    };
    let entity = WorldSpatialEntity {
        motion: None,
        display_name: None,
        entity: target,
        kind: EntityKind::Creature,
        position: ActorPosition {
            x: 101,
            y: 200,
            floor: 0,
        },
        detail: oteryn_session::EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    };
    let mut view = view()?.with_entities(vec![entity])?;
    let (link, mut commands, outbox) = play_channel();
    view.click(TileCoord::new(101, 200))?;
    assert_eq!(view.tick(&link), Ok(()));
    assert_eq!(
        commands.try_recv(),
        Ok(crate::combat_input::PlayCommand::Combat(
            CombatCommand::Attack(target)
        ))
    );
    outbox.battle_result(CombatReport::Attack(AttackIntentDisposition::Ok), None, None);
    assert_eq!(view.tick(&link), Ok(()));
    assert!(view.combat_status().ends_with("Attacking"));
    Ok(())
}

use oteryn_client_runtime::world_interaction::{Action, Target};

#[test]
fn world_context_live_map_target_dispatch_and_stale_dismissal() -> Result<(), BatchError> {
    let mut view = view()?;
    let mut map = super::feature_local_world_view::live_snapshot(101);
    map.tiles[0].items[0].origin = oteryn_session::world_map::MapOrigin::BaseOrdinal {
        ordinal: 2,
        object_revision: 7,
    };
    view.set_live_map(&map)?;
    let (link, mut commands, outbox) = play_channel();
    outbox.world_context_initialize(true, true);
    let menu = view.world_context(TileCoord::new(101, 200), &link);
    assert!(menu.is_some());
    let menu = menu.ok_or(BatchError::InvalidView)?;
    assert!(matches!(menu.target, Target::Object(_)));
    assert!(view.world_context_choose(&link, &menu, Action::Look));
    assert!(matches!(
        commands.try_recv(),
        Ok(crate::combat_input::PlayCommand::WorldContext(_))
    ));
    super::super::mailbox::lock(&outbox.0).world_context.pending = false;
    view.set_live_map(&super::feature_local_world_view::live_snapshot(102))?;
    assert!(!view.world_context_choose(&link, &menu, Action::Use));
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn world_context_own_tile_item_and_reset_binding() -> Result<(), BatchError> {
    let mut view = view()?;
    let mut map = super::feature_local_world_view::live_snapshot(100);
    map.tiles[0].items[0].origin =
        oteryn_session::world_map::MapOrigin::Handle(std::num::NonZeroU64::MIN);
    view.set_live_map(&map)?;
    let (link, mut commands, outbox) = play_channel();
    outbox.world_context_initialize(true, true);
    let menu = view
        .world_context(TileCoord::new(100, 200), &link)
        .ok_or(BatchError::InvalidView)?;
    assert!(matches!(menu.target, Target::Item(_)));
    assert_eq!(menu.entries.len(), 3);
    map.header.reset_epoch += 1;
    view.set_live_map(&map)?;
    assert!(!view.world_context_choose(&link, &menu, Action::Open));
    assert!(commands.try_recv().is_err());
    map.tiles[0].items[0].origin = oteryn_session::world_map::MapOrigin::DisplayOnly;
    view.set_live_map(&map)?;
    assert!(
        view.world_context(TileCoord::new(100, 200), &link)
            .is_none()
    );
    assert!(view.world_context(TileCoord::new(99, 200), &link).is_none());
    Ok(())
}

#[test]
fn world_context_creature_look_keeps_generation_without_attack() -> Result<(), BatchError> {
    let entity = WorldSpatialEntity {
        motion: None,
        display_name: None,
        entity: EntityRef {
            identity: [3; 16],
            generation: 9,
        },
        kind: EntityKind::Creature,
        position: ActorPosition {
            x: 101,
            y: 200,
            floor: 0,
        },
        detail: oteryn_session::EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    };
    let view = view()?.with_entities(vec![entity])?;
    let (link, mut commands, outbox) = play_channel();
    outbox.world_context_initialize(true, true);
    let menu = view
        .world_context(TileCoord::new(101, 200), &link)
        .ok_or(BatchError::InvalidView)?;
    assert_eq!(menu.entries, vec![("Look", Action::Look)]);
    assert!(!view.world_context_choose(&link, &menu, Action::Use));
    assert!(view.world_context_choose(&link, &menu, Action::Look));
    assert_eq!(
        commands.try_recv(),
        Ok(crate::combat_input::PlayCommand::WorldContext(
            oteryn_client_runtime::world_interaction::Command {
                target: Target::Creature(entity.entity),
                action: Action::Look,
                selection: Some(oteryn_client_runtime::world_interaction::Selection {
                    position: entity.position,
                    binding: None
                }),
            }
        ))
    );
    assert!(view.inputs.is_empty());
    Ok(())
}

#[test]
fn world_context_npc_is_inspectable_without_becoming_attackable() -> Result<(), BatchError> {
    let entity = WorldSpatialEntity {
        motion: None,
        display_name: None,
        entity: EntityRef {
            identity: [4; 16],
            generation: 3,
        },
        kind: EntityKind::Npc,
        position: ActorPosition {
            x: 101,
            y: 200,
            floor: 0,
        },
        detail: oteryn_session::EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    };
    let mut view = view()?.with_entities(vec![entity])?;
    let (link, _commands, outbox) = play_channel();
    outbox.world_context_initialize(true, true);
    let menu = view
        .world_context(TileCoord::new(101, 200), &link)
        .ok_or(BatchError::InvalidView)?;
    assert_eq!(menu.target, Target::Creature(entity.entity));
    view.click(TileCoord::new(101, 200))?;
    assert!(view.inputs.is_empty());
    Ok(())
}

#[test]
fn tileless_context_revalidates_carried_items() -> Result<(), Box<dyn std::error::Error>> {
    let view = view()?;
    let item = oteryn_session::ItemEntry {
        appearance_id: 0,
        handle: std::num::NonZeroU64::MIN,
        item_definition_ref: NonZeroU32::MIN,
        count: NonZeroU32::MIN,
        sub_type: 0,
    };
    for action in [
        Action::Use,
        Action::Look,
        Action::Open,
        Action::UseWith,
        Action::UseRune,
    ] {
        let (link, mut commands, outbox) = play_channel();
        outbox.world_context_initialize(true, false);
        outbox.targeting_initialize(true);
        outbox.inventory_initialize(
            Some(oteryn_session::CharacterInventory {
                main_backpack: Some(item),
                ..Default::default()
            }),
            false,
        );
        let menu = oteryn_client_ui::world_interaction::menu::Menu::new(
            Target::Item(item.handle),
            true,
            false,
        )
        .with_targeting(true);
        assert!(
            view.world_context_choose(&link, &menu, action),
            "{action:?}"
        );
        if matches!(action, Action::UseWith | Action::UseRune) {
            assert!(link.use_with_armed());
        } else {
            assert!(
                matches!(commands.try_recv(), Ok(crate::combat_input::PlayCommand::WorldContext(command)) if command.action == action && command.selection.is_none())
            );
        }
        super::super::mailbox::lock(&link.mailbox)
            .world_context
            .pending = false;
        outbox.inventory_initialize(Some(Default::default()), false);
        assert!(!view.world_context_choose(&link, &menu, action));
    }
    Ok(())
}

#[test]
fn modifier_mouse_release_reaches_authoritative_look_use_and_attack_callers()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_input_actions::*;
    use oteryn_client_runtime::world_interaction::Action;
    use crate::combat_input::PlayCommand;
    for button in [MouseButton::PRIMARY, MouseButton::SECONDARY] {
        for modifier in [Modifier::Shift, Modifier::Control, Modifier::Alt] {
            let (link, mut commands, outbox) = crate::play::play_channel();
            outbox.world_context_initialize(true,false);
            let mut view = view()?;
            let tile = if modifier == Modifier::Alt {
                let tile = TileCoord::new(101,200);
                view = view.with_entities(vec![WorldSpatialEntity {
                    motion: None,
                    entity: EntityRef { identity: [4;16], generation: 1 }, kind: EntityKind::Creature,
                    position: ActorPosition {x:tile.x,y:tile.y,floor:0}, display_name: None,
                    detail: oteryn_session::EntityDetail::Actor { direction: StepDirection::South, appearance_ref: 1, health_percent: 100 }
                }])?;
                tile
            } else {
                let mut map = super::feature_local_world_view::live_snapshot(101);
                map.tiles[0].items[0].origin = oteryn_session::world_map::MapOrigin::BaseOrdinal { ordinal: 2, object_revision: 7 };
                view.set_live_map(&map)?;
                TileCoord::new(101,200)
            };
            let at = view.scene().view().tile_to_screen(tile);
            let mut router = crate::input::MouseActions::new()?;
            let moved = NormalizedInputEvent::PointerMoved {
                position: PointerPosition::new(PointerCoordinate::new(at[0] as i32 + 24)?,PointerCoordinate::new(at[1] as i32 + 24)?),
                motion: PointerMotion::new(PointerDelta::new(0)?,PointerDelta::new(0)?) };
            let event = |state| NormalizedInputEvent::MouseButton {button,state,modifiers:Modifiers::one(modifier)};
            assert!(router.route(&[moved,event(ButtonState::Pressed)]).is_empty());
            for click in router.route(&[event(ButtonState::Released)]) {
                let tile = view.click_tile(click.x,click.y).ok_or("tile")?;
                assert!(view.modifier_click(tile,&link,click.shortcut.ok_or("shortcut")?), "modifier {modifier:?}, button {button:?}, tile {tile:?}");
            }
            match (modifier,commands.try_recv()?) {
                (Modifier::Shift,PlayCommand::WorldContext(command)) => assert_eq!(command.action,Action::Look),
                (Modifier::Control,PlayCommand::WorldContext(command)) => assert_eq!(command.action,Action::Use),
                (Modifier::Alt,PlayCommand::Combat(_)) => {}
                _ => return Err("modifier command did not reach owner".into()),
            }
        }
    }
    Ok(())
}

#[test]
fn control_click_uses_top_item_under_actor() -> Result<(), Box<dyn std::error::Error>> {
    control_click_cases(&[(false, false, true), (false, true, true)])
}
#[test]
fn control_click_uses_entity_ground_appearance() -> Result<(), Box<dyn std::error::Error>> {
    control_click_cases(&[(true, true, true), (true, false, false)])
}
fn control_click_cases(cases: &[(bool, bool, bool)]) -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_assets::world::{MANIFEST_PATH, MANIFEST_SHA256};
    use oteryn_client_assets::{AppearanceIndex, AssetStore, Catalog};
    use oteryn_input_actions::Modifier;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    if std::env::var_os(crate::map::ASSET_DIR_ENV).as_deref() != Some(root.as_os_str()) {
        let fixture = crate::play::screenshot::TestDir::new()?;
        let output = std::process::Command::new(std::env::current_exe()?)
            .env("TMPDIR", fixture.path())
            .env(crate::map::ASSET_DIR_ENV, &root)
            .arg(std::thread::current().name().ok_or("test thread")?)
            .args(["--exact", "--nocapture"])
            .output()?;
        crate::play::screenshot::assert_one_test_passed(&output);
        return Ok(());
    }
    let store = AssetStore::open_pinned(
        &root.join("content/assets/files"),
        &root.join(MANIFEST_PATH),
        MANIFEST_SHA256,
    )?;
    let catalog = Catalog::load(&store)?;
    let index = AppearanceIndex::load(&store, &catalog)?;
    let multi = index
        .iter()
        .find(|a| a.multi_use)
        .ok_or("multi-use appearance")?
        .id;
    #[cfg(bevy_shell)]
    for scheme in [
        crate::play::MouseScheme::Classic,
        crate::play::MouseScheme::LeftSmart,
    ] {
        let mut world = view()?;
        let mut map = super::feature_local_world_view::live_snapshot(101);
        let tile = &mut map.tiles[0];
        tile.items.truncate(1);
        tile.items[0].origin = oteryn_session::world_map::MapOrigin::BaseOrdinal {
            ordinal: 0,
            object_revision: 0,
        };
        tile.items[0].appearance_id = multi.try_into()?;
        let at = TileCoord::new(tile.position.x, tile.position.y);
        world.set_live_map(&map)?;
        let mut observer = battle_list_creature(oteryn_session::EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        });
        observer.position.x = at.x - 1;
        world.set_presentations(vec![oteryn_session::VisibleAppearance {
            entity: observer,
            look: None,
            light: None,
            forge_stacks: None,
        }])?;
        assert_eq!(
            world.scene().smart_item_action(multi.try_into()?),
            Some(Action::UseWith)
        );
        let (link, mut rx, outbox) = play_channel();
        outbox.world_context_initialize(true, false);
        outbox.targeting_initialize(true);
        assert!(world.direct_click(at, &link, scheme));
        assert!(
            matches!(rx.try_recv()?,crate::combat_input::PlayCommand::WorldContext(cmd) if cmd.action==Action::Use && matches!(cmd.target,Target::Object(_)))
        );
    }
    for &(entity_item, multi_use, map_present) in cases {
        let mut view = view()?;
        let mut map = super::feature_local_world_view::live_snapshot(101);
        map.tiles[0].items.truncate(1);
        map.tiles[0].items[0].origin =
            oteryn_session::world_map::MapOrigin::Handle(std::num::NonZeroU64::MIN);
        if multi_use {
            map.tiles[0].items[0].appearance_id = multi.try_into()?;
        }
        let position = map.tiles[0].position;
        let mut entities = vec![WorldSpatialEntity {
            motion: None,
            entity: EntityRef {
                identity: [4; 16],
                generation: 1,
            },
            kind: EntityKind::Creature,
            position,
            display_name: None,
            detail: oteryn_session::EntityDetail::Actor {
                direction: StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
        }];
        if entity_item {
            entities.clear();
            entities.push(WorldSpatialEntity {
                motion: None,
                entity: EntityRef {
                    identity: [5; 16],
                    generation: 1,
                },
                kind: EntityKind::GroundItem,
                position,
                display_name: None,
                detail: oteryn_session::EntityDetail::Object {
                    item_definition_ref: 1,
                    quantity: 1,
                    item_handle: Some(std::num::NonZeroU64::MIN),
                },
            });
        }
        if map_present {
            view.set_live_map(&map)?;
        }
        view = view.with_entities(entities)?;
        let (link, mut commands, outbox) = play_channel();
        outbox.world_context_initialize(true, false);
        outbox.targeting_initialize(true);
        if !map_present {
            outbox.inventory_initialize(
                Some(oteryn_session::CharacterInventory {
                    entries: vec![oteryn_session::ItemEntry {
                        appearance_id: multi.try_into()?,
                        handle: std::num::NonZeroU64::MIN,
                        item_definition_ref: NonZeroU32::MIN,
                        count: NonZeroU32::MIN,
                        sub_type: 0,
                    }],
                    ..Default::default()
                }),
                false,
            );
        }
        assert!(view.modifier_click(
            TileCoord::new(position.x, position.y),
            &link,
            Modifier::Control
        ));
        if multi_use {
            assert!(link.use_with_armed());
            assert!(commands.try_recv().is_err());
        } else {
            assert!(
                matches!(commands.try_recv()?, crate::combat_input::PlayCommand::WorldContext(command) if command.action == Action::Use && command.target == Target::Item(std::num::NonZeroU64::MIN))
            );
        }
    }
    Ok(())
}

#[cfg(bevy_shell)]
#[test]
fn battle_list_look_sends_the_visible_creature_position() -> Result<(), Box<dyn std::error::Error>>
{
    let (link, mut commands, outbox) = play_channel();
    outbox.world_context_initialize(true, false);
    let entity = battle_list_creature(oteryn_session::EntityDetail::Actor {
        direction: StepDirection::South,
        appearance_ref: 1,
        health_percent: 75,
    });
    let view = view()?.with_entities(vec![entity])?;
    let menu = view
        .creature_context(entity.entity, &link)
        .ok_or("creature menu")?;
    assert!(view.world_context_choose(&link, &menu, Action::Look));
    let crate::combat_input::PlayCommand::WorldContext(command) = commands.try_recv()? else {
        return Err("look command".into());
    };
    assert_eq!(
        command.selection.map(|selection| selection.position),
        Some(entity.position)
    );
    Ok(())
}

#[cfg(bevy_shell)]
fn battle_list_creature(detail: oteryn_session::EntityDetail) -> WorldSpatialEntity {
    WorldSpatialEntity {
        motion: None,
        display_name: None,
        kind: EntityKind::Creature,
        entity: EntityRef {
            identity: [1; 16],
            generation: 1,
        },
        position: ActorPosition {
            x: 101,
            y: 200,
            floor: 0,
        },
        detail,
    }
}

#[cfg(bevy_shell)]
#[test]
fn battle_list_and_health_bar_accept_actors_with_icons() -> Result<(), Box<dyn std::error::Error>> {
    let (link, _, outbox) = play_channel();
    outbox.world_context_initialize(true, false);
    let entity = battle_list_creature(oteryn_session::EntityDetail::ActorWithIcons {
        direction: StepDirection::South,
        appearance_ref: 1,
        health_percent: 75,
        icons: Default::default(),
    });
    let view = view()?.with_entities(vec![entity])?;
    assert!(view.creature_context(entity.entity, &link).is_some());
    assert!(
        view.creature_health_bars()
            .iter()
            .any(|bar| bar.health == 75)
    );
    Ok(())
}

/// G37: a shrine USE never reaches `Dispatch::Use`; it opens the panel and queries the
/// selected item on command 85, the only imbuement route the server serves.
#[test]
fn shrine_use_opens_imbuement_panel_through_command_85() -> Result<(), Box<dyn std::error::Error>>
{
    use oteryn_protocol_oteryn::imbuement as wire;
    fn wire_err(error: impl std::fmt::Debug) -> String {
        format!("{error:?}")
    }
    let shrine_menu = |view: &mut PlayView, link: &crate::play::PlayLink, appearance: u16| {
        let mut map = super::feature_local_world_view::live_snapshot(101);
        map.tiles[0].items[0].origin = oteryn_session::world_map::MapOrigin::BaseOrdinal {
            ordinal: 2,
            object_revision: 7,
        };
        map.tiles[0].items[0].appearance_id = appearance;
        view.set_live_map(&map)?;
        view.world_context(TileCoord::new(101, 200), link)
            .ok_or(BatchError::InvalidView)
    };
    let mut view = view()?;
    let (link, mut commands, outbox) = play_channel();
    outbox.world_context_initialize(true, true);
    outbox.imbue_initialize(true);

    // No item selected: the panel opens and asks for one; nothing goes to the server.
    let menu = shrine_menu(&mut view, &link, 24_964)?;
    assert!(matches!(menu.target, Target::Object(_)));
    assert!(view.world_context_choose(&link, &menu, Action::Use));
    assert!(commands.try_recv().is_err());
    let panel = link.imbue_panel();
    assert!(panel.open);
    assert!(panel.feedback.is_some());
    assert!(link.world_context_feedback().is_none());

    // An item selected: one Query on command 85 that the server decoder accepts.
    let item = oteryn_session::ItemHandle::MIN;
    link.imbue_item(item);
    let menu = shrine_menu(&mut view, &link, 25_202)?;
    assert!(view.world_context_choose(&link, &menu, Action::Use));
    let crate::combat_input::PlayCommand::Imbue(command) = commands.try_recv()? else {
        return Err("shrine USE did not send command 85".into());
    };
    assert!(commands.try_recv().is_err());
    let sent = wire::decode_imbuement_request(&wire::encode_imbuement_request(&command.0).map_err(wire_err)?).map_err(wire_err)?;
    assert_eq!(sent.item, item);
    assert_eq!(sent.action, wire::ImbuementAction::Query);

    // The server's reply bytes, decoded as the session does, fill the open panel.
    let window = wire::ImbuementWindow {
        slot_count: 2,
        applied: Vec::new(),
        offers: Vec::new(),
        clear_gold: 15_000,
    };
    let bytes = wire::encode_imbuement_reply(&wire::ImbuementReply {
        result: wire::ImbuementResult::Ok,
        window: Some(window.clone()),
    })
    .map_err(wire_err)?;
    outbox.imbue_result(&oteryn_client_runtime::imbue::ImbueResponse {
        status: oteryn_session::CommandStatus::Accepted,
        reply: Some(wire::decode_imbuement_reply(&bytes).map_err(wire_err)?),
        success: Vec::new(),
    });
    let panel = link.imbue_panel();
    assert!(panel.open);
    assert_eq!(panel.item, Some(item));
    assert_eq!(panel.window, Some(window));

    // A shrine two tiles away is out of reach: it stays a world USE and opens nothing.
    super::super::mailbox::lock(&outbox.0).imbue.open = false;
    let mut map = super::feature_local_world_view::live_snapshot(101);
    let mut far = map.tiles[0].clone();
    far.position.x += 2;
    far.items[0].origin = oteryn_session::world_map::MapOrigin::BaseOrdinal {
        ordinal: 2,
        object_revision: 7,
    };
    far.items[0].appearance_id = 24_964;
    map.tiles.push(far);
    view.set_live_map(&map)?;
    let menu = view
        .world_context(TileCoord::new(103, 200), &link)
        .ok_or(BatchError::InvalidView)?;
    assert!(view.world_context_choose(&link, &menu, Action::Use));
    assert!(matches!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::WorldContext(
            oteryn_client_runtime::world_interaction::Command { action: Action::Use, .. }
        )
    ));
    assert!(!link.imbue_panel().open);
    super::super::mailbox::lock(&outbox.0).world_context.pending = false;

    // Any other object still goes to the server as a world USE.
    let menu = shrine_menu(&mut view, &link, 25_203)?;
    assert!(view.world_context_choose(&link, &menu, Action::Use));
    assert!(matches!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::WorldContext(
            oteryn_client_runtime::world_interaction::Command { action: Action::Use, .. })
    ));
    assert!(!link.imbue_panel().open);
    Ok(())
}
