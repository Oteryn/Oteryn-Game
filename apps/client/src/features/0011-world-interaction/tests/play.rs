use super::*;
use oteryn_session::EntityKind;

#[test]
fn click_on_empty_ground_becomes_a_step_toward_it() -> Result<(), BatchError> {
    let mut view = view()?;
    view.click(TileCoord::new(98, 200))?;
    assert_eq!(view.next_step(), Some(StepDir::West));
    // One outstanding step: nothing more until its outcome.
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
    outbox.combat(CombatReport::Attack(AttackIntentDisposition::Ok));
    assert_eq!(view.tick(&link), Ok(()));
    assert!(view.combat_status().ends_with("Attacking"));
    Ok(())
}
