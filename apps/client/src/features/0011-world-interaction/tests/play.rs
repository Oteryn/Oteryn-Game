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
    // The NPC is still drawn: player plus one glyph per visible entity.
    assert_eq!(view.scene().sprites().len(), 2 + 2);
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
    outbox.battle_result(
        CombatReport::Attack(AttackIntentDisposition::Ok),
        None,
        None,
    );
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

fn load_rotate_appearances(
    view: &mut PlayView,
    map: &oteryn_session::world_map::WorldMapViewSnapshot,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let world = crate::map::World::resolve_live_map(&root, map)?;
    view.local.set_world(std::sync::Arc::new(world));
    view.rebuild()?;
    Ok(())
}

#[test]
fn srv7_native_world_context_exposes_explicit_rotate_on_live_item()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut view = view()?;
    let mut map = super::feature_local_world_view::live_snapshot(101);
    map.tiles[0].items[0].origin =
        oteryn_session::world_map::MapOrigin::Handle(std::num::NonZeroU64::MIN);
    map.tiles[0].items[0].appearance_id = 2025; // Official flag 22 is true.
    view.set_live_map(&map)?;
    load_rotate_appearances(&mut view, &map)?;
    let (link, mut commands, outbox) = play_channel();
    outbox.world_context_initialize(true, true);
    let old_menu = view
        .world_context(TileCoord::new(101, 200), &link)
        .ok_or(BatchError::InvalidView)?;
    assert!(!old_menu.entries.iter().any(|(label, _)| *label == "Rotate"));
    outbox.rotation_initialize(true);
    let menu = view
        .world_context(TileCoord::new(101, 200), &link)
        .ok_or(BatchError::InvalidView)?;
    assert!(menu.entries.iter().any(|(label, _)| *label == "Rotate"));
    assert!(view.world_context_choose(
        &link,
        &menu,
        Action::Rotate(Some(std::num::NonZeroU32::MIN))
    ));
    assert!(matches!(
        commands.try_recv(),
        Ok(crate::combat_input::PlayCommand::WorldContext(
            oteryn_client_runtime::world_interaction::Command {
                action: Action::Rotate(_),
                ..
            }
        ))
    ));
    super::super::mailbox::lock(&outbox.0).world_context.pending = false;
    map.tiles[0].items[0].definition = oteryn_session::world_map::MapDefinition::Item(
        std::num::NonZeroU32::new(2).ok_or("definition")?,
    );
    view.set_live_map(&map)?;
    load_rotate_appearances(&mut view, &map)?;
    assert!(!view.world_context_choose(
        &link,
        &menu,
        Action::Rotate(Some(std::num::NonZeroU32::MIN))
    ));
    assert!(commands.try_recv().is_err());
    map.tiles[0].items[0].definition =
        oteryn_session::world_map::MapDefinition::Item(std::num::NonZeroU32::MIN);
    map.header.reset_epoch += 1;
    view.set_live_map(&map)?;
    load_rotate_appearances(&mut view, &map)?;
    assert!(!view.world_context_choose(
        &link,
        &menu,
        Action::Rotate(Some(std::num::NonZeroU32::MIN))
    ));
    assert!(commands.try_recv().is_err());
    map.tiles[0].items[0].appearance_id = 100; // Official ground has no Rotate affordance.
    view.set_live_map(&map)?;
    load_rotate_appearances(&mut view, &map)?;
    let wall = view
        .world_context(TileCoord::new(101, 200), &link)
        .ok_or("nonrotatable menu")?;
    assert!(!wall.entries.iter().any(|(label, _)| *label == "Rotate"));
    // Bundle placements use their revision-CAS target, not ItemHandle metadata.
    map.tiles[0].items[0].appearance_id = 2025;
    map.tiles[0].items[0].origin = oteryn_session::world_map::MapOrigin::BaseOrdinal {
        ordinal: 1,
        object_revision: 1,
    };
    view.set_live_map(&map)?;
    load_rotate_appearances(&mut view, &map)?;
    let base = view
        .world_context(TileCoord::new(101, 200), &link)
        .ok_or("base Rotate menu")?;
    assert!(
        base.entries
            .iter()
            .any(|(label, action)| *label == "Rotate" && *action == Action::Rotate(None))
    );
    assert!(view.world_context_choose(&link, &base, Action::Rotate(None)));
    let crate::combat_input::PlayCommand::WorldContext(command) = commands.try_recv()? else {
        return Err("base Rotate command".into());
    };
    assert!(matches!(command.target, Target::Object(_)));
    assert_eq!(command.action, Action::Rotate(None));
    Ok(())
}

#[test]
fn srv7_native_rotate_session_loop_shows_server_completion()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            use crate::play::{Stepper, run_session};
            use oteryn_client_runtime::world_interaction::{Command, rotate_payload, world_rotate};
            struct Rotator;
            impl Stepper for Rotator {
                async fn step(&mut self, _: StepDirection) -> Result<StepOutcome, SessionError> {
                    Err(SessionError::SessionUnusable)
                }
                async fn serve(&mut self, _: Duration) -> Result<(), SessionError> {
                    Err(SessionError::SessionUnusable)
                }
                fn drain(&mut self) -> Vec<SessionEvent> {
                    Vec::new()
                }
                async fn combat(&mut self, _: CombatCommand) -> Result<CombatReport, SessionError> {
                    Err(SessionError::SessionUnusable)
                }
                async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
                    Err(SessionError::SessionUnusable)
                }
                fn feature_selected(&self, capability: u32) -> bool {
                    matches!(capability, 4 | 6 | 11 | 15 | 18 | 388)
                }
                async fn world_context(
                    &mut self,
                    command: &Command,
                ) -> Result<String, SessionError> {
                    assert_eq!(
                        command.action,
                        Action::Rotate(Some(std::num::NonZeroU32::MIN))
                    );
                    let payload = rotate_payload(&command.target, Some(std::num::NonZeroU32::MIN))?;
                    let (intent, observed) = world_rotate::decode_rotate_intent(&payload)
                        .map_err(|_| SessionError::SessionUnusable)?;
                    assert_eq!(observed, Some(std::num::NonZeroU32::MIN));
                    assert_eq!(
                        intent.target,
                        world_rotate::UseTarget::Item(std::num::NonZeroU64::MIN)
                    );
                    assert_eq!(
                        (
                            world_rotate::CAPABILITY_WORLD_ROTATE_V1,
                            world_rotate::COMMAND_TYPE_ROTATE_WORLD_ITEM
                        ),
                        (388, 670)
                    );
                    let payload = world_rotate::encode_rotate_result(
                        world_rotate::UseDisposition::Committed,
                        None,
                    )
                    .map_err(|_| SessionError::SessionUnusable)?;
                    let (disposition, _) = world_rotate::decode_rotate_result(&payload)
                        .map_err(|_| SessionError::SessionUnusable)?;
                    assert_eq!(disposition, world_rotate::UseDisposition::Committed);
                    Ok("Rotate: completed".into())
                }
            }
            let mut view = view()?;
            let mut map = super::feature_local_world_view::live_snapshot(101);
            map.tiles[0].items[0].origin =
                oteryn_session::world_map::MapOrigin::Handle(std::num::NonZeroU64::MIN);
            map.tiles[0].items[0].appearance_id = 2025;
            view.set_live_map(&map)?;
            load_rotate_appearances(&mut view, &map)?;
            let (link, commands, outbox) = play_channel();
            outbox.rotation_initialize(true);
            let menu = view
                .world_context(TileCoord::new(101, 200), &link)
                .ok_or("rotate menu")?;
            assert!(view.world_context_choose(
                &link,
                &menu,
                Action::Rotate(Some(std::num::NonZeroU32::MIN))
            ));
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(run_session(Rotator, commands, outbox));
            assert_eq!(
                link.world_context_feedback().as_deref(),
                Some("Rotate: completed")
            );
            Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
        })?
        .join()
        .map_err(|_| "rotate native session-loop test thread panicked")?
}

#[test]
fn srv7_native_carried_rotate_requires_current_advisory_probe()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use oteryn_client_runtime::raw_feature::GatedCommandResult;
    use oteryn_client_runtime::world_interaction::world_rotate;
    let item = oteryn_session::ItemEntry {
        handle: std::num::NonZeroU64::MIN,
        item_definition_ref: std::num::NonZeroU32::MIN,
        count: std::num::NonZeroU32::MIN,
        sub_type: 0,
    };
    let inventory = oteryn_session::CharacterInventory {
        entries: vec![item],
        ..Default::default()
    };
    let mut map = super::feature_local_world_view::live_snapshot(100);
    let (link, mut commands, outbox) = play_channel();
    outbox.rotation_initialize(true);
    outbox.inventory_initialize(Some(inventory.clone()), true);
    outbox.map_initialize(Some(
        oteryn_session::world_map::encode_world_map_snapshot(&map).map_err(|e| format!("{e:?}"))?,
    ));
    assert!(
        super::super::mailbox::lock(&outbox.0)
            .inventory
            .select(item.handle)
    );
    assert!(!crate::play::PlayLink::inventory_rotate_shared(
        &link.mailbox,
        &link.commands
    ));
    assert!(crate::play::PlayLink::inventory_rotate_probe_shared(
        &link.mailbox,
        &link.commands
    ));
    let crate::combat_input::PlayCommand::Feature(query) = commands.try_recv()? else {
        return Err("inventory affordance query".into());
    };
    let request =
        world_rotate::decode_rotate_request(&query.payload).map_err(|e| format!("{e:?}"))?;
    assert!(request.probe);
    assert_eq!(request.observed_definition, Some(item.item_definition_ref));
    let reply = GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: oteryn_session::CommandStatus::Accepted,
        payload: world_rotate::encode_rotate_affordance(true),
    };
    outbox.rotation_probe_result(&reply);
    assert!(crate::play::PlayLink::inventory_rotate_shared(
        &link.mailbox,
        &link.commands
    ));
    let crate::combat_input::PlayCommand::WorldContext(command) = commands.try_recv()? else {
        return Err("carried Rotate action".into());
    };
    assert_eq!(command.target, Target::Item(item.handle));
    assert_eq!(
        command.action,
        Action::Rotate(Some(item.item_definition_ref))
    );
    assert!(command.selection.is_none());
    super::super::mailbox::lock(&outbox.0).world_context.pending = false;
    // A revision invalidates even the same selected handle and definition.
    outbox.inventory_events(&[SessionEvent::Inventory(AppliedDelta {
        server_sequence: 2,
        base_revision: 1,
        new_revision: 2,
        value: inventory,
    })]);
    assert!(!crate::play::PlayLink::inventory_rotate_shared(
        &link.mailbox,
        &link.commands
    ));
    assert!(crate::play::PlayLink::inventory_rotate_probe_shared(
        &link.mailbox,
        &link.commands
    ));
    assert!(matches!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::Feature(_)
    ));
    map.header.reset_epoch += 1;
    outbox.map_initialize(Some(
        oteryn_session::world_map::encode_world_map_snapshot(&map).map_err(|e| format!("{e:?}"))?,
    ));
    outbox.rotation_probe_result(&reply);
    assert!(!crate::play::PlayLink::inventory_rotate_shared(
        &link.mailbox,
        &link.commands
    ));
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn srv7_native_carried_rotate_probe_and_completion_cross_session_loop()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            use crate::play::{PlayLink, Stepper, run_session};
            use oteryn_client_runtime::raw_feature::{FeatureCommand, GatedCommandResult};
            use oteryn_client_runtime::world_interaction::{Command, rotate_payload, world_rotate};
            struct CarriedRotator {
                shared: crate::play::mailbox::SharedMailbox,
                commands: tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
                inventory: oteryn_session::CharacterInventory,
                map: Vec<u8>,
                phase: u8,
                completed: std::sync::Arc<std::sync::atomic::AtomicBool>,
            }
            impl Stepper for CarriedRotator {
                async fn step(&mut self, _: StepDirection) -> Result<StepOutcome, SessionError> {
                    Err(SessionError::SessionUnusable)
                }
                async fn serve(&mut self, _: Duration) -> Result<(), SessionError> {
                    match self.phase {
                        0 => {
                            assert!(
                                crate::play::mailbox::lock(&self.shared)
                                    .inventory
                                    .select(std::num::NonZeroU64::MIN)
                            );
                            assert!(!PlayLink::inventory_rotate_shared(
                                &self.shared,
                                &self.commands
                            ));
                            assert!(PlayLink::inventory_rotate_probe_shared(
                                &self.shared,
                                &self.commands
                            ));
                            self.phase = 1;
                        }
                        1 => {
                            assert!(PlayLink::inventory_rotate_shared(
                                &self.shared,
                                &self.commands
                            ));
                            self.phase = 2;
                        }
                        _ => return Err(SessionError::SessionUnusable),
                    }
                    Ok(())
                }
                fn drain(&mut self) -> Vec<SessionEvent> {
                    Vec::new()
                }
                async fn combat(&mut self, _: CombatCommand) -> Result<CombatReport, SessionError> {
                    Err(SessionError::SessionUnusable)
                }
                async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
                    Err(SessionError::SessionUnusable)
                }
                fn inventory(&self) -> Option<oteryn_session::CharacterInventory> {
                    Some(self.inventory.clone())
                }
                fn feature_snapshot(&self, domain: u32) -> Option<Vec<u8>> {
                    (domain == 17).then(|| self.map.clone())
                }
                fn feature_selected(&self, capability: u32) -> bool {
                    matches!(capability, 4 | 6 | 11 | 15 | 18 | 388)
                }
                async fn feature(
                    &mut self,
                    command: &FeatureCommand,
                ) -> Result<GatedCommandResult, SessionError> {
                    assert_eq!((command.capability, command.kind), (388, 670));
                    let request = world_rotate::decode_rotate_request(&command.payload)
                        .map_err(|_| SessionError::SessionUnusable)?;
                    assert!(request.probe);
                    assert_eq!(
                        request.intent.target,
                        world_rotate::UseTarget::Item(std::num::NonZeroU64::MIN)
                    );
                    assert_eq!(request.observed_definition, Some(std::num::NonZeroU32::MIN));
                    Ok(GatedCommandResult {
                        command_id: 1,
                        server_sequence: 1,
                        status: oteryn_session::CommandStatus::Accepted,
                        payload: world_rotate::encode_rotate_affordance(true),
                    })
                }
                async fn world_context(
                    &mut self,
                    command: &Command,
                ) -> Result<String, SessionError> {
                    assert!(command.selection.is_none());
                    let payload = rotate_payload(&command.target, Some(std::num::NonZeroU32::MIN))?;
                    let request = world_rotate::decode_rotate_request(&payload)
                        .map_err(|_| SessionError::SessionUnusable)?;
                    assert!(!request.probe);
                    assert_eq!(request.observed_definition, Some(std::num::NonZeroU32::MIN));
                    let payload = world_rotate::encode_rotate_result(
                        world_rotate::UseDisposition::Committed,
                        None,
                    )
                    .map_err(|_| SessionError::SessionUnusable)?;
                    let (disposition, _) = world_rotate::decode_rotate_result(&payload)
                        .map_err(|_| SessionError::SessionUnusable)?;
                    assert_eq!(disposition, world_rotate::UseDisposition::Committed);
                    self.completed
                        .store(true, std::sync::atomic::Ordering::SeqCst);
                    Ok("Rotate: completed".into())
                }
            }
            let item = oteryn_session::ItemEntry {
                handle: std::num::NonZeroU64::MIN,
                item_definition_ref: std::num::NonZeroU32::MIN,
                count: std::num::NonZeroU32::MIN,
                sub_type: 0,
            };
            let inventory = oteryn_session::CharacterInventory {
                entries: vec![item],
                ..Default::default()
            };
            let map = super::feature_local_world_view::live_snapshot(100);
            let map = oteryn_session::world_map::encode_world_map_snapshot(&map)
                .map_err(|e| format!("{e:?}"))?;
            let (link, commands, outbox) = play_channel();
            let completed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let session = CarriedRotator {
                shared: link.mailbox.clone(),
                commands: link.commands.clone(),
                inventory,
                map,
                phase: 0,
                completed: completed.clone(),
            };
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(run_session(session, commands, outbox));
            assert!(completed.load(std::sync::atomic::Ordering::SeqCst));
            assert_eq!(
                link.world_context_feedback().as_deref(),
                Some("Rotate: completed")
            );
            Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
        })?
        .join()
        .map_err(|_| "carried Rotate session-loop test thread panicked")?
}
