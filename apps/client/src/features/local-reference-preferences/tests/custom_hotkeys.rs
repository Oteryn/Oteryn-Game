use super::*;

#[test]
fn custom_hotkeys_suppress_builtin_input_and_survive_profiles()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{ReferencePreferences, SettingsState};
    use oteryn_input_actions::{ButtonState, KeyCode, Modifiers, NormalizedInputEvent};
    let mut json = serde_json::to_value(ReferencePreferences::default())?;
    json["custom_hotkeys"] =
        serde_json::json!({"F9": {"kind": "text", "text": "hello", "send": true}});
    let mut settings = SettingsState::new(None);
    settings.preferences = serde_json::from_value(json)?;
    let event = NormalizedInputEvent::Key {
        code: KeyCode::new(66)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::NONE,
        repeat: false,
    };
    assert!(settings.input(&[event]).0.is_empty());
    settings.preferences.save_hotkey_profile("custom")?;
    let saved = serde_json::to_vec(&settings.preferences)?;
    let mut restarted: ReferencePreferences = serde_json::from_slice(&saved)?;
    restarted.custom_hotkeys.clear();
    restarted.load_hotkey_profile("custom")?;
    assert_eq!(
        serde_json::to_value(&restarted)?["custom_hotkeys"],
        serde_json::to_value(&settings.preferences)?["custom_hotkeys"]
    );
    Ok(())
}

#[test]
fn hotkey_use_with_crosshair_arms_mode()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::combat_input::PlayCommand;
    use crate::play::{CustomAction, ObjectTarget, SettingsState};
    use oteryn_client_runtime::world_interaction::{Action, Target};
    use oteryn_input_actions::{ButtonState, KeyCode, Modifiers, NormalizedInputEvent};
    let (link, mut commands, outbox) = play_channel();
    let mut play = view()?;
    outbox.chat_initialize(true);
    let mut settings = SettingsState::new(None);
    settings.preferences.assign_custom_hotkey(
        "F9",
        CustomAction::Text {
            text: "hello".into(),
            send: true,
        },
    )?;
    let press = NormalizedInputEvent::Key {
        code: KeyCode::new(66)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::NONE,
        repeat: false,
    };
    assert!(settings.input(std::slice::from_ref(&press)).0.is_empty());
    let actions = settings.take_custom_actions();
    assert_eq!(actions.len(), 1);
    assert!(link.custom_hotkey_action(&play, &actions[0]));
    assert!(
        matches!(commands.try_recv()?, PlayCommand::ChatPublic(oteryn_session::ChatIntent::Say { text, .. }) if text == "hello")
    );
    settings.input(&[press]);
    assert!(settings.take_custom_actions().is_empty());
    let release = NormalizedInputEvent::Key {
        code: KeyCode::new(66)?,
        state: ButtonState::Released,
        modifiers: Modifiers::NONE,
        repeat: false,
    };
    settings.input(&[release]);
    assert!(settings.take_custom_actions().is_empty());
    let before = settings.preferences.clone();
    assert!(
        settings
            .preferences
            .assign_custom_hotkey("1", actions[0].clone())
            .is_err()
    );
    assert_eq!(settings.preferences, before);
    crate::play::mailbox::lock(&link.mailbox).chat.pending = false;
    assert!(link.custom_hotkey_action(
        &play,
        &CustomAction::Text {
            text: "draft".into(),
            send: false
        }
    ));
    assert_eq!(
        crate::play::mailbox::lock(&link.mailbox)
            .custom_chat_paste
            .as_deref(),
        Some("draft")
    );
    assert!(commands.try_recv().is_err());
    let item = |handle: u64| -> Result<oteryn_session::ItemEntry, Box<dyn std::error::Error>> {
        Ok(oteryn_session::ItemEntry {
            appearance_id: 0,
            handle: handle.try_into()?,
            item_definition_ref: 900u32.try_into()?,
            count: 1u32.try_into()?,
            sub_type: 7,
        })
    };
    let inventory = |item| oteryn_session::CharacterInventory {
        main_backpack: None,
        entries: vec![item],
        equipment: vec![],
    };
    outbox.inventory_initialize(Some(inventory(item(1)?)), false);
    outbox.targeting_initialize(true);
    let use_item = CustomAction::Object {
        definition: 900,
        sub_type: 7,
        target: ObjectTarget::Use,
    };
    assert!(link.custom_hotkey_action(&play, &use_item));
    assert!(
        matches!(commands.try_recv()?, PlayCommand::WorldContext(command) if command.action == Action::Use && command.target == Target::Item(1u64.try_into()?))
    );
    crate::play::mailbox::lock(&link.mailbox)
        .world_context
        .pending = false;
    outbox.inventory_initialize(Some(inventory(item(99)?)), false);
    let own = oteryn_session::WorldSpatialEntity {
        motion: None,
        display_name: None,
        kind: oteryn_session::EntityKind::Player,
        entity: oteryn_session::EntityRef {
            identity: [3; 16],
            generation: 1,
        },
        position: oteryn_session::ActorPosition {
            x: play.own().x,
            y: play.own().y,
            floor: play.map_center().2,
        },
        detail: oteryn_session::EntityDetail::Actor {
            direction: oteryn_session::StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    };
    // An open ground bag may expose the same object definition, but cannot supply the shortcut.
    outbox.container_initialize(Some(oteryn_session::container_tree::ContainerViews {views: vec![
        oteryn_session::container_tree::ContainerView {view_id:4,container_handle:20.try_into()?,
            parent_handle:None, capacity:20, entries:vec![item(2)?], browse:None}
    ]}));
    let position = own.position;
    crate::play::mailbox::lock(&link.mailbox).bar_actions.cursor = Some((
        Target::Position(position),
        oteryn_client_runtime::world_interaction::Selection { position, binding: None },
    ));
    let immediate: CustomAction = serde_json::from_str(
        r#"{"kind":"object","definition":900,"sub_type":7,"target":"cursor_position"}"#,
    )?;
    assert!(link.custom_hotkey_action(&play, &immediate));
    assert!(matches!(commands.try_recv()?, PlayCommand::WorldContext(command)
        if command.target == Target::Position(position) && command.action == Action::UseOn(99u64.try_into()?)));
    assert!(!link.use_with_armed());
    outbox.container_initialize(Some(Default::default()));
    crate::play::mailbox::lock(&link.mailbox).world_context.pending = false;
    outbox.hotkey_own_actor(Some(own));
    assert!(link.custom_hotkey_action(
        &play,
        &CustomAction::Object {
            definition: 900,
            sub_type: 7,
            target: ObjectTarget::SelfActor
        }
    ));
    assert!(
        matches!(commands.try_recv()?, PlayCommand::WorldContext(command) if command.action == Action::UseOn(99u64.try_into()?) && command.target == Target::Creature(own.entity))
    );
    crate::play::mailbox::lock(&link.mailbox)
        .world_context
        .pending = false;
    assert!(link.custom_hotkey_action(
        &play,
        &CustomAction::Object {
            definition: 900,
            sub_type: 7,
            target: ObjectTarget::Cursor
        }
    ));
    assert!(link.use_with_armed());
    play.primary_click(oteryn_renderer::TileCoord::new(98, 200), &link)?;
    assert!(
        matches!(commands.try_recv()?, PlayCommand::WorldContext(command) if command.action == Action::UseOn(99u64.try_into()?))
    );
    crate::play::mailbox::lock(&link.mailbox)
        .world_context
        .pending = false;
    outbox.inventory_initialize(None, false);
    assert!(!link.custom_hotkey_action(&play, &use_item));
    assert!(commands.try_recv().is_err());
    outbox.hotkey_equip_initialize(true);
    outbox.hotkey_equip_items(vec![900]);
    assert!(link.custom_hotkey_action(
        &play,
        &CustomAction::Object {
            definition: 900,
            sub_type: 0,
            target: ObjectTarget::Equip
        }
    ));
    assert!(
        matches!(commands.try_recv()?, PlayCommand::ServerMechanic(command) if command.capability == 179 && command.command_type == 348 && command.payload == 900u32.to_be_bytes())
    );
    use oteryn_session::spell_catalog::{SpellCatalogEntry, encode_catalog};
    let entry = SpellCatalogEntry {
        cast_index: 19,
        name: "Ability".into(),
        words: "ability".into(),
        level: 1,
        mana: 0,
        mana_percent: false,
        soul: 0,
        learned: false,
        available: false,
        cooldown_groups: vec![1],
        rune_item: None,
    };
    outbox.spell_catalog_initialize(
        true,
        Some(encode_catalog(&[entry]).map_err(|e| format!("{e:?}"))?),
    )?;
    let passive = CustomAction::Passive { cast_index: 19 };
    assert!(link.custom_hotkey_action(&play, &passive));
    assert_eq!(link.custom_hotkey_status(), "Ability: cooldown unknown");
    let received_at = std::time::Instant::now() + std::time::Duration::from_secs(1);
    outbox.push(vec![oteryn_session::SessionEvent::ActorCooldowns(
        oteryn_session::AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value: oteryn_session::ActorCooldownUpdate {
                value: oteryn_session::ActorCooldowns {
                    server_now_ms: 100,
                    rtt_ms: None,
                    entries: vec![oteryn_session::ActorCooldown {
                        kind: oteryn_session::ActorCooldownKind::Group,
                        id: 1,
                        expires_at_ms: 500,
                    }],
                },
                received_at,
                snapshot: true,
            },
        },
    )]);
    outbox.push(vec![oteryn_session::SessionEvent::ActorCooldowns(
        oteryn_session::AppliedDelta {
            server_sequence: 2,
            base_revision: 1,
            new_revision: 2,
            value: oteryn_session::ActorCooldownUpdate {
                value: oteryn_session::ActorCooldowns {
                    server_now_ms: 0,
                    rtt_ms: None,
                    entries: vec![oteryn_session::ActorCooldown {
                        kind: oteryn_session::ActorCooldownKind::Spell,
                        id: 99,
                        expires_at_ms: 1000,
                    }],
                },
                received_at,
                snapshot: false,
            },
        },
    )]);
    assert_eq!(
        link.custom_hotkey_status(),
        "Ability: server cooldown 400 ms"
    );
    assert!(commands.try_recv().is_err()); // Tracking never executes an unavailable ability.
    outbox.logged_out();
    assert!(!link.custom_hotkey_action(&play, &passive));
    let (fresh, _, _) = play_channel();
    assert!(fresh.custom_hotkey_status().is_empty());
    Ok(())
}

#[path = "custom_interfaces.rs"]
mod custom_interfaces;

#[test]
fn hotkey_use_with_label() {
    let action = crate::play::CustomAction::Object { definition: 900, sub_type: 7, target: crate::play::ObjectTarget::Cursor };
    assert_eq!(action.hotkey_label("F1"), "F1: (use object with crosshair)");
}

#[test]
fn action_bar_and_hotkeys_dispatch_combat_and_chat_shortcuts()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{BarAction, CustomAction, mailbox::lock};
    use crate::combat_input::CombatCommand;
    use oteryn_session::{ActorCombatState, FightModes};
    let (link, mut commands, outbox) = play_channel();
    let view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(10,10), 7, &[])?;
    let a = oteryn_session::WorldSpatialEntity {
        motion: None, display_name: None, kind: oteryn_session::EntityKind::Creature,
        entity: oteryn_session::EntityRef { identity: [1;16], generation: 1 },
        position: oteryn_session::ActorPosition { x:10, y:11, floor:7 },
        detail: oteryn_session::EntityDetail::Actor { direction: oteryn_session::StepDirection::South, appearance_ref: 1, health_percent: 100 },
    };
    let b = oteryn_session::WorldSpatialEntity { entity: oteryn_session::EntityRef {identity:[2;16],generation:1}, ..a };
    // The main battle list defines the cycle even while a filtered secondary list is open.
    {
        let mut mailbox = lock(&link.mailbox);
        let mut secondary = mailbox.battle_lists.preferences[0].clone();
        secondary.creatures = false;
        mailbox.battle_lists.preferences.push(secondary);
        mailbox.battle_lists.active = 1;
    }
    for kind in ["attack_next", "attack_previous", "toggle_chase", "toggle_chat"] {
        for bar in [false,true] {
            let combat = ActorCombatState { target: Some(a.entity), modes: FightModes::DEFAULT, in_fight: true };
            outbox.battle_publish(Some(combat),Some(7), &[a,b]);
            lock(&link.mailbox).battle.pending = false;
            let before = lock(&link.mailbox).chat.console.mode;
            if bar {
                let action: BarAction = serde_json::from_value(serde_json::json!({"kind":"Special","action":kind}))?;
                assert!(link.assign_bar_action(0,action));
                link.cast_spell_slot(std::num::NonZeroU32::MIN);
            } else {
                let action: CustomAction = serde_json::from_value(serde_json::json!({"kind":"special","action":kind}))?;
                assert!(link.custom_hotkey_action(&view,&action));
            }
            match kind {
                "toggle_chat" => assert_ne!(lock(&link.mailbox).chat.console.mode,before),
                "toggle_chase" => assert!(matches!(commands.try_recv()?, crate::combat_input::PlayCommand::Battle(CombatCommand::Modes(m)) if m.chase != combat.modes.chase)),
                _ => assert!(matches!(commands.try_recv()?, crate::combat_input::PlayCommand::Battle(CombatCommand::Attack(e)) if e == b.entity)),
            }
        }
    }
    Ok(())
}
