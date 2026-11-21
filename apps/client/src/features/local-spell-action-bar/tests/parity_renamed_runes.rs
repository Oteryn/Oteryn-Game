use super::*;

#[test]
fn renamed_rune_target_options_survive_settings_reload_and_multiaction()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{BarAction, BarGesture, SpellAim};
    use oteryn_session::spell_catalog::*;
    use std::num::NonZeroU32;
    for (item, old, current) in [
        (3153, "antidote rune", "Cure Poison Rune"),
        (3197, "desintegrate rune", "Disintegrate Rune"),
        (3149, "energybomb rune", "Energy Bomb Rune"),
        (3192, "firebomb rune", "Fire Bomb Rune"),
        (3144, "energy field rune", "Energy Field Rune"),
    ] {
        for aim in [SpellAim::Target, SpellAim::Facing] {
            let entry = |index, name: &str| SpellCatalogEntry {
                cast_index: index,
                name: name.into(),
                words: "".into(),
                level: 1,
                mana: 0,
                mana_percent: false,
                soul: 0,
                learned: true,
                available: true,
                cooldown_groups: vec![],
                rune_item: Some(item),
            };
            let (old_link, _, old_outbox) = play_channel();
            old_outbox.spell_catalog_initialize(
                true,
                Some(encode_catalog(&[entry(3, old)]).map_err(|e| format!("{e:?}"))?),
            )?;
            assert!(old_link.spell_bar_bind(&["3".into(), "".into(), "".into(), "".into()]));
            assert!(old_link.bar_gesture(BarGesture::Aim { slot: 0, aim }));
            let path = std::env::temp_dir().join(format!(
                "rune-target-reload-{}-{item}-{aim:?}.json",
                std::process::id()
            ));
            let mut settings = SettingsState::new(Some(path.clone()));
            settings.sync_bar_actions(&old_link);
            let mut reloaded = SettingsState::new(Some(path.clone()));
            assert_eq!(
                reloaded.preferences.bar_actions,
                settings.preferences.bar_actions
            );
            std::fs::remove_file(path)?;
            let (link, mut commands, outbox) = play_channel();
            outbox.spell_catalog_initialize(
                true,
                Some(encode_catalog(&[entry(71, current)]).map_err(|e| format!("{e:?}"))?),
            )?;
            reloaded.sync_bar_actions(&link);
            let mut view = view()?;
            let expected = match aim {
                SpellAim::Facing => crate::combat_input::CombatCommand::CastFacing {
                    spell: NonZeroU32::new(71).ok_or("index")?,
                    direction: None,
                },
                _ => crate::combat_input::CombatCommand::Cast(NonZeroU32::new(71).ok_or("index")?),
            };
            view.cast(NonZeroU32::MIN);
            view.tick(&link).map_err(|_| "tick")?;
            assert_eq!(
                commands.try_recv()?,
                crate::combat_input::PlayCommand::Combat(expected),
                "{old}, {aim:?}"
            );
            let action = reloaded
                .preferences
                .bar_actions
                .0
                .get(&0)
                .ok_or("saved target action")?
                .clone();
            assert!(link.assign_bar_action(
                0,
                BarAction::Multi {
                    actions: vec![action]
                }
            ));
            assert!(
                link.spell_slot_labels()[0].contains("Next 1/1:"),
                "{old} readiness"
            );
            view.cast(NonZeroU32::MIN);
            view.tick(&link).map_err(|_| "tick")?;
            assert_eq!(
                commands.try_recv()?,
                crate::combat_input::PlayCommand::Combat(expected)
            );
            assert!(commands.try_recv().is_err());
        }
    }
    Ok(())
}

#[test]
fn saved_spell_target_options_refuse_unproven_or_ambiguous_identity()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{BarAction, SpellAim};
    use oteryn_session::spell_catalog::*;
    use std::num::NonZeroU32;
    let entry = |index, name: &str, words: &str, rune_item| SpellCatalogEntry {
        cast_index: index,
        name: name.into(),
        words: words.into(),
        level: 1,
        mana: 0,
        mana_percent: false,
        soul: 0,
        learned: true,
        available: true,
        cooldown_groups: vec![],
        rune_item,
    };
    for (saved_words, entries) in [
        ("", vec![entry(71, "Unknown Rune", "", Some(3153))]),
        (
            "changed words",
            vec![entry(71, "Cure Poison Rune", "", Some(3153))],
        ),
        ("", vec![entry(71, "Cure Poison Rune", "", Some(3197))]),
        (
            "",
            vec![entry(71, "Cure Poison Rune", "instant words", None)],
        ),
        (
            "",
            vec![
                entry(71, "antidote rune", "", Some(3153)),
                entry(72, "Cure Poison Rune", "", Some(3153)),
            ],
        ),
        (
            "",
            vec![
                entry(71, "antidote rune", "", Some(3153)),
                entry(72, "antidote rune", "", Some(3153)),
            ],
        ),
    ] {
        let (link, mut commands, outbox) = play_channel();
        outbox.spell_catalog_initialize(
            true,
            Some(encode_catalog(&entries).map_err(|e| format!("{e:?}"))?),
        )?;
        let action = BarAction::Spell {
            name: "antidote rune".into(),
            words: saved_words.into(),
            aim: SpellAim::Target,
        };
        assert!(link.assign_bar_action(0, action.clone()));
        let mut view = view()?;
        view.cast(NonZeroU32::MIN);
        view.tick(&link).map_err(|_| "tick")?;
        assert!(commands.try_recv().is_err(), "must refuse {entries:?}");
        assert!(link.assign_bar_action(
            0,
            BarAction::Multi {
                actions: vec![action]
            }
        ));
        assert!(!link.spell_slot_labels()[0].contains("Next 1/1:"));
        view.cast(NonZeroU32::MIN);
        view.tick(&link).map_err(|_| "tick")?;
        assert!(commands.try_recv().is_err());
    }
    let (link, mut commands, outbox) = play_channel();
    outbox.spell_catalog_initialize(
        true,
        Some(encode_catalog(&[entry(71, "Instant", "words", None)]).map_err(|e| format!("{e:?}"))?),
    )?;
    assert!(link.assign_bar_action(
        0,
        BarAction::Spell {
            name: "instant".into(),
            words: "words".into(),
            aim: SpellAim::Target
        }
    ));
    let mut view = view()?;
    view.cast(NonZeroU32::MIN);
    view.tick(&link).map_err(|_| "tick")?;
    assert!(commands.try_recv().is_err(), "instant names remain exact");
    Ok(())
}
