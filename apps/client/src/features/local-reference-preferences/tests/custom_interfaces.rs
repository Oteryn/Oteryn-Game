use super::*;

#[test]
fn custom_passive_choices_use_confirmed_action_bar_status_interfaces()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::SettingsState;
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let (link, mut commands, outbox) = play_channel();
    let play = view()?;
    let action = link
        .custom_hotkey_choices()
        .into_iter()
        .find(|(label, _)| label == "Passive: Harmony")
        .map(|(_, action)| action)
        .ok_or("Harmony choice is missing")?;
    let path = std::env::temp_dir().join(format!(
        "cgopt-passive-interface-{}.json",
        std::process::id()
    ));
    let mut settings = SettingsState::new(Some(path.clone()));
    let mut preferences = settings.preferences.clone();
    preferences.assign_custom_hotkey("SUPER+F9", action)?;
    preferences.save_hotkey_profile("Passive tracking")?;
    settings.apply(preferences);
    settings = SettingsState::new(Some(path.clone()));
    let event = NormalizedInputEvent::Key {
        code: KeyCode::new(66)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Super),
        repeat: false,
    };
    assert!(settings.input_in_context(&[event], false).0.is_empty());
    let actions = settings.take_custom_actions();
    assert_eq!(actions.len(), 1);
    assert!(link.custom_hotkey_action(&play, &actions[0]));
    assert!(link.custom_hotkey_status().contains("Unknown"));
    outbox.vitals(Some(oteryn_session::ActorVitals {
        health: 100,
        max_health: 100,
        harmony: 3,
        ..Default::default()
    }));
    assert_eq!(link.custom_hotkey_status(), "Harmony 3/5");
    assert!(link.assign_bar_action(
        35,
        crate::play::BarAction::Passive {
            ability: crate::play::PassiveAbility::Harmony
        }
    ));
    assert!(link.spell_slot_labels()[35].contains(&link.custom_hotkey_status()));
    outbox.vitals(Some(oteryn_session::ActorVitals {
        health: 100,
        max_health: 100,
        harmony: 5,
        ..Default::default()
    }));
    assert_eq!(link.custom_hotkey_status(), "Harmony 5/5");
    assert!(commands.try_recv().is_err());
    outbox.logged_out();
    assert!(link.custom_hotkey_status().is_empty());
    let _ = link.poll();
    assert!(!link.custom_hotkey_action(&play, &actions[0]));
    let (fresh, _, _) = play_channel();
    assert!(fresh.custom_hotkey_status().is_empty());
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn named_custom_profiles_restore_owner_bar_keys_and_mixed_assignments()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{BarAction, PassiveAbility, SettingsState};
    let path =
        std::env::temp_dir().join(format!("cgopt-owner-profiles-{}.json", std::process::id()));
    let (link, _, _) = play_channel();
    let mut settings = SettingsState::new(Some(path.clone()));
    settings.sync_bar_actions(&link);
    assert!(link.assign_bar_action(
        35,
        BarAction::Text {
            text: "First profile".into()
        }
    ));
    assert!(link.bind_action_key(35, "SUPER+F9"));
    settings.sync_bar_actions(&link);
    let mut first = settings.preferences.clone();
    first.save_hotkey_profile("First")?;
    settings.apply(first);
    assert!(link.assign_bar_action(
        35,
        BarAction::Passive {
            ability: PassiveAbility::Harmony
        }
    ));
    assert!(link.bind_action_key(35, "SUPER+F10"));
    settings.sync_bar_actions(&link);
    let mut second = settings.preferences.clone();
    second.save_hotkey_profile("Second")?;
    settings.apply(second);
    settings = SettingsState::new(Some(path.clone()));
    let mut first = settings.preferences.clone();
    first.load_hotkey_profile("First")?;
    assert_eq!(
        first.action_keys.0.get(&35).map(String::as_str),
        Some("SUPER+F9")
    );
    assert!(
        matches!(first.bar_actions.0.get(&35), Some(BarAction::Text { text }) if text == "First profile")
    );
    // A pending older live edit cannot overwrite the explicitly loaded snapshot.
    assert!(link.assign_bar_action(
        35,
        BarAction::Text {
            text: "Pending old edit".into()
        }
    ));
    settings.apply(first);
    settings.sync_bar_actions(&link);
    assert!(link.spell_slot_labels()[35].contains("First profile"));
    assert!(!link.spell_slot_labels()[35].contains("Pending old edit"));
    assert!(link.assign_bar_action(
        35,
        BarAction::Text {
            text: "Queued after selection".into()
        }
    ));
    settings.load_custom_hotkey_profile("First")?;
    settings.sync_bar_actions(&link);
    assert!(link.spell_slot_labels()[35].contains("First profile"));
    assert!(!link.spell_slot_labels()[35].contains("Queued after selection"));
    let mut second = settings.preferences.clone();
    second.load_hotkey_profile("Second")?;
    assert_eq!(
        second.action_keys.0.get(&35).map(String::as_str),
        Some("SUPER+F10")
    );
    assert!(matches!(
        second.bar_actions.0.get(&35),
        Some(BarAction::Passive {
            ability: PassiveAbility::Harmony
        })
    ));
    settings.apply(second);
    settings.sync_bar_actions(&link);
    assert!(link.spell_slot_labels()[35].contains("Harmony"));
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn custom_object_picker_uses_open_container_handles_and_current_combat_generation()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{CustomAction, ObjectTarget};
    use oteryn_client_runtime::world_interaction::{Action, Target};
    use oteryn_session::{
        ActorPosition, EntityDetail, EntityKind, EntityRef, StepDirection, WorldSpatialEntity,
    };
    let (link, mut commands, outbox) = play_channel();
    let play = view()?;
    outbox.targeting_initialize(true);
    outbox.container_initialize(Some(oteryn_session::container_tree::ContainerViews {
        views: vec![oteryn_session::container_tree::ContainerView {
            view_id: 0,
            container_handle: 1u64.try_into()?,
            parent_handle: None,
            capacity: 20,
            browse: None,
            entries: vec![oteryn_session::ItemEntry {
                handle: 2u64.try_into()?,
                item_definition_ref: 900u32.try_into()?,
                count: 1u32.try_into()?,
                sub_type: 7,
            }],
        }],
    }));
    let action = link
        .custom_hotkey_choices()
        .into_iter()
        .find_map(|(_, action)| {
            matches!(
                action,
                CustomAction::Object {
                    definition: 900,
                    sub_type: 7,
                    ..
                }
            )
            .then_some(action)
        })
        .ok_or("open-container item missing")?;
    assert!(link.custom_hotkey_action(&play, &action));
    assert!(
        matches!(commands.try_recv()?, crate::combat_input::PlayCommand::WorldContext(command) if command.target == Target::Item(2u64.try_into()?) && command.action == Action::Use)
    );
    crate::play::mailbox::lock(&link.mailbox)
        .world_context
        .pending = false;
    let target = WorldSpatialEntity {
        display_name: None,
        kind: EntityKind::Creature,
        entity: EntityRef {
            identity: [2; 16],
            generation: 7,
        },
        position: ActorPosition {
            x: 100,
            y: 200,
            floor: 0,
        },
        detail: EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    };
    let combat = oteryn_session::ActorCombatState {
        target: Some(target.entity),
        modes: oteryn_session::FightModes::DEFAULT,
        in_fight: false,
    };
    outbox.battle_publish(Some(combat), Some(0), &[target]);
    let action = CustomAction::Object {
        definition: 900,
        sub_type: 7,
        target: ObjectTarget::CurrentTarget,
    };
    assert!(link.custom_hotkey_action(&play, &action));
    assert!(
        matches!(commands.try_recv()?, crate::combat_input::PlayCommand::WorldContext(command) if command.target == Target::Creature(target.entity) && command.action == Action::UseOn(2u64.try_into()?))
    );
    crate::play::mailbox::lock(&link.mailbox)
        .world_context
        .pending = false;
    let mut replacement = target;
    replacement.entity.generation = 8;
    outbox.battle_publish(Some(combat), Some(0), &[replacement]);
    assert!(!link.custom_hotkey_action(&play, &action));
    assert!(commands.try_recv().is_err());
    outbox.container_end();
    assert!(!link.custom_hotkey_action(
        &play,
        &CustomAction::Object {
            definition: 900,
            sub_type: 7,
            target: ObjectTarget::Use
        }
    ));
    Ok(())
}
