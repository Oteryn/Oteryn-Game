use super::*;

#[test]
fn sound_subchannel_checkboxes_edit_independent_preferences() -> TestResult {
    for (page, ids) in [
        (Page::Sound, &["sound.anthem"][..]),
        (
            Page::BattleSounds,
            &[
                "battle_sound.creature_death",
                "battle_sound.creature_noises",
                "battle_sound.other_spells",
                "battle_sound.other_weapons",
                "battle_sound.own_spells",
                "battle_sound.own_weapons",
            ][..],
        ),
        (
            Page::UiSounds,
            &[
                "ui_sound.eating",
                "ui_sound.global",
                "ui_sound.guild",
                "ui_sound.move_item",
                "ui_sound.npc",
                "ui_sound.party_messages",
                "ui_sound.private",
                "ui_sound.private_without_tab",
                "ui_sound.raid",
                "ui_sound.system",
            ][..],
        ),
    ] {
        let mut app = app(page, None)?;
        for id in ids {
            let before = app
                .world()
                .non_send::<HostOwner>()
                .preferences_draft()
                .ok_or("draft")?
                .audio;
            let checkbox = app
                .world_mut()
                .spawn(Control::Check(id))
                .observe(toggle)
                .id();
            app.world_mut().trigger(ValueChange {
                source: checkbox,
                value: false,
                is_final: true,
            });
            let after = app
                .world()
                .non_send::<HostOwner>()
                .preferences_draft()
                .ok_or("draft")?
                .audio;
            assert_ne!(before, after, "{id}");
        }
    }
    Ok(())
}

#[test]
fn disabled_sound_groups_preserve_their_child_choices() -> TestResult {
    for (page, parent, child) in [
        (
            Page::BattleSounds,
            "battle_sound.own_spells",
            "battle_sound.own_attacks",
        ),
        (
            Page::BattleSounds,
            "battle_sound.other_spells",
            "battle_sound.other_support",
        ),
        (Page::UiSounds, "ui_sound.chat", "ui_sound.private"),
    ] {
        let mut app = app(page, None)?;
        let parent_control = app
            .world_mut()
            .spawn(Control::Check(parent))
            .observe(toggle)
            .id();
        let child_control = app
            .world_mut()
            .spawn(Control::Check(child))
            .observe(toggle)
            .id();
        let value = |app: &App| -> TestResult {
            let draft = app
                .world()
                .non_send::<HostOwner>()
                .preferences_draft()
                .ok_or("draft")?;
            assert_eq!(
                metadata::find(page, child).ok_or("child")?.value(draft),
                Value::Check(false)
            );
            Ok(())
        };
        app.world_mut().trigger(ValueChange {
            source: child_control,
            value: false,
            is_final: true,
        });
        app.world_mut().trigger(ValueChange {
            source: parent_control,
            value: false,
            is_final: true,
        });
        app.world_mut().trigger(ValueChange {
            source: child_control,
            value: true,
            is_final: true,
        });
        value(&app)?;
        app.world_mut().trigger(ValueChange {
            source: parent_control,
            value: true,
            is_final: true,
        });
        value(&app)?;
    }
    Ok(())
}
