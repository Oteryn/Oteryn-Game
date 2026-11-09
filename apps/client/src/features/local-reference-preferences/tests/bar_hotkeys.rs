use super::*;

#[test]
fn action_bar_hotkeys_cover_three_rows_via_real_cast_dispatch()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{PreferenceStore, SettingsState};
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let path = std::env::temp_dir().join(format!("cgopt-all-bar-keys-{}.json", std::process::id()));
    let mut settings = SettingsState::new(Some(path.clone()));
    let mut preferences = settings.preferences.clone();
    for slot in 0..36 {
        let modifier = ["SUPER", "CTRL+SUPER", "ALT+SUPER"][slot / 12];
        preferences.rebind(
            &format!("local-spell-action-bar-spell-{}", slot + 1),
            &format!("{modifier}+F{}", slot % 12 + 1),
        )?;
    }
    settings.apply(preferences);
    assert_eq!(
        PreferenceStore::load(path.clone())?.preferences(),
        &settings.preferences
    );
    settings = SettingsState::new(Some(path.clone()));
    let bindings = crate::play::preference_bindings()?;
    assert_eq!(
        bindings
            .iter()
            .filter(|binding| binding.id.starts_with("local-spell-action-bar-spell-"))
            .count(),
        36
    );
    let (link, mut commands, outbox) = play_channel();
    let mut play = view()?;
    let catalog: Vec<_> = (71..107)
        .map(
            |cast_index| oteryn_session::spell_catalog::SpellCatalogEntry {
                cast_index,
                name: format!("Spell {cast_index}"),
                words: format!("cast {cast_index}"),
                level: 1,
                mana: 0,
                mana_percent: false,
                soul: 0,
                learned: true,
                available: true,
                cooldown_groups: vec![],
                rune_item: None,
            },
        )
        .collect();
    outbox.spell_catalog_initialize(
        true,
        Some(
            oteryn_session::spell_catalog::encode_catalog(&catalog)
                .map_err(|e| format!("{e:?}"))?,
        ),
    )?;
    let fields: Vec<String> = (71..107).map(|id| id.to_string()).collect();
    assert!(link.spell_bar_bind(&fields));
    // Drive the same slot-selection handler used by the native Bind button.
    for slot in 0..36 {
        crate::play::feature_local_spell_action_bar::input::edit_slot(
            &mut crate::play::mailbox::lock(&link.mailbox),
            &link.commands,
            0,
            &[(slot + 1).to_string(), (71 + slot).to_string()],
        );
    }
    let mut router = crate::spell::SpellHotkeys::new()?;
    for slot in 0u16..36 {
        let modifiers = match slot / 12 {
            0 => Modifiers::NONE.with(Modifier::Super),
            1 => Modifiers::NONE
                .with(Modifier::Super)
                .with(Modifier::Control),
            _ => Modifiers::NONE.with(Modifier::Super).with(Modifier::Alt),
        };
        let code = KeyCode::new(58 + slot % 12)?;
        let gesture =
            [ButtonState::Pressed, ButtonState::Released].map(|state| NormalizedInputEvent::Key {
                code,
                modifiers,
                state,
                repeat: false,
            });
        let (events, _) = settings.input(&gesture);
        for action in settings.take_custom_actions() {
            assert!(link.custom_hotkey_action(&play, &action));
        }
        for slot in router.route(&events) {
            play.cast(slot);
        }
        play.tick(&link).map_err(|_| "tick")?;
        assert!(
            matches!(commands.try_recv()?, crate::combat_input::PlayCommand::Combat(crate::combat_input::CombatCommand::Cast(id)) if id.get() == 71 + u32::from(slot))
        );
        assert!(commands.try_recv().is_err());
    }
    let before = settings.preferences.clone();
    assert!(
        settings
            .preferences
            .rebind("local-spell-action-bar-spell-36", "CTRL+SUPER+F1")
            .is_err()
    );
    assert_eq!(settings.preferences, before);
    assert!(
        settings
            .preferences
            .rebind("local-spell-action-bar-spell-37", "SUPER+F12")
            .is_err()
    );
    assert_eq!(settings.preferences, before);
    settings
        .preferences
        .rebind("local-spell-action-bar-spell-36", "")?;
    assert!(
        !settings
            .preferences
            .custom_hotkeys
            .values()
            .any(|action| serde_json::to_value(action).is_ok_and(|value| value["slot"] == 36))
    );
    crate::play::mailbox::lock(&link.mailbox).death.dead = true;
    assert!(!link.custom_hotkey_action(&play, &crate::play::CustomAction::ActionBar { slot: 36 }));
    assert!(commands.try_recv().is_err());
    assert!(!settings.preferences.action_keys.0.contains_key(&35));
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn rebound_spell_releases_after_modifier_and_preference_changes()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{PreferenceInput, ReferencePreferences};
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let mut preferences = ReferencePreferences::default();
    preferences.rebind("local-spell-action-bar-spell-1", "ALT+H")?;
    let code = KeyCode::new(11)?;
    let key = |state, modifiers| NormalizedInputEvent::Key {
        code,
        state,
        modifiers,
        repeat: false,
    };
    let mut input = PreferenceInput::default();
    let mut spells = crate::spell::SpellHotkeys::new()?;
    let press = input.route(
        &preferences,
        &[key(ButtonState::Pressed, Modifiers::one(Modifier::Alt))],
    )?;
    assert_eq!(spells.route(&press).len(), 1);
    preferences.rebind("local-spell-action-bar-spell-1", "ALT+J")?;
    let release = input.route(&preferences, &[key(ButtonState::Released, Modifiers::NONE)])?;
    spells.route(&release);
    let press = input.route(
        &preferences,
        &[NormalizedInputEvent::Key {
            code: KeyCode::new(13)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::one(Modifier::Alt),
            repeat: false,
        }],
    )?;
    assert_eq!(spells.route(&press).len(), 1);
    Ok(())
}
