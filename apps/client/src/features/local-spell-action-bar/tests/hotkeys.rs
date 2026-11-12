use crate::play::{BarAction, ItemUseMode, SettingsState, play_channel};
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
use std::num::{NonZeroU32, NonZeroU64};

#[test]
fn custom_text_keys_persist_and_do_not_repeat_or_steal_chat_letters()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = play_channel();
    outbox.chat_initialize(true);
    let path = std::env::temp_dir().join(format!("custom-keys-{}.json", std::process::id()));
    let mut settings = SettingsState::new(Some(path.clone()));
    settings.sync_bar_actions(&link);
    assert!(link.assign_bar_action(
        0,
        BarAction::Text {
            text: "A saved greeting".into(),
            send_automatically: true
        }
    ));
    assert!(link.bind_action_key(0, "SUPER+ALT+F12"));
    assert!(!link.bind_action_key(1, "ALT+SUPER+F12"));
    assert!(!link.bind_action_key(1, "ESCAPE"));
    let event = NormalizedInputEvent::Key {
        code: KeyCode::new(69)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Super).with(Modifier::Alt),
        repeat: false,
    };
    assert!(link.custom_hotkeys_input(std::slice::from_ref(&event)));
    assert!(matches!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::ChatPublic(_)
    ));
    let mut repeat = event;
    if let NormalizedInputEvent::Key { repeat, .. } = &mut repeat {
        *repeat = true;
    }
    assert!(!link.custom_hotkeys_input(&[repeat]));
    assert!(commands.try_recv().is_err());
    assert!(link.bind_action_key(1, "A"));
    crate::play::mailbox::lock(&link.mailbox).chat.open = true;
    assert!(!link.custom_hotkeys_input(&[NormalizedInputEvent::Key {
        code: KeyCode::new(4)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::NONE,
        repeat: false
    }]));
    settings.sync_bar_actions(&link);
    assert_eq!(
        SettingsState::new(Some(path.clone()))
            .preferences
            .action_keys,
        settings.preferences.action_keys
    );
    let original = crate::play::preference_bindings()?
        .into_iter()
        .find(|binding| binding.id == "0072-task-board-soulseals")
        .ok_or("binding")?
        .default_chord;
    settings
        .preferences
        .rebind("0072-task-board-soulseals", "SUPER+ALT+F10")?;
    settings.sync_bar_actions(&link);
    assert!(
        !link.bind_action_key(2, original),
        "translated default keys cannot also become custom keys"
    );
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn item_hotkeys_resolve_live_handles_for_self_target_and_cursor()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::world_interaction::{Action, Target};
    use oteryn_session::{
        ActorPosition, EntityDetail, EntityKind, EntityRef, StepDirection, WorldSpatialEntity,
    };
    let (link, mut commands, outbox) = play_channel();
    outbox.targeting_initialize(true);
    let actor = |id| WorldSpatialEntity {
        display_name: None,
        kind: EntityKind::Player,
        entity: EntityRef {
            identity: [id; 16],
            generation: 1,
        },
        position: ActorPosition {
            x: 10 + i32::from(id),
            y: 20,
            floor: 7,
        },
        detail: EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    };
    let own = actor(1);
    let target = actor(2);
    outbox.hotkey_own_actor(Some(own));
    outbox.battle_publish(
        Some(oteryn_session::ActorCombatState {
            target: Some(target.entity),
            modes: oteryn_session::FightModes::DEFAULT,
            in_fight: false,
        }),
        Some(7),
        &[target],
    );
    outbox.inventory_initialize(
        Some(oteryn_session::CharacterInventory {
            entries: vec![oteryn_session::ItemEntry {
                handle: NonZeroU64::MIN,
                item_definition_ref: NonZeroU32::new(256).ok_or("item")?,
                count: NonZeroU32::MIN,
                sub_type: 0,
            }],
            ..Default::default()
        }),
        true,
    );
    assert!(link.bind_action_key(0, "SUPER+ALT+F11"));
    let key = NormalizedInputEvent::Key {
        code: KeyCode::new(68)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Super).with(Modifier::Alt),
        repeat: false,
    };
    for (mode, expected) in [(ItemUseMode::SelfActor, own), (ItemUseMode::Target, target)] {
        assert!(link.assign_bar_action(
            0,
            BarAction::Use {
                definition: NonZeroU32::new(256).ok_or("item")?,
                mode
            }
        ));
        assert!(link.custom_hotkeys_input(std::slice::from_ref(&key)));
        let crate::combat_input::PlayCommand::WorldContext(command) = commands.try_recv()? else {
            return Err("use command".into());
        };
        assert_eq!(command.action, Action::UseOn(NonZeroU64::MIN));
        assert_eq!(command.target, Target::Creature(expected.entity));
        let payload =
            oteryn_client_runtime::item_use::use_with_payload(NonZeroU64::MIN, &command.target)?;
        assert!(!payload.is_empty());
        crate::play::mailbox::lock(&link.mailbox)
            .world_context
            .pending = false;
    }
    assert!(link.assign_bar_action(
        0,
        BarAction::Use {
            definition: NonZeroU32::new(256).ok_or("item")?,
            mode: ItemUseMode::Cursor
        }
    ));
    assert!(link.custom_hotkeys_input(std::slice::from_ref(&key)));
    assert!(commands.try_recv().is_err());
    assert_eq!(
        crate::play::mailbox::lock(&link.mailbox)
            .world_context
            .targeting,
        Some(NonZeroU64::MIN)
    );
    outbox.inventory_end();
    assert!(
        crate::play::mailbox::lock(&link.mailbox)
            .world_context
            .targeting
            .is_none()
    );
    Ok(())
}
