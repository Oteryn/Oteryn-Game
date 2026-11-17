use super::*;
use oteryn_client_runtime::attack::{ActorPosition, EntityDetail, EntityKind, StepDirection};
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};

#[test]
fn projection_replaces_visible_actors_and_rejects_stale_selection() {
    let entity = WorldSpatialEntity {
        display_name: None,
        kind: EntityKind::Creature,
        entity: EntityRef {
            identity: [7; 16],
            generation: 2,
        },
        position: ActorPosition {
            x: 1,
            y: 2,
            floor: 7,
        },
        detail: EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 80,
        },
    };
    let mut panel = BattlePanel::default();
    panel.replace_entities(&[entity], Some(7));
    assert_eq!(panel.rows.len(), 1);
    assert_eq!(request(&mut panel, Action::Attack(entity.entity)), None);
    panel.combat = Some(ActorCombatState {
        target: None,
        modes: FightModes::DEFAULT,
        in_fight: false,
    });
    assert_eq!(
        request(&mut panel, Action::Attack(entity.entity)),
        Some(CombatCommand::Attack(entity.entity))
    );
    panel.pending = false;
    panel.replace_entities(
        &[WorldSpatialEntity {
            kind: EntityKind::Npc,
            ..entity
        }],
        Some(7),
    );
    assert!(panel.rows.is_empty());
    assert_eq!(request(&mut panel, Action::Attack(entity.entity)), None);
    assert_eq!(request(&mut panel, Action::Stop), Some(CombatCommand::Stop));
}

#[test]
fn mode_controls_preserve_other_server_modes() {
    let mut panel = BattlePanel {
        combat: Some(ActorCombatState {
            target: None,
            modes: FightModes::DEFAULT,
            in_fight: false,
        }),
        ..Default::default()
    };
    let modes = FightModes {
        fight_mode: FightMode::Defensive,
        ..FightModes::DEFAULT
    };
    assert_eq!(
        request(&mut panel, Action::Fight(FightMode::Defensive)),
        Some(CombatCommand::Modes(modes))
    );
    assert_eq!(panel.combat.map(|s| s.modes), Some(FightModes::DEFAULT));
    panel.pending = false;
    panel.combat = Some(ActorCombatState {
        target: None,
        modes,
        in_fight: false,
    });
    assert_eq!(
        request(&mut panel, Action::Chase(ChaseMode::Chase)),
        Some(CombatCommand::Modes(FightModes {
            chase: ChaseMode::Chase,
            ..modes
        }))
    );
}

#[test]
fn alt_b_toggles_without_claiming_inventory_or_repeats() -> Result<(), Box<dyn std::error::Error>> {
    let event = |code, modifiers, repeat| NormalizedInputEvent::Key {
        code,
        modifiers,
        repeat,
        state: ButtonState::Pressed,
    };
    assert!(toggle_key(&[event(
        KeyCode::new(5)?,
        Modifiers::one(Modifier::Alt),
        false
    )]));
    assert!(!toggle_key(&[event(
        KeyCode::new(5)?,
        Modifiers::one(Modifier::Alt),
        true
    )]));
    assert!(!toggle_key(&[event(
        KeyCode::new(12)?,
        Modifiers::one(Modifier::Control),
        false
    )]));
    Ok(())
}

#[test]
fn mode_input_waits_for_the_previous_server_answer() {
    let mut panel = BattlePanel {
        combat: Some(ActorCombatState {
            target: None,
            modes: FightModes::DEFAULT,
            in_fight: false,
        }),
        ..Default::default()
    };
    assert!(request(&mut panel, Action::Fight(FightMode::Offensive)).is_some());
    assert_eq!(request(&mut panel, Action::Chase(ChaseMode::Chase)), None);
}

#[test]
fn battle_rows_follow_the_own_actors_floor() {
    let creature = WorldSpatialEntity {
        display_name: None,
        kind: EntityKind::Creature,
        entity: EntityRef {
            identity: [9; 16],
            generation: 1,
        },
        position: ActorPosition {
            x: 10,
            y: 20,
            floor: 7,
        },
        detail: EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    };
    let upstairs = WorldSpatialEntity {
        display_name: None,
        entity: EntityRef {
            identity: [10; 16],
            generation: 1,
        },
        position: ActorPosition {
            floor: 8,
            ..creature.position
        },
        ..creature
    };
    let mut panel = BattlePanel::default();
    panel.replace_entities(&[creature, upstairs], Some(7));
    assert_eq!(panel.rows, vec![creature]);
    panel.replace_entities(&[creature, upstairs], Some(8));
    assert_eq!(panel.rows, vec![upstairs]);
    panel.replace_entities(&[creature, upstairs], None);
    assert!(panel.rows.is_empty());
}

#[path = "tests/metadata.rs"]
mod metadata;
