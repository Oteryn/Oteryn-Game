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
